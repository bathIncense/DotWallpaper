// DotWallpaper 平台抽象层
// 为 macOS 提供统一的壁纸能力接口

pub mod macos;

use serde::Serialize;
use serde::Deserialize;
use objc2_foundation::MainThreadMarker;

/// 平台能力探测结果
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlatformCapabilities {
    /// 平台标识："macos"
    pub platform: String,
    /// 支持的媒体类型
    pub media_kinds: Vec<String>,
    /// 支持的填充模式
    pub supported_fit_modes: Vec<String>,
    /// 是否支持多显示器
    pub multi_display: bool,
    /// 是否原生支持动态 HEIC
    pub native_dynamic_heic: bool,
}

/// 显示器信息
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    /// 显示器稳定标识（CoreGraphics display ID）
    pub id: String,
    /// 显示器名称
    pub name: String,
    /// 逻辑边界（x, y, width, height）
    pub logical_bounds: (f64, f64, f64, f64),
    /// 缩放比
    pub scale_factor: f64,
    /// 是否主显示器
    pub primary: bool,
    /// 是否镜像其他显示器
    pub mirrored: bool,
}

/// 壁纸填充模式
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FitMode {
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
}

impl FitMode {
    /// 获取所有填充模式
    pub fn all() -> Vec<FitMode> {
        vec![
            FitMode::Fill,
            FitMode::Fit,
            FitMode::Stretch,
            FitMode::Center,
            FitMode::Tile,
        ]
    }

    /// 获取名称
    pub fn name(&self) -> &'static str {
        match self {
            FitMode::Fill => "fill",
            FitMode::Fit => "fit",
            FitMode::Stretch => "stretch",
            FitMode::Center => "center",
            FitMode::Tile => "tile",
        }
    }
}

/// 媒体类型
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Video,
    Gif,
    DynamicHeic,
}

impl MediaKind {
    /// 获取名称
    pub fn name(&self) -> &'static str {
        match self {
            MediaKind::Image => "image",
            MediaKind::Video => "video",
            MediaKind::Gif => "gif",
            MediaKind::DynamicHeic => "dynamic_heic",
        }
    }
}

/// 壁纸分配请求
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperAssignment {
    /// 目标显示器 ID
    pub display_id: String,
    /// 媒体 ID
    pub media_id: String,
    /// 填充模式
    pub fit_mode: FitMode,
    /// 是否静音（视频/GIF）
    pub muted: bool,
}

/// 显示器壁纸状态
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DisplayWallpaperState {
    /// 显示器 ID
    pub display_id: String,
    /// 请求 ID（用于匹配请求与响应）
    pub request_id: u64,
    /// 状态版本号
    pub revision: u64,
    /// 当前阶段
    pub phase: String,
    /// 期望的壁纸分配
    pub desired_assignment: Option<WallpaperAssignment>,
    /// 实际的壁纸分配
    pub actual_assignment: Option<WallpaperAssignment>,
    /// 系统当前壁纸路径
    pub system_wallpaper: Option<String>,
    /// 错误信息
    pub error: Option<String>,
}

/// 平台 trait：定义各平台必须实现的壁纸能力
pub trait Platform: Send + Sync {
    /// 获取平台能力
    fn capabilities(&self) -> PlatformCapabilities;

    /// 枚举所有显示器
    fn list_displays(&self) -> Result<Vec<DisplayInfo>, String>;

    /// 应用壁纸到指定显示器
    fn apply_wallpaper(
        &self,
        assignment: &WallpaperAssignment,
        request_id: u64,
        mtm: &MainThreadMarker,
    ) -> Result<DisplayWallpaperState, String>;

    /// 获取指定显示器的壁纸状态
    fn get_wallpaper_state(
        &self,
        display_id: &str,
    ) -> Result<DisplayWallpaperState, String>;

    /// 暂停指定显示器的动态壁纸
    fn pause_wallpaper(&self, display_id: &str) -> Result<(), String>;

    /// 恢复指定显示器的动态壁纸
    fn resume_wallpaper(&self, display_id: &str) -> Result<(), String>;

    /// 停止指定显示器的动态壁纸
    fn stop_wallpaper(&self, display_id: &str) -> Result<(), String>;
}

/// 获取当前平台的实现
pub fn current() -> Box<dyn Platform> {
    Box::new(macos::MacPlatform::new())
}
