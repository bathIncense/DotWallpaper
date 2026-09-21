// 显示器枚举与稳定标识。
// 使用 vendor/model/serial 组合成跨启动稳定的显示器 ID（CGDisplayID 是临时的），
// 运行时再映射回当前 CGDirectDisplayID。全部 CoreGraphics 调用线程安全。

use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayBounds, CGDisplayIsAsleep, CGDisplayIsBuiltin, CGDisplayIsMain,
    CGDisplayPixelsWide, CGDisplaySerialNumber, CGDisplayVendorNumber, CGGetOnlineDisplayList,
};

use crate::types::DisplayInfo;

pub fn online_displays() -> Vec<CGDirectDisplayID> {
    let mut ids = vec![0u32; 16];
    let mut count: u32 = 0;
    let err =
        unsafe { CGGetOnlineDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) };
    if err.0 != 0 {
        return Vec::new();
    }
    ids.truncate(count as usize);
    ids
}

/// 稳定显示器 ID：内置屏固定为 builtin；外接屏用厂商+型号+序列号；无序列号退回 cgdisplay 临时 ID。
pub fn stable_id(display: CGDirectDisplayID) -> String {
    if unsafe { CGDisplayIsBuiltin(display) } {
        return "builtin".to_string();
    }
    let serial = unsafe { CGDisplaySerialNumber(display) };
    if serial != 0 {
        let vendor = unsafe { CGDisplayVendorNumber(display) };
        let model = vendor.wrapping_mul(31) ^ display; // 同型号多屏时以当前连接 ID 区分通道
        return format!("disp-{vendor:x}-{model:x}-{serial:x}");
    }
    format!("cgdisplay-{display}")
}

/// 稳定 ID -> 当前在线 CGDisplayID
pub fn cg_id_for_stable(stable: &str) -> Option<CGDirectDisplayID> {
    online_displays()
        .into_iter()
        .find(|id| stable_id(*id) == stable)
}

pub fn logical_bounds(display: CGDirectDisplayID) -> (f64, f64, f64, f64) {
    let b = unsafe { CGDisplayBounds(display) };
    (
        b.origin.x,
        b.origin.y,
        b.size.width,
        b.size.height,
    )
}

pub fn is_asleep(display: CGDirectDisplayID) -> bool {
    unsafe { CGDisplayIsAsleep(display) }
}

pub fn is_main(display: CGDirectDisplayID) -> bool {
    unsafe { CGDisplayIsMain(display) }
}

pub fn enumerate() -> Vec<DisplayInfo> {
    let displays = online_displays();
    let total = displays.len();
    displays
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let bounds = logical_bounds(*id);
            let pixel_w = unsafe { CGDisplayPixelsWide(*id) } as f64;
            let scale = if bounds.2 > 0.0 { pixel_w / bounds.2 } else { 1.0 };
            DisplayInfo {
                id: stable_id(*id),
                name: if is_main(*id) {
                    "主显示器".to_string()
                } else {
                    format!("显示器 {}（共 {total} 块）", i + 1)
                },
                logical_bounds: bounds,
                scale_factor: scale,
                primary: is_main(*id),
                mirrored: false,
            }
        })
        .collect()
}
