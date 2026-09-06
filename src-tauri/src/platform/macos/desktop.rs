// macOS 桌面壁纸实现
// - 静态图片：NSWorkspace.setDesktopImageURL
// - 视频/GIF：桌面播放层（NSWindow + AVPlayerLayer）

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use objc2::rc::Retained;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSScreen, NSWindow, NSWindowCollectionBehavior,
    NSWindowStyleMask, NSWindowTitleVisibility, NSWorkspace,
};
use objc2_core_graphics::CGWindowLevelForKey;
use objc2_foundation::{MainThreadMarker, NSString, NSURL};

use crate::platform::{
    DisplayWallpaperState, FitMode, WallpaperAssignment,
};

/// 桌面管理器状态（只存储可 Send 的数据）
struct DesktopState {
    display_states: HashMap<String, DisplayWallpaperState>,
    dynamic_displays: HashMap<String, bool>,
}

impl DesktopState {
    fn new() -> Self {
        Self {
            display_states: HashMap::new(),
            dynamic_displays: HashMap::new(),
        }
    }
}

/// 全局桌面管理器
static DESKTOP_STATE: once_cell::sync::Lazy<Arc<Mutex<DesktopState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(DesktopState::new())));

/// 应用壁纸到指定显示器
pub fn apply_wallpaper(
    assignment: &WallpaperAssignment,
    request_id: u64,
) -> Result<DisplayWallpaperState, String> {
    let mut state = DESKTOP_STATE.lock().map_err(|e| e.to_string())?;
    let display_id = assignment.display_id.clone();

    // 提取媒体路径和类型
    let (media_type, path) = if let Some(p) = assignment.media_id.strip_prefix("image/") {
        ("image", p)
    } else if let Some(p) = assignment.media_id.strip_prefix("video/") {
        ("video", p)
    } else if let Some(p) = assignment.media_id.strip_prefix("gif/") {
        ("gif", p)
    } else if let Some(p) = assignment.media_id.strip_prefix("dynamic_heic/") {
        ("dynamic_heic", p)
    } else {
        return Err(format!("无效的媒体 ID: {}", assignment.media_id));
    };

    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err(format!("媒体文件不存在: {}", path));
    }

    if media_type == "video" || media_type == "gif" {
        // 动态壁纸：使用桌面播放层
        apply_dynamic_wallpaper(&display_id, path, assignment, &mut state)?;
    } else {
        // 静态壁纸：使用 NSWorkspace
        apply_static_wallpaper(path, &assignment.fit_mode)?;
    }

    // 更新状态
    let display_state = DisplayWallpaperState {
        display_id: display_id.clone(),
        request_id,
        revision: state
            .display_states
            .get(&display_id)
            .map(|s| s.revision + 1)
            .unwrap_or(1),
        phase: "active".to_string(),
        desired_assignment: Some(assignment.clone()),
        actual_assignment: Some(assignment.clone()),
        system_wallpaper: Some(path.to_string()),
        error: None,
    };

    state
        .display_states
        .insert(display_id.clone(), display_state.clone());
    state.dynamic_displays.insert(display_id, media_type != "image");

    Ok(display_state)
}

/// 应用静态图片壁纸（NSWorkspace）
fn apply_static_wallpaper(path: &str, _fit_mode: &FitMode) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or_else(|| "无法获取主线程标记".to_string())?;

    // 创建 NSURL
    let url_string = format!(
        "file://{}",
        Path::new(path)
            .canonicalize()
            .map_err(|e| e.to_string())?
            .display()
    );

    let ns_url = create_nsurl(&url_string)?;

    // 获取主屏幕
    let screens = NSScreen::screens(mtm);
    let main_screen = screens.objectAtIndex(0);

    // 获取 NSWorkspace
    let workspace = NSWorkspace::sharedWorkspace();

    // TODO: 构建选项字典并调用 NSWorkspace.setDesktopImageURL
    // 实际实现需要根据 objc2 API 调整
    let _ = (ns_url, main_screen, workspace);

    Ok(())
}

/// 创建 NSURL
fn create_nsurl(url_string: &str) -> Result<Retained<NSURL>, String> {
    let ns_string = NSString::from_str(url_string);
    // 使用 objc2 的 msg_send 宏调用 fileURLWithString:
    let url: Retained<NSURL> = unsafe {
        objc2::msg_send![objc2::class!(NSURL), fileURLWithString: &*ns_string]
    };
    Ok(url)
}

/// 应用动态壁纸（桌面播放层）
fn apply_dynamic_wallpaper(
    display_id: &str,
    path: &str,
    assignment: &WallpaperAssignment,
    _state: &mut DesktopState,
) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or_else(|| "无法获取主线程标记".to_string())?;

    // 获取主屏幕
    let screens = NSScreen::screens(mtm);
    let screen = screens.objectAtIndex(0);

    // 创建新的播放窗口
    let window = create_playback_window(&screen)?;

    // 设置播放器
    crate::platform::macos::playback::setup_player(&window, path, display_id, assignment.muted)?;

    // 注意：窗口由 Rust 拥有，但不存储在全局状态中
    let _ = display_id;

    Ok(())
}

/// 获取指定显示器的壁纸状态
pub fn get_wallpaper_state(display_id: &str) -> Result<DisplayWallpaperState, String> {
    let state = DESKTOP_STATE.lock().map_err(|e| e.to_string())?;

    // 如果显示器状态不存在，返回默认状态而非报错
    Ok(state
        .display_states
        .get(display_id)
        .cloned()
        .unwrap_or_else(|| DisplayWallpaperState {
            display_id: display_id.to_string(),
            request_id: 0,
            revision: 0,
            phase: "idle".to_string(),
            desired_assignment: None,
            actual_assignment: None,
            system_wallpaper: None,
            error: None,
        }))
}

/// 创建桌面播放窗口
fn create_playback_window(screen: &NSScreen) -> Result<Retained<NSWindow>, String> {
    let _mtm = MainThreadMarker::new().ok_or_else(|| "无法获取主线程标记".to_string())?;

    let frame = screen.frame();

    // 创建窗口
    let window: Retained<NSWindow> = unsafe {
        let cls = objc2::class!(NSWindow);
        let allocated: *mut NSWindow = objc2::msg_send![cls, alloc];
        let window: *mut NSWindow = objc2::msg_send![allocated, initWithContentRect: frame
                                         styleMask: NSWindowStyleMask::Borderless
                                           backing: NSBackingStoreType::Buffered
                                             defer: false];
        Retained::from_raw(window).expect("窗口创建失败")
    };

    // 设置窗口层级（桌面壁纸层，在桌面图标之下）
    let desktop_icon_level = unsafe {
        CGWindowLevelForKey(objc2_core_graphics::CGWindowLevelKey::DesktopIconWindowLevelKey)
    };
    window.setLevel((desktop_icon_level - 1) as isize);

    // 设置窗口行为
    let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::Stationary
        | NSWindowCollectionBehavior::IgnoresCycle;
    window.setCollectionBehavior(behavior);

    // 设置窗口属性
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setIgnoresMouseEvents(true);
    window.setAcceptsMouseMovedEvents(false);
    window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
    window.setHidesOnDeactivate(false);

    // 显示窗口
    window.makeKeyAndOrderFront(None);

    Ok(window)
}
