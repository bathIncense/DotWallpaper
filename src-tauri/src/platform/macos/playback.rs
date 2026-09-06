// macOS 桌面播放层
// 使用 AVFoundation 实现视频/GIF 的桌面播放

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use objc2::rc::Retained;
use objc2_app_kit::NSWindow;
use objc2_av_foundation::{AVAsset, AVPlayerItem, AVPlayerLayer, AVQueuePlayer, AVPlayerLooper};
use objc2_foundation::{MainThreadMarker, NSArray, NSString, NSURL};

/// 播放层状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Preparing,
    Playing,
    Paused,
    Stopped,
    Error(String),
}

/// 播放器信息（只存储可 Send 的数据）
#[derive(Clone)]
pub struct PlayerInfo {
    pub state: PlaybackState,
    pub muted: bool,
    pub file_path: String,
}

impl PlayerInfo {
    pub fn new(file_path: String) -> Self {
        Self {
            state: PlaybackState::Idle,
            muted: true,
            file_path,
        }
    }
}

/// 全局播放器管理器
static PLAYBACK_STATE: once_cell::sync::Lazy<Arc<Mutex<HashMap<String, PlayerInfo>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// 设置播放器到指定窗口
pub fn setup_player(
    window: &NSWindow,
    file_path: &str,
    display_id: &str,
    muted: bool,
) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or_else(|| "无法获取主线程标记".to_string())?;

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    // 创建 NSURL
    let ns_string = NSString::from_str(file_path);
    let url: Retained<NSURL> = unsafe {
        objc2::msg_send![objc2::class!(NSURL), fileURLWithPath: &*ns_string]
    };

    // 创建 AVAsset
    let asset = unsafe { AVAsset::assetWithURL(&url) };

    // 创建 AVPlayerItem（需要 MainThreadMarker）
    let player_item = unsafe { AVPlayerItem::playerItemWithAsset(&asset, mtm) };

    // 创建 AVQueuePlayer（需要 NSArray 和 MainThreadMarker）
    let items = NSArray::from_slice(&[&*player_item]);
    let player = unsafe { AVQueuePlayer::queuePlayerWithItems(&*items, mtm) };

    // 创建 AVPlayerLooper 实现循环播放
    let _looper = unsafe { AVPlayerLooper::playerLooperWithPlayer_templateItem(&player, &player_item) };

    // 创建 AVPlayerLayer
    let player_layer = unsafe { AVPlayerLayer::playerLayerWithPlayer(Some(&player)) };

    // 设置 layer 为窗口内容视图
    if let Some(content_view) = window.contentView() {
        content_view.setWantsLayer(true);
        content_view.setLayer(Some(&player_layer));
    }

    // 设置静音
    unsafe { player.setMuted(muted) };

    // 开始播放
    unsafe { player.play() };

    // 更新状态
    let mut state = PLAYBACK_STATE.lock().map_err(|e| e.to_string())?;
    let mut info = PlayerInfo::new(file_path.to_string());
    info.state = PlaybackState::Playing;
    info.muted = muted;
    state.insert(display_id.to_string(), info);

    Ok(())
}

/// 暂停指定显示器的播放
pub fn pause(display_id: &str) -> Result<(), String> {
    let mut state = PLAYBACK_STATE.lock().map_err(|e| e.to_string())?;

    if let Some(info) = state.get_mut(display_id) {
        info.state = PlaybackState::Paused;
        // TODO: 实际暂停 AVPlayer（需要保存 player 引用）
        Ok(())
    } else {
        Err(format!("未找到显示器 {} 的播放器", display_id))
    }
}

/// 恢复指定显示器的播放
pub fn resume(display_id: &str) -> Result<(), String> {
    let mut state = PLAYBACK_STATE.lock().map_err(|e| e.to_string())?;

    if let Some(info) = state.get_mut(display_id) {
        info.state = PlaybackState::Playing;
        // TODO: 实际恢复 AVPlayer
        Ok(())
    } else {
        Err(format!("未找到显示器 {} 的播放器", display_id))
    }
}

/// 停止指定显示器的播放
pub fn stop(display_id: &str) -> Result<(), String> {
    let mut state = PLAYBACK_STATE.lock().map_err(|e| e.to_string())?;

    if let Some(info) = state.get_mut(display_id) {
        info.state = PlaybackState::Stopped;
        // TODO: 实际停止 AVPlayer
        state.remove(display_id);
        Ok(())
    } else {
        Err(format!("未找到显示器 {} 的播放器", display_id))
    }
}

/// 获取指定显示器的播放状态
pub fn get_state(display_id: &str) -> Option<PlaybackState> {
    PLAYBACK_STATE
        .lock()
        .ok()?
        .get(display_id)
        .map(|info| info.state.clone())
}

/// 设置静音
pub fn set_muted(display_id: &str, muted: bool) -> Result<(), String> {
    let mut state = PLAYBACK_STATE.lock().map_err(|e| e.to_string())?;

    if let Some(info) = state.get_mut(display_id) {
        info.muted = muted;
        // TODO: 实际设置 AVPlayer 静音
        Ok(())
    } else {
        Err(format!("未找到显示器 {} 的播放器", display_id))
    }
}
