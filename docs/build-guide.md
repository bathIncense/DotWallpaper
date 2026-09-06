# DotWallpaper macOS 构建指南

## 环境要求

- macOS 12（Monterey）或更高版本
- Apple Silicon Mac（M1/M2/M3/M4）
- Xcode Command Line Tools（无需完整 Xcode）
- Node.js 18+
- Rust 稳定工具链

## 快速开始

### 1. 安装依赖

```bash
# 前端依赖
cd ui
npm install

# Rust 依赖（首次构建时自动下载）
cd ../src-tauri
cargo fetch
```

### 2. 开发模式

```bash
# 在项目根目录
npm run dev
```

### 3. 构建 macOS 应用

```bash
# 构建 .app 和 .dmg
npm run build:mac

# 仅构建 .app
npm run build:mac:app
```

### 4. 构建产物

- `.app`：`src-tauri/target/aarch64-apple-darwin/release/bundle/macos/DotWallpaper.app`
- `.dmg`：`src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/DotWallpaper_0.1.2_aarch64.dmg`

## 项目结构

```
DotWallpaper/
├── ui/                          # 前端项目
│   ├── src/
│   │   ├── types/media.ts       # 媒体类型定义
│   │   ├── lib/platform.ts      # 平台工具函数
│   │   ├── stores/wallpaper.ts  # 壁纸状态管理
│   │   └── ...
│   └── package.json
├── src-tauri/                   # Rust 后端
│   ├── src/
│   │   ├── platform/
│   │   │   ├── mod.rs           # 平台 trait + DTO
│   │   │   ├── windows.rs       # Windows 实现
│   │   │   └── macos/
│   │   │       ├── mod.rs       # macOS 平台入口
│   │   │       ├── desktop.rs   # NSWorkspace + 播放窗口
│   │   │       ├── displays.rs  # CGDisplay 枚举
│   │   │       ├── playback.rs  # AVPlayer 状态管理
│   │   │       └── heic.rs      # HEIC 检测
│   │   ├── main.rs              # Tauri 入口
│   │   └── ...
│   ├── tauri.macos.conf.json    # macOS 打包配置
│   ├── icons/icon.icns          # macOS 图标
│   └── Cargo.toml
├── docs/
│   └── p0-verification-checklist.md
└── package.json
```

## 核心命令

| 命令 | 描述 |
|------|------|
| `get_platform_capabilities` | 获取平台能力 |
| `list_displays` | 枚举所有显示器 |
| `apply_wallpaper` | 应用壁纸到指定显示器 |
| `get_wallpaper_state` | 获取显示器壁纸状态 |
| `pause_wallpaper` | 暂停动态壁纸 |
| `resume_wallpaper` | 恢复动态壁纸 |
| `stop_wallpaper` | 停止动态壁纸 |

## 故障排除

### 编译失败
```bash
# 清理并重新构建
cargo clean
cargo build --target aarch64-apple-darwin
```

### 前端构建失败
```bash
cd ui
rm -rf node_modules
npm install
npm run build
```

### 图标缺失
```bash
# 重新生成 icon.icns
cd src-tauri/icons
sips -s format png --resampleWidth 512 icon.png --out icon_512.png
iconutil -c icns iconset -o icon.icns
```

## 后续步骤

1. 完成 P0 实机验证
2. 根据验证结果调整实现
3. 添加 Developer ID 签名
4. 配置自动更新
5. 发布到 GitHub Releases
