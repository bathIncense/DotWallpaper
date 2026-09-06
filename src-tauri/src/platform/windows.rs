// Windows 平台实现
// 使用 Win32 SystemParametersInfoW 设置桌面壁纸
// 注意：此模块仅在 Windows 平台编译

use crate::platform::{
    DisplayInfo, DisplayWallpaperState, Platform, PlatformCapabilities, WallpaperAssignment,
};

/// Windows 平台实现
pub struct WinPlatform {
    capabilities: PlatformCapabilities,
}

impl WinPlatform {
    pub fn new() -> Self {
        let capabilities = PlatformCapabilities {
            platform: "windows".to_string(),
            media_kinds: vec!["image".to_string()],
            supported_fit_modes: vec![
                "fill".to_string(),
                "fit".to_string(),
                "stretch".to_string(),
                "center".to_string(),
                "tile".to_string(),
            ],
            multi_display: false,    // Windows 版本暂不支持多显示器
            native_dynamic_heic: false,
            updater_enabled: true,   // Windows 启用自动更新
        };

        Self { capabilities }
    }
}

impl Platform for WinPlatform {
    fn capabilities(&self) -> PlatformCapabilities {
        self.capabilities.clone()
    }

    fn list_displays(&self) -> Result<Vec<DisplayInfo>, String> {
        // Windows 版本返回单一主显示器
        // TODO: 获取实际分辨率
        Ok(vec![DisplayInfo {
            id: "primary".to_string(),
            name: "主显示器".to_string(),
            logical_bounds: (0.0, 0.0, 1920.0, 1080.0),
            scale_factor: 1.0,
            primary: true,
            mirrored: false,
        }])
    }

    fn apply_wallpaper(
        &self,
        assignment: &WallpaperAssignment,
        request_id: u64,
    ) -> Result<DisplayWallpaperState, String> {
        let path = assignment
            .media_id
            .strip_prefix("image/")
            .ok_or_else(|| "无效的图片媒体 ID".to_string())?;

        set_wallpaper_win32(path)?;

        Ok(DisplayWallpaperState {
            display_id: "primary".to_string(),
            request_id,
            revision: 1,
            phase: "active".to_string(),
            desired_assignment: Some(assignment.clone()),
            actual_assignment: Some(assignment.clone()),
            system_wallpaper: Some(path.to_string()),
            error: None,
        })
    }

    fn get_wallpaper_state(&self, display_id: &str) -> Result<DisplayWallpaperState, String> {
        Ok(DisplayWallpaperState {
            display_id: display_id.to_string(),
            request_id: 0,
            revision: 1,
            phase: "active".to_string(),
            desired_assignment: None,
            actual_assignment: None,
            system_wallpaper: Some(get_current_wallpaper_win32()?),
            error: None,
        })
    }

    fn pause_wallpaper(&self, _display_id: &str) -> Result<(), String> {
        // Windows 静态壁纸无需暂停
        Ok(())
    }

    fn resume_wallpaper(&self, _display_id: &str) -> Result<(), String> {
        // Windows 静态壁纸无需恢复
        Ok(())
    }

    fn stop_wallpaper(&self, _display_id: &str) -> Result<(), String> {
        // Windows 静态壁纸无需停止
        Ok(())
    }
}

/// 通过 Win32 SystemParametersInfoW 设置桌面壁纸
fn set_wallpaper_win32(path: &str) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_SETDESKWALLPAPER, SPIF_UPDATEINIFILE, SPIF_SENDCHANGE,
    };

    let mut path_utf16: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Some(path_utf16.as_mut_ptr() as *mut std::ffi::c_void),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
        .map_err(|e| format!("设置壁纸失败 (Win32 错误: {e})"))?;
    }

    Ok(())
}

/// 获取当前桌面壁纸路径
fn get_current_wallpaper_win32() -> Result<String, String> {
    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETDESKWALLPAPER};

    let mut buffer = [0u16; 2048];

    unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buffer.len() as u32,
            Some(buffer.as_mut_ptr() as *mut std::ffi::c_void),
            Default::default(),
        )
        .map_err(|e| format!("获取当前壁纸失败 (Win32 错误: {e})"))?;
    }

    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let path = String::from_utf16_lossy(&buffer[..len]);

    if path.is_empty() {
        return Err("未获取到当前壁纸路径".into());
    }

    Ok(path)
}
