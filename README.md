# DotWallpaper

轻量 macOS 壁纸管理工具：本地壁纸库统一管理静态图片与桌面视频动态壁纸，多显示器独立控制，菜单栏常驻。当前使用 macOS 26+ 原生 Xcode 构建：SwiftUI/WKWebView 宿主 + Rust static library + Vue 3 前端。

## 项目目的

- 把"图片 / 视频桌面壁纸"做成一个低资源占用的单二进制 macOS 应用：不用 Electron、不引入重型 UI 框架
- Rust 端直接调用 AppKit / AVFoundation / CoreGraphics / ImageIO（objc2 + 少量 C 声明），替代 `image` crate 与各类插件，压缩体积与依赖面
- 前端只保留三段式极简界面（顶栏 / 媒体网格 / 预览面板），状态用 Vue 组合式单例管理

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面宿主 | 原生 SwiftUI / AppKit / WKWebView（Xcode 27） |
| 系统层 | Rust：objc2-app-kit / av-foundation / core-graphics / core-media，ImageIO 直调 |
| 前端 | Vue 3 + TypeScript |
| 样式 | Tailwind CSS v4（无 Naive UI / Pinia / 图标库） |
| 构建 | Xcode 27 + Cargo + Vite，仅 arm64，最低 macOS 26.0 |

## 项目结构

```
DotWallpaper/
├── scripts/
│   ├── build_xcode_mac.sh           # Xcode 构建并校验 arm64/macOS 26
│   └── release_mac.sh               # 构建 → strip → 签名 → hdiutil 出 DMG
├── ui/                              # 前端（Vue 3，无第三方 UI 依赖）
│   └── src/
│       ├── App.vue                  # 布局、⌘R/⌘S 快捷键、窗口拖放导入
│       ├── composables/useApp.ts    # 全局状态单例（媒体列表/显示器/播放状态/设置/toast）
│       ├── lib/api.ts               # WKWebView 原生 bridge 唯一出口
│       ├── types/index.ts           # 与后端 types.rs 对齐的 wire 类型
│       └── components/              # TopBar / MediaGrid / PreviewPanel / Toaster / SvgIcon
├── xcode/
│   ├── WallpaperEngine.xcodeproj    # macOS 26+ 原生 Xcode 工程
│   └── WallpaperEngineApp/          # SwiftUI/WKWebView 宿主与 FFI 调用层
├── src-tauri/                       # Rust 核心库（staticlib，目录名为兼容历史代码）
│   └── src/
│       ├── lib.rs                   # C FFI 核心库入口
│       ├── ffi.rs                   # Swift 调用的 C ABI
│       ├── types.rs                 # DTO（camelCase 序列化，前后端契约）
│       ├── runtime.rs               # 主线程调度原语：on_main<T>() / spawn_main()
│       ├── engine.rs                # 核心：PlaybackSession（NSWindow+AVQueuePlayer+Looper+Layer 共同持有，
│       │                            #   仅主线程访问）、per-display generation 令牌防竞态、事务式切换
│       │                            #   （失败保留旧壁纸）、首帧就绪后才替换+落海报、Preparing 可停止、
│       │                            #   启动恢复、2s 轮询热插拔与休眠唤醒、状态事件 publish
│       ├── desktop.rs               # NSWorkspace 静态壁纸（系统读回校验）、NSScreen 匹配、NSOpenPanel
│       ├── displays.rs              # CGDisplay 枚举 + 稳定显示器 ID（builtin / vendor+serial，不掺 CGDisplayID）
│       ├── media.rs                 # 库扫描/分类/导入复制/删除 + 路径越界防护 + 使用中删除保护
│       ├── settings.rs              # 版本化 JSON 持久化（app_config_dir/settings.json，临时文件+rename 原子落盘，损坏自动 .bak）
│       ├── thumbs.rs                # 缩略图缓存管线（ImageIO 缩放，后台生成 + 事件推送，海报投递排期）
│       ├── cfmedia.rs               # ImageIO/CoreFoundation 裸 C 声明（JPEG 编码/取尺寸——尺寸走属性字典，不解码位图）
│       └── unit_tests.rs            # 16 个单元测试（分类/边界/持久化/ID 迁移/序列化契约/IPC 键名交叉校验/缩略图缓存键/扫描顺序与 mtime/应用支持目录/图片尺寸读取/符号链接不入扫描）
├── package.json                     # dev / build:mac / build:mac:app / release:mac
└── README.md
```

## 已实现功能

### 壁纸应用
- **静态图片**：JPG / JPEG / PNG / BMP / WebP / HEIC，经 `NSWorkspace.setDesktopImageURL` 设置，填充模式可选（填充 / 适应），设置后用 `desktopImageURLForScreen` 读回校验，失败即报错不假成功
- **视频动态壁纸**：MP4 / MOV，桌面层级播放窗口（壁纸图标层之下、不抢焦点、跟随所有空间），`AVQueuePlayer + AVPlayerLooper` 无缝循环，可选静音（候补准备期强制静音，窗口上位后才按偏好出声）
- **事务式视频切换**：应用视频先建"候补会话"（phase = `preparing`），旧壁纸全程不动；首帧 `ReadyToPlay` 后才替换窗口、并把首帧海报落为系统静态壁纸；抽取失败/解码失败/15s 超时 → 候补销毁、旧视频与原壁纸保持不变并上报错误
- **重连恢复不叠发**：监视线程自己按住「正在恢复」的显示器 ID，该次 apply 收尾才放行。`display_inflight` 只看 SESSIONS/STAGED，而从 tick 里 spawn 出恢复任务、到 apply 真的把候补塞进 STAGED 之间有空窗（主线程被海报任务占住时可拉到数秒），期间下一个 2 秒 tick 会对同一块屏再发起一次 apply，两份候补互相覆盖、被顶掉的那份留下没人持有的播放窗口
- **generation 防竞态**：每次应用/停止为该显示器递增令牌，被取代的准备中会话自我销毁——快速连切 A→B→C 只会留下 C，无叠窗、无幽灵播放；watcher 只取走与自己代数相同的候补，过期 watcher 不会误杀新一代正在准备的会话
- **播放控制**：每显示器 暂停 / 恢复 / 停止（停止回到海报静态壁纸，并清除该屏持久化分配，重启后不再自动恢复），准备中也可停止（watcher 作废）；托盘可全局批量操作
- **静态切换同样事务**：先 `setDesktopImageURL` 并从系统读回确认成功，之后才销毁该屏旧视频会话；设置失败旧壁纸原样保留
- **设置记录时机**：静态在成功后写入；视频仅在真正进入播放后才写入持久化分配，准备中失败/退出不会留下坏配置
- **同步退出清理**：由原生 `applicationWillTerminate` 调用 Rust FFI 的同步 teardown，退出前销毁所有播放会话与候补，避免残留桌面窗口
- **多显示器**：稳定 ID（内建屏 `builtin` / 外接屏 vendor+serial，不掺易变的 CGDisplayID），每屏独立赋值；无序列号的显示器返回临时 ID 并在界面上标注「临时」提醒重新指定；拔插与睡眠唤醒后自动重连恢复（2s 轮询，准备中的会话计入在途不被重复套用）。恢复失败的「该屏 + 该文件」组合会被锁定，不再每 2 秒空转重试、反复上报；一旦该组合重新套用成功（含换内容后修好的同名文件）锁自动解除
- **配置版本迁移**：旧版（v1）显示器 ID 形如 `disp-<vendor>-<model>-<serial>`，其中 model 段内嵌了易变的 CGDisplayID，拔插后必然失配；载入时自动改写为现行 `disp-<vendor>-<serial>` 并以 v2 落盘，升级后既有的逐屏壁纸配置不会变成"永远恢复不了的幽灵项"
- **删除保护**：三种情况一律拒绝删除——① 正被会话使用（含准备中的候补）；② 仍被某显示器记录为壁纸配置；③ 状态镜像里某块屏当前展示的内容就是它（视频「停止」后持久化配置已清、会话已销毁，但桌面留的是它的首帧海报，删源文件会连海报缓存一起清掉，壁纸就指向不存在的文件）。前两种是"配置在用"，第三种是"画面还在用"。"是否在用"的查询**失败即视为在用**（联系不上主线程或读不到镜像时都不放行），宁可删不掉也不能放走正在播放的文件
- **实时状态同步**：所有壁纸状态变化经 `wallpaper-state` 事件推给前端（Preparing→Playing、热插拔、休眠恢复），UI 无需手动刷新即与桌面一致

### 库管理
- 自选壁纸目录（原生 NSOpenPanel），递归扫描，目录与偏好持久化
- 拖放导入、原生文件选择器导入（复制到库内、重名自动编号、不支持格式跳过并提示）
- 列表内删除（仅允许删库内文件，删图同时清缩略图/海报缓存，含内容变化后遗留的旧缓存名）
- 256px 缩略图后台生成 + `thumbnail-ready` 事件逐张回填，首屏窗口内优先保证；缓存名 = 路径哈希 + 内容指纹（mtime/大小），同名替换文件后旧缓存自动失效
- **取尺寸不再整幅解码**：「小图直接用原图当缩略图」的判定原先用 `CGImageSourceCreateImageAtIndex` 问宽高——每张稍大的图都要先整幅解码一次、再生成一次缩略图，两次解码全压在首屏预算里；现在只读 ImageIO 属性字典的 `PixelWidth`/`PixelHeight`（不解位图）。判定用的是 `max(w,h)`，与属性里不含 EXIF 旋转这件事无关，语义不变。该读取路径由 `image_size_reads_properties_and_thumbnail_round_trip` 用仓库自带的 1024×1024 图标钉住（键名或 `CFNumber` 类型判断写错会静默返回 `None`，正反向断言才拦得住）
- **扫描只认库内真实文件**：目录项用 `file_type()`（lstat 语义，不跟随符号链接）判定，软链既不入列也不递归。跟随的旧写法有两个后果：库内一个指向上层/家目录的软链会被当成子目录无限递归（自我引用即栈只进不出），库外文件则会被列进界面——而应用与删除侧都按 canonical 后的授权边界拒绝它们，于是只剩「看得见点不动」的死条目（`scan_ignores_symlinks_instead_of_recursing_or_listing_them` 用自我引用软链 + 库外目标钉住两点）
- **海报投递有排期**：视频首帧只能在主线程抽取（AVFoundation），而主线程就是窗口的事件线程，所以 worker 每 120ms 才放行一个海报任务、落后的 worker 睡在自己线程上。不排期时 6 个 worker 会把主线程的 runloop 排成一片连续任务，扫描视频多的库期间窗口点不动；图片路径不受影响（ImageIO 在 worker 线程）
- 媒体网格四态区分：加载骨架屏（>1.2s 才提示）/ 读取失败（错误信息 + 重试）/ 空目录引导 / 正常网格
- 库内轻量检索：按文件名搜索 + 图片/类型筛选 + 排序（名称 A→Z / Z→A、最近/最早修改），全部由前端 `computed` 完成，不额外走 IPC；`mtime` 取不到（记 0）的文件一律在时间序里按名称排在末尾。网格恒定按 60 项分批渲染、滚动到底再追加，搜索/筛选/排序变更时重置到首批

### 应用形态
- 菜单栏常驻：打开 / 暂停全部 / 恢复全部 / 停止全部 / 退出；关窗 = 隐藏，应用继续播
- **单实例护栏**：启动时以 `~/Library/Application Support/<identifier>/single-instance.lock` 上的 advisory 锁（flock）认领"唯一实例"，第二个进程打印一行提示后直接退出。没有这道护栏时，重复启动会让两个进程各开一套壁纸窗口互相覆盖桌面，各自的状态镜像与删除保护也各执一词（一边"解除占用"了，另一边仍拒删）。锁随 fd 关闭自动释放，进程被杀不留残需要清理的锁文件；护栏自身出错时**放行**（宁可双开也不能让用户打不开应用）。注意 `File::open` 侧的 std 约束：`create(true)` 必须同时具备写权限，只读打开会直接返回 `InvalidInput`
- **控制按钮与后端能力一致**：`preparing` 期间只允许"停止"（后端此时拒绝暂停/恢复，按钮亮着等报错等于骗用户点），并在按钮下方写明"暂停 / 恢复要等首帧就绪"；显示器拔出后的 `error` 状态在右侧列表保留为一行"已断开显示器"，那里的"停止"显示为**解除占用**——清除持久化分配、同时放开该文件的删除保护，否则断开的那块屏会永久锁住它的壁纸文件
- 显示器列表会自我修正：状态事件里出现未知 ID 或 `error` 阶段时自动重取快照（拔屏后列表还留着旧屏、重连后临时 ID 变了，都要靠 ⌘R 才能刷新的话，界面就会拿着不存在的屏幕做应用）
- 快捷键：`⌘R` 刷新，`⌘S` 应用当前选中
- 深色极简三段式 UI，操作反馈 toast

### 工程
- 原生 bridge 暴露 10 个 action：`getAppSnapshot`、`listMedia`、`listDisplays`、`applyWallpaper`、`controlPlayback`、`pickLibraryDirectory`、`pickMediaFiles`、`importMedia`、`deleteMedia`、`updateSettings`
- Rust 通过 `dotwallpaper.h` 输出 C ABI；Swift 负责 WKWebView 消息分发，Vue 通过 `window.DotWallpaperNative` 调用
- `cargo test` 16/16；前端执行 `vue-tsc --noEmit && vite build`，并在构建后内联 JS/CSS，使 `file://` WKWebView 不依赖外部资源
- 同一条命令还踩过第二个坑：`update_settings` 每次都把前端的 `libraryDir` 回写进设置，而窗口刚开、快照未读回（或读取失败）时它是空串——先点一下静音就把空目录落盘，下次扫描直接报「无法读取壁纸目录」。库目录已从该载荷移除（只能经 `pick_library_directory` 修改，它自己落盘并授权 asset scope），`FrontendSettings` 加 `deny_unknown_fields` 让两边的字段漂移显式失败而不是被 serde 静默丢掉；`settings_wire_contract_both_directions` 同时钉住"带目录的旧载荷必须解析失败"

## 未实现 / 已知边界

- **GIF 动图**：不作为动态壁纸（导入直接跳过）
- **动态 HEIC（日/夜/天文）**：不检测、不应用
- **开机自启、定时轮换、播放列表/分组、快捷键自定义**：未做
- **镜像显示**：只读不控。`DisplayInfo.mirrored` 现取自 `CGDisplayIsInMirrorSet`（系统语义：镜像集里只有**副屏**为 true，镜像源那块仍是 false），界面据此打「镜像」徽章；但应用仍按枚举到的每块屏各开一套壁纸窗口，也没做「同画面去重」，镜像下的实际显示由系统说了算
- **网络/系统壁纸源**：无（仅本地库）
- **自动更新**：updater 插件与相关 UI 已整体移除
- **Intel / Universal 二进制**：仅产出 aarch64；旧 Mac 需自行改 target
- **Developer ID 签名与公证**：发布脚本已内置该分支（`DW_SIGN_IDENTITY` / `DW_NOTARY_PROFILE`），但本机 keychain 里没有 Developer ID 证书，**这条路径未做过正向实测**（只负向验证过身份缺失时脚本会失败退出、不会静默降级）。默认产物仍是 ad-hoc 签名，本机自装不受影响，见下方"签名与公证"说明
- **缓存回收粒度**：缩略图/海报按「路径哈希 + 内容指纹」命名，同名替换文件时旧变体只在源文件删除时按前缀统一清理，长期高频替换会让缓存目录缓慢增长
- 真机验收项（多显示器热插拔实测、30 分钟内存占用、Mission Control 切换表现）尚未在目标机器上系统跑过

## 开发

```bash
npm install && cd ui && npm install && cd ..   # 依赖
npm run dev                                     # 启动 Vue/Vite 开发服务器
cd src-tauri && cargo test                      # 后端单元测试
```

## 打包发布

环境：macOS 26+ / Apple Silicon、Node.js、稳定 Rust 工具链、完整 Xcode 27。

```bash
npm run release:mac
```

一条命令完成：Xcode Release 构建（同时构建 Vue 与 Rust static library）→ 对主二进制 `strip -x` → `codesign` 签名 → `codesign --verify --strict` 自检 → `diskutil image create from` 生成压缩 DMG →（可选）公证并装订 → 打印体积报告。产物：

```
build/xcode-derived/Build/Products/Release/WallpaperEngine.app
build/WallpaperEngine_0.1.2_arm64.dmg
```

体积组成（当前实测）：主二进制 strip 后 7.59 MB（未 strip 9.31 MB）· 前端 dist 156 KB（JS 125,144 B + CSS 26,691 B + index.html）· `.app` 总计 7.4 MB · DMG ≈1.91 MB。较 14 MB 基线减少约 47%。

两个刻意的设计（勿改回）：

- `profile.release` 中 **`strip = false`**：新版 macOS 的 strip 会产出 LINKEDIT 未对齐的 dylib，直接打挂编译期 proc-macro（rust-lang/rust#157750），因此改为发布脚本对最终二进制单独 strip
- **DMG 使用 macOS 26 原生 `diskutil image create from`**：不依赖 Finder 自动化，适合本机与 CI 的无交互构建环境

只构建并校验 `.app`（不出 DMG）：`npm run build:mac:app`。

### 签名与公证（分发时需要）

`scripts/release_mac.sh` 内置了签名分支，默认为 ad-hoc（本机自用）。有证书时用环境变量切到 Developer ID + 公证，仍然一条命令出盘：

```bash
export DW_SIGN_IDENTITY="Developer ID Application: 你的证书名 (TEAMID)"
export DW_NOTARY_PROFILE=<xcrun notarytool store-credentials 存的 profile 名>
npm run release:mac
```

脚本里的顺序是刻意的，手工签名时也不要反过来：

- 先 `strip -x` 再签名（顺序反了签名即失效）；二进制与 `.app` 分别内→外签，不用已废弃的 `--deep`
- 带 `--options runtime --timestamp`（Hardened Runtime + 安全时间戳），公证要求这两项
- 签名 → **再**打 DMG → 公证 **DMG** → `stapler` 同时装订 DMG 和 `.app`。DMG 必须晚于签名生成，否则盘里装的是旧签名；对已出厂的 DMG 补签名等于没签
- 身份缺失/找不到即 `set -e` 失败退出，不会静默退化成 ad-hoc（本机 2026-09-22 用不存在的身份实测：`no identity found`，脚本 exit 1 且不产出 DMG）；只设 `DW_SIGN_IDENTITY` 不设 profile 时照签但打印「已签名但未公证」告警，因为这种包他人下载仍会被拦

仅本机使用（ad-hoc 签名）：从别处拷贝来的 DMG 若被 Gatekeeper 拦截，可执行 `xattr -dr com.apple.quarantine /Applications/DotWallpaper.app` 放行；自行 `release:mac` 构建的本地产物不带隔离属性，可直接运行。本机实测：`codesign --verify --strict` 通过（`Signature=adhoc`、TeamIdentifier 未设置），但 `spctl -a -t execute` 判定 **rejected**——ad-hoc 包只要沾上隔离属性就会被 Gatekeeper 拦，对外分发必须补 Developer ID 签名 + 公证。证书到位后 `release:mac` 末尾会自动跑一次 `spctl --assess` 复核。
