# DotWallpaper macOS 构建指南

## 环境要求

- macOS 12（Monterey）或更高版本
- Apple Silicon Mac（M1/M2/M3/M4）
- Xcode Command Line Tools（无需完整 Xcode）
- Node.js 18+
- Rust 稳定工具链

## 安装步骤

### 1. 安装 Xcode Command Line Tools

```bash
xcode-select --install
```

### 2. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

添加 Apple Silicon 目标：

```bash
rustup target add aarch64-apple-darwin
```

### 3. 安装 Node.js 依赖

```bash
# 前端依赖
cd ui
npm install

# 根目录依赖（Tauri CLI）
cd ..
npm install
```

## 开发模式

```bash
npm run tauri dev
```

## 构建 macOS 应用

```bash
# 构建 .app 和 .dmg
npm run build

# 仅构建 .app
npm run build:mac:app
```

## 构建产物

- `.app`：`src-tauri/target/aarch64-apple-darwin/release/bundle/macos/DotWallpaper.app`
- `.dmg`：`src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/DotWallpaper_0.1.2_aarch64.dmg`

## 项目结构

```
DotWallpaper/
├── ui/                              # 前端项目
│   ├── src/
│   │   ├── App.vue                  # 根组件
│   │   ├── main.ts                  # 入口
│   │   ├── styles/                  # 样式
│   │   ├── lib/                     # 工具库
│   │   ├── stores/                  # Pinia 状态
│   │   └── components/              # 组件
│   ├── index.html
│   └── package.json
├── src-tauri/                       # Rust 后端
│   ├── src/
│   │   ├── main.rs                  # Tauri 入口
│   │   ├── wallpaper.rs             # 壁纸管理
│   │   ├── thumbs.rs                # 缩略图管线
│   │   └── platform/
│   │       ├── mod.rs               # 平台 trait
│   │       └── macos/
│   │           ├── mod.rs           # macOS 入口
│   │           ├── desktop.rs       # NSWorkspace + 播放
│   │           ├── displays.rs      # CGDisplay
│   │           ├── playback.rs      # AVFoundation
│   │           └── heic.rs          # HEIC 检测
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── tauri.macos.conf.json        # macOS 打包配置
│   ├── capabilities/default.json
│   └── icons/
│       └── icon.icns                # macOS 图标
├── docs/
│   ├── build-guide.md               # 本文件
│   └── p0-verification-checklist.md # P0 验证清单
├── .github/workflows/workflow.yml   # CI 配置
├── README.md
└── package.json
```

## 常见问题

### 构建失败

```bash
# 清理并重新构建
cargo clean
cargo build --target aarch64-apple-darwin
```

### 签名与公证

发布版本需要 Apple Developer 账号进行签名和公证。开发构建无需签名即可在本地运行。

### 动态壁纸

视频/GIF 动态壁纸使用 AVFoundation 实现桌面播放层，需要 macOS 12+ 支持。
