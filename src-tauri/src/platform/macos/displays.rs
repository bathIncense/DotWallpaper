// macOS 显示器枚举
// 使用 CoreGraphics 获取显示器信息

use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayBounds, CGDisplayIsMain, CGDisplayPixelsHigh, CGDisplayPixelsWide,
    CGGetOnlineDisplayList,
};

use crate::platform::DisplayInfo;

/// 枚举所有在线显示器
pub fn list_displays() -> Result<Vec<DisplayInfo>, String> {
    let mut displays = Vec::new();

    let max_displays: u32 = 16;
    let mut online_displays: Vec<CGDirectDisplayID> = vec![0; max_displays as usize];
    let mut display_count: u32 = 0;

    let result = unsafe {
        CGGetOnlineDisplayList(
            max_displays,
            online_displays.as_mut_ptr(),
            &mut display_count,
        )
    };

    if result != objc2_core_graphics::CGError(0) {
        return Err(format!("获取显示器列表失败，错误码: {:?}", result));
    }

    online_displays.truncate(display_count as usize);

    for (index, &display_id) in online_displays.iter().enumerate() {
        if let Some(info) = get_display_info(display_id, index) {
            displays.push(info);
        }
    }

    Ok(displays)
}

/// 获取单个显示器信息
fn get_display_info(display_id: CGDirectDisplayID, index: usize) -> Option<DisplayInfo> {
    let bounds = CGDisplayBounds(display_id);
    let is_main = CGDisplayIsMain(display_id);

    // 使用简洁的 display ID 格式，便于在 apply_wallpaper 中匹配
    let id = format!("cgdisplay-{}", display_id);

    let name = if is_main {
        "主显示器".to_string()
    } else {
        format!("显示器 {}", index + 1)
    };

    let pixel_w = CGDisplayPixelsWide(display_id) as f64;
    let pixel_h = CGDisplayPixelsHigh(display_id) as f64;
    let logical_w = bounds.size.width;
    let logical_h = bounds.size.height;

    let scale_factor = if logical_w > 0.0 && logical_h > 0.0 {
        (pixel_w / logical_w + pixel_h / logical_h) / 2.0
    } else {
        1.0
    };

    Some(DisplayInfo {
        id,
        name,
        logical_bounds: (
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
        ),
        scale_factor,
        primary: is_main,
        mirrored: false,
    })
}
