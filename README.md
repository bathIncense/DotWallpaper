# DotWallpaper

简单、轻量的 Windows 壁纸切换工具，让你快速浏览并设置喜欢的桌面壁纸。

基于 Tauri 2 + Rust + Vue 3 构建的 Windows 桌面壁纸管理工具，提供本地壁纸管理、系统壁纸浏览、壁纸收藏、一键设壁、壁纸样式设置、拖入导入与自定义目录能力，支持深色主题；应用内更新走 GitHub Releases，并提供微软商店 MSIX 打包支持。

## 核心功能

- **壁纸应用**：点击壁纸卡片预览，一键设为桌面壁纸
- **当前壁纸**：实时读取并展示当前桌面壁纸，支持"重新设为壁纸"与左右切换
- **壁纸样式**：支持设置桌面壁纸样式（填充 / 适应 / 拉伸 / 居中 / 平铺），实时读取当前系统样式
- **壁纸收藏**：右键菜单或卡片星标收藏 / 取消收藏，独立"收藏"选项卡集中展示，书签持久化到 localStorage（快捷键 Ctrl+F）
- **系统壁纸**：独立"系统"选项卡只读浏览 Windows 自带壁纸目录
- **右键管理**：右键菜单支持设为桌面壁纸、收藏 / 取消收藏、在资源管理器中显示、永久删除文件
- **拖入导入**：将本地图片拖入应用窗口，自动复制保存到壁纸目录（支持 JPG/JPEG/PNG/BMP/WebP），Ctrl+I 可唤出拖放遮罩
- **目录切换**：自由选择壁纸存放目录，选择记忆持久化到 localStorage
- **分页加载**：壁纸列表按页加载并自动补页，适配大量壁纸目录；缩略图由 Rust 端生成缓存，前端懒加载
- **应用内更新**：启动后自动检查 GitHub Releases 新版本，弹窗展示更新说明与下载进度（微软商店版自动剥离自更新入口）
- **深色主题**：全局深色毛玻璃设计，视觉贴合桌面环境
- **快捷键**：Ctrl+S 重新设置当前壁纸、Ctrl+R 刷新壁纸列表、Ctrl+F 收藏当前预览壁纸、Ctrl+I 显示拖放遮罩

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2 |
| 系统层 | Rust（Win32 API 壁纸设置、注册表样式读写、文件系统、缩略图生成） |
| 前端框架 | Vue 3 + TypeScript |
| UI 组件 | Naive UI |
| 样式方案 | Tailwind CSS v4 |
| 图标库 | Lucide |
| 状态管理 | Pinia |
| 更新机制 | tauri-plugin-updater（GitHub Releases） |

## 项目结构

```
DotWallpaper/
├── .github/
│   └── workflows/workflow.yml      # 推送 v* 标签时自动构建 GitHub Releases
├── scripts/
│   └── build-msix.ps1              # 微软商店 MSIX 打包脚本（商店模式构建 + winapp 打包）
├── ui/                             # 前端项目（Vue 3）
│   ├── src/
│   │   ├── App.vue                 # 根组件（布局、快捷键、启动更新检查）
│   │   ├── main.ts                 # 入口（Pinia）
│   │   ├── styles/main.css         # Tailwind 主题配置
│   │   ├── lib/naive-host.ts       # Naive UI 宿主绑定
│   │   ├── stores/
│   │   │   ├── wallpaper.ts        # Pinia 壁纸状态（目录记忆 / 收藏 / 来源排序）
│   │   │   └── updater.ts          # Pinia 更新状态（检查 / 下载 / 弹窗）
│   │   ├── utils/
│   │   │   └── updater.ts          # 更新 API 封装 + isStoreBuild 商店构建判断
│   │   └── components/
│   │       ├── TitleBar.vue        # 自定义标题栏（含更新角标与设置入口）
│   │       ├── Sidebar.vue         # 左栏壁纸网格（本地 / 系统 / 收藏选项卡）
│   │       ├── CurrentPanel.vue    # 右栏当前壁纸展示与样式设置
│   │       ├── ContextMenu.vue     # 右键菜单
│   │       ├── DropZone.vue        # 拖放接收（Tauri 原生 drag-drop）
│   │       ├── NaiveBridge.vue     # Naive UI 全局上下文桥接
│   │       ├── UpdateDialog.vue    # 更新弹窗（检查 / 发布说明 / 下载进度）
│   │       └── SettingsCenter/     # 设置中心
│   │           ├── SettingsCenter.vue       # 设置中心主入口（导航 + 子面板）
│   │           ├── SettingHeader.vue        # 设置中心头部框架
│   │           ├── DirectoryPanel.vue       # 壁纸目录
│   │           ├── SourceVisibilityPanel.vue # 壁纸来源（本地/系统/收藏显隐与排序）
│   │           ├── ShortcutsPanel.vue       # 快捷键说明
│   │           ├── SupportPanel.vue         # 交流打赏
│   │           └── AboutPanel.vue           # 关于（版本、GitHub 项目主页、更新检查）
│   ├── index.html
│   └── package.json
├── src-tauri/                      # Rust 后端
│   ├── src/
│   │   ├── main.rs                 # Tauri 入口、命令注册
│   │   ├── wallpaper.rs            # 壁纸设置/获取、系统壁纸扫描、桌面样式读写
│   │   └── thumbs.rs               # 缩略图缓存与后台渐进生成
│   ├── Assets/                     # MSIX 商店资源（AppList/StoreLogo/WideTile/MedTile 多缩放）
│   ├── icons/                      # 应用图标
│   ├── capabilities/default.json   # 主窗口能力（窗口 / dialog / updater / opener）
│   ├── Package.appxmanifest        # MSIX 应用清单
│   ├── tauri.conf.json             # 主配置（含 updater / asset scope）
│   ├── tauri.microsoftstore.conf.json # 商店构建配置（createUpdaterArtifacts:false）
│   ├── build.rs
│   ├── Cargo.toml
│   └── Cargo.lock
├── package.json                    # 根工程（tauri 命令 / MSIX 打包脚本）
└── README.md
```

## 环境要求

- **操作系统**：Windows 10 / 11
- **Node.js**：18+（前端构建；CI 使用 20）
- **Rust**：稳定工具链（需 MSVC 构建工具链）
- **PowerShell 7+（pwsh）**：仅 MSIX 打包需要（`scripts/build-msix.ps1` 依赖）
- **winapp CLI**：仅 MSIX 打包需要（Windows 应用打包工具，含证书生成）

## 快速开始

### 安装依赖

```bash
# 1. 前端依赖
cd ui
npm install

# 2. Rust 依赖
cd ../src-tauri
cargo fetch
```

### 开发模式

```bash
npm run tauri dev
```

### 构建发布版（GitHub Releases）

```bash
$env:TAURI_SIGNING_PRIVATE_KEY="Path or content of your private key"

npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。推送 `v*` 标签到 GitHub 后，CI（`.github/workflows/workflow.yml`）会自动构建并发布 Release；应用内更新通过 tauri-plugin-updater 读取该 Release 的 `latest.json`。

### 构建微软商店 MSIX

```bash
npm run pack:msix
# 等价于：npm run build-msix → pwsh -File scripts/build-msix.ps1 -Sign
```

脚本流程（详见 `scripts/build-msix.ps1`）：

1. 以 `VITE_STORE_BUILD=1` 执行商店模式构建，前端通过 `isStoreBuild` 剥离全部自更新入口（跳过启动检查更新、隐藏更新按钮）
2. 合并 `tauri.microsoftstore.conf.json`（`createUpdaterArtifacts: false`），不生成更新产物
3. 组装 `DotWallpaper.exe + Package.appxmanifest + Assets` 包布局
4. 调用 winapp 打包为 MSIX；`-Sign` 时自动生成 / 校验本地自签证书

产物位于 `src-tauri/target/msix/xiaohai.DotWallpaper_<版本>_x64.msix`。首次安装需信任自签证书（管理员执行一次 `winapp cert install src-tauri/devcert.pfx`）；提交商店后由商店自行重签。

## 核心命令

Rust 后端暴露的 Tauri 命令（见 `src-tauri/src/main.rs`）：

| 命令 | 描述 |
|------|------|
| `set_wallpaper` | 将本地壁纸路径设为桌面壁纸 |
| `get_current_wallpaper` | 获取当前桌面壁纸路径 |
| `list_local_wallpapers` | 列出本地壁纸文件（含缩略图路径） |
| `list_system_wallpapers` | 列出 Windows 自带系统壁纸（只读） |
| `list_wallpapers_by_paths` | 按传入路径列表返回壁纸条目（收藏夹等场景，不扫描目录） |
| `pick_wallpaper_directory` | 弹出目录选择框 |
| `reveal_in_explorer` | 在资源管理器中显示指定文件 |
| `save_dropped_paths` | 保存拖入的本地图片到壁纸目录 |
| `delete_wallpaper` | 永久删除壁纸文件 |
| `get_wallpaper_style` | 获取桌面壁纸样式（填充/适应/拉伸/居中/平铺） |
| `set_desktop_style` | 设置桌面壁纸样式（含平铺开关） |
| `get_desktop_screen` | 获取主屏幕分辨率与缩放比 |

## 使用方式

1. **查看当前壁纸**：启动后右侧展示当前桌面壁纸，可切换壁纸样式
2. **浏览壁纸**：左侧网格按来源选项卡展示（本地 / 系统 / 收藏），滚动加载更多
3. **设置壁纸**：点击壁纸卡片预览，点击"设为壁纸"应用到桌面
4. **收藏壁纸**：点击卡片右上角星标或右键 → 收藏，可在"收藏"选项卡集中浏览（Ctrl+F 收藏当前预览壁纸）
5. **拖入壁纸**：将本地图片拖入窗口（或 Ctrl+I），自动保存并刷新列表
6. **删除壁纸**：右键壁纸 → "删除壁纸"，确认后从磁盘永久删除（系统壁纸来源只读，不提供删除）
7. **切换目录**：设置中心 → 壁纸目录，自由指定壁纸存放位置
8. **检查更新**：非商店版启动后自动检查更新；也可通过标题栏更新角标或"关于"面板手动检查

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+S` | 将当前预览壁纸重新设为桌面壁纸 |
| `Ctrl+R` | 刷新壁纸列表 |
| `Ctrl+F` | 收藏 / 取消收藏当前预览壁纸 |
| `Ctrl+I` | 显示拖放遮罩 |

## 后续规划

- 多壁纸分组与分类管理
- 每日定时自动切换
- macOS 版本适配

---

## 参考文档
使用 Tauri 的 winapp CLI  https://learn.microsoft.com/en-us/windows/apps/dev-tools/winapp-cli/guides/tauri#4-debug-with-identity
