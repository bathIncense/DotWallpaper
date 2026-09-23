# DotWallpaper

原生 macOS 桌面壁纸应用：管理本地静态图片与视频动态壁纸，为多显示器独立设置壁纸，并通过菜单栏控制播放。应用宿主使用 SwiftUI / AppKit / WKWebView，Rust static library 负责媒体、壁纸播放和系统集成，Vue 3 + TypeScript 提供管理界面。当前构建面向 Apple Silicon，最低 macOS 26.0。

## 功能概览

### 壁纸与播放

- 静态图片：JPG、JPEG、PNG、BMP、WebP、HEIC；支持填充或适应显示模式，设置后从系统读回验证。
- 视频动态壁纸：MP4、MOV；通过 `AVQueuePlayer` + `AVPlayerLooper` 循环播放，可设为静音。视频首帧就绪后才替换当前壁纸；准备失败或超时会保留原壁纸。
- 每台显示器独立应用、暂停、恢复或停止壁纸。停止视频后桌面保留首帧静态海报。
- 显示器热插拔和休眠唤醒后尝试恢复播放；支持稳定显示器 ID，并提示无序列号屏幕使用临时 ID。
- 菜单栏提供管理窗口、全部暂停、全部恢复、全部停止和退出操作；关闭管理窗口不会退出应用或停止播放。
- 切换壁纸采用事务式处理；快速连续切换由 generation 令牌防止旧请求覆盖新选择。播放状态通过原生 bridge 实时同步至界面。

### 媒体库与设置

- 首次启动引导用户选择媒体库目录，也可稍后设置；通过 macOS 原生目录选择器取得所选目录的访问权限。
- 递归扫描壁纸目录，支持拖放或原生文件选择器导入；导入文件复制到媒体库，重名时自动编号。
- 支持媒体搜索、类型筛选、排序、缩略图缓存及库内文件删除；被壁纸分配或当前显示使用的文件会受到删除保护。
- 设置面板可更换媒体库目录、配置默认图片显示方式和视频静音偏好，并说明设置文件与缓存位置。
- 应用仅访问用户选择的媒体目录，不要求屏幕录制或辅助功能权限。

### 尚未支持

GIF 动态壁纸、动态 HEIC、网络壁纸源、开机自启动、定时轮换、播放列表、按 Space 独立配置、锁屏壁纸接管、Intel/Universal 构建和应用内自动更新均未实现。静态图片叠加水滴、雪花、樱花等动态效果也尚未实现。镜像显示目前只展示系统报告的镜像状态，不提供镜像屏去重或独立控制保证。

## 技术栈

| 层级 | 技术 |
| --- | --- |
| macOS 宿主 | SwiftUI、AppKit、WKWebView（Xcode 27） |
| 系统与壁纸核心 | Rust static library、objc2、AppKit、AVFoundation、CoreGraphics、ImageIO |
| 管理界面 | Vue 3、TypeScript、Tailwind CSS v4 |
| 构建目标 | Apple Silicon arm64，最低 macOS 26.0 |

## 仓库结构

```text
scripts/                  macOS 原生构建与发布脚本
ui/                       Vue 管理界面及 WKWebView bridge
xcode/                    SwiftUI/AppKit/WKWebView Xcode 工程
rust-core/                Rust 壁纸与媒体核心（static library）
  dotwallpaper.h           Swift 调用的 C ABI
  src/                     FFI、播放引擎、媒体、设置与测试
.github/workflows/         原生 macOS 发布工作流
```

## 开发与验证

环境：macOS 26+、Xcode 27+、Node.js/npm、Rust stable。准备 Rust 交叉编译目标：

```bash
rustup target add aarch64-apple-darwin
npm install --prefix ui
```

启动前端开发服务器（仅用于 UI 开发）：

```bash
npm run dev
```

构建并运行原生应用时，WKWebView 使用打包后的前端资源：

```bash
npm run build:mac:app
open build/xcode-derived/Build/Products/Release/WallpaperEngine.app
```

运行离线检查：

```bash
npm run build --prefix ui
cargo test --manifest-path rust-core/Cargo.toml
cargo clippy --manifest-path rust-core/Cargo.toml --all-targets -- -D warnings
```

## 构建与发布

构建 `.app` 并生成 DMG：

```bash
npm run release:mac
```

发布脚本依次构建原生应用、对最终主程序执行 `strip -x`、签名与验证，再通过 macOS `diskutil image create from` 生成压缩 DMG。输出路径：

```text
build/xcode-derived/Build/Products/Release/WallpaperEngine.app
build/WallpaperEngine_<版本>_arm64.dmg
```

默认使用 ad-hoc 签名，仅适合本机或内部验证；这不等于可安全对外分发的 Developer ID 签名和公证。正式分发需在钥匙串中配置 Developer ID，并保存公证凭据：

```bash
export DW_SIGN_IDENTITY="Developer ID Application: 证书名称 (TEAMID)"
export DW_NOTARY_PROFILE="notarytool 凭据名称"
npm run release:mac
```

提供签名身份但不提供 `DW_NOTARY_PROFILE` 时，脚本会提示产物尚未公证。身份无效时会失败退出，不会静默退回 ad-hoc 签名。

## 当前验证边界

构建成功与单元测试通过不能替代真实 GUI 和壁纸播放验收。多显示器热插拔、休眠唤醒、长时间播放、首次启动引导和目录授权、菜单栏交互、Mission Control 表现，以及 Developer ID + 公证的正向发布链路，仍需在目标机器和对应签名环境中验证。
