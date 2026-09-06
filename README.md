# DotWallpaper

macOS 桌面壁纸管理工具。基于 Tauri 2 + Rust + Vue 3 构建，提供本地壁纸管理、一键设壁、拖入导入与自定义目录能力，支持深色主题。

## 核心功能

- **壁纸应用**：点击壁纸即可设为桌面壁纸
- **当前壁纸**：读取并展示当前桌面壁纸，提供"重新设为壁纸"按钮
- **右键管理**：右键菜单支持设为桌面壁纸、永久删除文件
- **拖入导入**：将本地图片/视频/GIF/HEIC 拖入应用窗口，自动复制保存到壁纸目录
- **目录切换**：自由选择壁纸存放目录，选择记忆持久化到 localStorage
- **分页加载**：壁纸列表按页加载，适配大量壁纸目录
- **深色主题**：全局深色毛玻璃设计，视觉贴合桌面环境
- **快捷键**：⌘+S 重新设置当前壁纸、⌘+R 刷新壁纸列表

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2 |
| 系统层 | Rust（AppKit/AVFoundation 壁纸设置、文件系统） |
| 前端框架 | Vue 3 + TypeScript |
| UI 组件 | Naive UI |
| 样式方案 | Tailwind CSS v4 |
| 图标库 | Lucide |
| 状态管理 | Pinia |

## 项目结构

```
DotWallpaper/
├── ui/                              # 前端项目
│   ├── src/
│   │   ├── App.vue                  # 根组件（布局、快捷键、拖放入口）
│   │   ├── main.ts                  # 入口（Pinia + 主题配置）
│   │   ├── styles/
│   │   │   └── main.css             # Tailwind 主题配置
│   │   ├── lib/
│   │   │   └── naive-host.ts        # Naive UI 宿主绑定
│   │   ├── stores/
│   │   │   └── wallpaper.ts         # Pinia 壁纸状态
│   │   └── components/
│   │       ├── TitleBar.vue         # 自定义标题栏
│   │       ├── Sidebar.vue          # 左侧壁纸网格
│   │       ├── CurrentPanel.vue     # 右侧当前壁纸展示
│   │       ├── ContextMenu.vue      # 右键菜单
│   │       ├── DropZone.vue         # 拖入区域
│   │       └── NaiveBridge.vue      # Naive UI 全局上下文桥接
│   ├── index.html
│   └── package.json
├── src-tauri/                       # Rust 后端
│   ├── src/
│   │   ├── main.rs                  # Tauri 入口、命令注册
│   │   ├── wallpaper.rs             # 壁纸管理核心逻辑
│   │   ├── thumbs.rs                # 缩略图生成管线
│   │   └── platform/
│   │       ├── mod.rs               # 平台 trait + DTO
│   │       └── macos/
│   │           ├── mod.rs           # macOS 平台入口
│   │           ├── desktop.rs       # NSWorkspace + 播放窗口
│   │           ├── displays.rs      # CGDisplay 枚举
│   │           ├── playback.rs      # AVFoundation 播放层
│   │           └── heic.rs          # 动态 HEIC 检测
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── tauri.macos.conf.json
│   ├── capabilities/default.json
│   └── icons/
├── README.md
└── package.json
```

## 环境要求

- **操作系统**：macOS 12（Monterey）或更高版本
- **硬件**：Apple Silicon Mac（M1/M2/M3/M4）
- **Node.js**：18+
- **Rust**：稳定工具链

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

### 构建发布版

```bash
npm run build
```

构建产物位于 `src-tauri/target/aarch64-apple-darwin/release/bundle/`。

## 核心命令

Rust 后端暴露的 Tauri 命令：

| 命令 | 描述 |
|------|------|
| `set_wallpaper` | 将本地壁纸路径设为桌面壁纸 |
| `get_current_wallpaper` | 获取当前桌面壁纸路径 |
| `list_local_wallpapers` | 列出本地壁纸文件 |
| `list_system_wallpapers` | 列出 macOS 系统壁纸（只读） |
| `pick_wallpaper_directory` | 弹出目录选择框 |
| `save_dropped_paths` | 保存拖入的本地图片到壁纸目录 |
| `delete_wallpaper` | 永久删除壁纸文件 |
| `get_desktop_screen` | 获取主屏幕分辨率与缩放比 |
| `get_platform_capabilities` | 获取平台能力 |
| `list_displays` | 枚举所有显示器 |
| `apply_wallpaper` | 应用壁纸到指定显示器 |

## 使用方式

1. **查看当前壁纸**：启动后右侧展示当前桌面壁纸
2. **浏览壁纸**：左侧网格展示本地壁纸列表，滚动加载更多
3. **设置壁纸**：点击壁纸卡片，右侧预览同步更新并设为桌面
4. **拖入壁纸**：将本地图片/视频/GIF/HEIC 拖入窗口，自动保存并刷新列表
5. **删除壁纸**：右键壁纸 → "删除壁纸"，确认后从磁盘永久删除（系统壁纸来源只读，不提供删除）
6. **切换目录**：点击顶部"切换目录"，自由指定壁纸存放位置

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `⌘+S` | 将当前壁纸重新设为桌面壁纸 |
| `⌘+R` | 刷新壁纸列表 |

## 支持格式

- **图片**：JPG、JPEG、PNG、BMP、WebP、HEIC/HEIF
- **视频**：MP4、MOV、M4V
- **动图**：GIF
- **动态 HEIC**：基于太阳位置/24小时/外观的动态壁纸

## 后续规划

- 多壁纸分组与分类管理
- 每日定时自动切换
- 多显示器独立壁纸设置

---
