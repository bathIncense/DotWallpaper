// macOS 平台实现
// 使用 NSWorkspace 设置静态壁纸，AVFoundation 实现桌面播放层

mod desktop;
mod displays;
mod playback;
mod heic;

use crate::platform::{
    DisplayInfo, DisplayWallpaperState, Platform, PlatformCapabilities, WallpaperAssignment,
};

/// macOS 平台实现
pub struct MacPlatform {
    capabilities: PlatformCapabilities,
}

impl MacPlatform {
    pub fn new() -> Self {
        let capabilities = PlatformCapabilities {
            platform: "macos".to_string(),
            media_kinds: vec![
                "image".to_string(),
                "video".to_string(),
                "gif".to_string(),
                "dynamic_heic".to_string(),
            ],
            supported_fit_modes: vec![
                "fill".to_string(),
                "fit".to_string(),
                "stretch".to_string(),
                "center".to_string(),
            ],
            multi_display: true,
            native_dynamic_heic: true, // 待 P0 验证确认
        };

        Self { capabilities }
    }
}

impl Platform for MacPlatform {
    fn capabilities(&self) -> PlatformCapabilities {
        self.capabilities.clone()
    }

    fn list_displays(&self) -> Result<Vec<DisplayInfo>, String> {
        displays::list_displays()
    }

    fn apply_wallpaper(
        &self,
        assignment: &WallpaperAssignment,
        request_id: u64,
        mtm: &objc2_foundation::MainThreadMarker,
    ) -> Result<DisplayWallpaperState, String> {
        desktop::apply_wallpaper(assignment, request_id, mtm)
    }

    fn get_wallpaper_state(&self, display_id: &str) -> Result<DisplayWallpaperState, String> {
        desktop::get_wallpaper_state(display_id)
    }

    fn pause_wallpaper(&self, display_id: &str) -> Result<(), String> {
        playback::pause(display_id)
    }

    fn resume_wallpaper(&self, display_id: &str) -> Result<(), String> {
        playback::resume(display_id)
    }

    fn stop_wallpaper(&self, display_id: &str) -> Result<(), String> {
        playback::stop(display_id)
    }
}

// 辅助类型导出
