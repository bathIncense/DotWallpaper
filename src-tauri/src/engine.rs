// macOS 视频壁纸引擎。
// - 每块显示器一个 PlaybackSession，仅在 macOS 主线程访问（thread_local 持有全部对象）
// - AVQueuePlayer + AVPlayerLooper + AVPlayerLayer 共同存活，窗口位于桌面图标之下
// - 先准备新会话（隐藏），首帧就绪后替换旧会话；失败保留旧壁纸并返回明确错误
// - 显示器热插拔 / 休眠唤醒由监视线程处理，操作一律投递回主线程

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_av_foundation::{
    AVAsset, AVAssetImageGenerator, AVPlayerItem, AVPlayerItemStatus, AVPlayerLayer,
    AVPlayerLooper, AVPlayerStatus, AVQueuePlayer,
};
use objc2_core_media::CMTime;
use objc2_foundation::MainThreadMarker;

use crate::desktop;
use crate::displays;
use crate::media;
use crate::runtime::{on_main, spawn_main};
use crate::settings;
use crate::types::{
    ControlAction, DisplayWallpaperState, FitMode, MediaKind, Phase, WallpaperAssignment,
};

/// 一个显示器的完整播放会话（播放器、循环器、图层共同存活）
struct Session {
    window: Retained<NSWindow>,
    player: Retained<AVQueuePlayer>,
    item: Retained<AVPlayerItem>,
    _looper: Retained<AVPlayerLooper>,
    layer: Retained<AVPlayerLayer>,
    assignment: WallpaperAssignment,
    phase: Phase,
    user_paused: bool,
    sleep_paused: bool,
}

thread_local! {
    /// 当前正在展示的会话
    static SESSIONS: RefCell<HashMap<String, Session>> = RefCell::new(HashMap::new());
    /// 首帧就绪前的候补会话（隐藏窗口）
    static STAGED: RefCell<HashMap<String, Session>> = RefCell::new(HashMap::new());
}

/// 任意线程可读的最新状态镜像（主线程写入）
fn states() -> &'static Mutex<HashMap<String, DisplayWallpaperState>> {
    static STATES: std::sync::OnceLock<Mutex<HashMap<String, DisplayWallpaperState>>> =
        std::sync::OnceLock::new();
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn publish(state: &DisplayWallpaperState) {
    if let Ok(mut map) = states().lock() {
        map.insert(state.display_id.clone(), state.clone());
    }
}

pub fn states_snapshot() -> Vec<DisplayWallpaperState> {
    states()
        .lock()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}

fn state_of(
    display_id: &str,
    phase: Phase,
    assignment: Option<WallpaperAssignment>,
    error: Option<String>,
) -> DisplayWallpaperState {
    DisplayWallpaperState { display_id: display_id.to_string(), phase, assignment, error }
}

fn same_path(a: &str, b: &str) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// 视频海报输出路径（应用缓存目录）
pub fn poster_path(video: &str) -> PathBuf {
    crate::thumbs::poster_dir().join(crate::thumbs::hashed_file_name(video, "jpg"))
}

#[allow(deprecated)]
/// 用 AVAssetImageGenerator 抽取首帧并编码为 JPEG（主线程调用）。
pub fn generate_poster(video: &str, out: &PathBuf) -> Result<(), String> {
    let _mtm = MainThreadMarker::new().ok_or("海报生成必须在主线程")?;
    let url = desktop::ns_url_for_path(video);
    let asset = unsafe { AVAsset::assetWithURL(&url) };
    let gen: Retained<AVAssetImageGenerator> = unsafe {
        objc2::msg_send![objc2::class!(AVAssetImageGenerator), imageGeneratorWithAsset: &*asset]
    };
    unsafe { gen.setAppliesPreferredTrackTransform(true) };
    let time = unsafe { CMTime::new(0, 600) };
    let mut actual = unsafe { std::mem::zeroed() };
    let image = unsafe {
        gen.copyCGImageAtTime_actualTime_error(time, &mut actual)
    }
    .map_err(|e| format!("无法提取视频首帧: {e}"))?;
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    crate::cfmedia::encode_jpeg(
        &*image as *const _ as *const _,
        &out.to_string_lossy(),
        0.9,
    )
}

/// 统一应用入口（任意线程调用，内部调度到主线程）。
pub fn apply(assignment: WallpaperAssignment) -> Result<DisplayWallpaperState, String> {
    let path = media::validate_for_apply(&assignment.path, assignment.kind)?;
    let assignment = WallpaperAssignment { path, ..assignment };

    // 幂等：同显示器同分配且已生效时不重复应用
    {
        let guard = states().lock().map_err(|e| e.to_string())?;
        if let Some(s) = guard.get(&assignment.display_id) {
            let same = s.assignment.as_ref().is_some_and(|a| *a == assignment);
            if same && matches!(s.phase, Phase::Static | Phase::Playing | Phase::Paused) {
                return Ok(s.clone());
            }
        }
    }

    let a = assignment.clone();
    let state = match assignment.kind {
        MediaKind::Image => on_main(move |mtm| apply_static_main(mtm, &a))?,
        MediaKind::Video => {
            let state = on_main(move |mtm| start_video_main(mtm, &a))?;
            spawn_prepare_watch(assignment.display_id.clone());
            state
        }
    }?;

    publish(&state);
    if state.phase != Phase::Error {
        settings::record_assignment(&state.assignment.clone().unwrap_or(assignment));
    }
    Ok(state)
}

fn apply_static_main(
    mtm: &MainThreadMarker,
    a: &WallpaperAssignment,
) -> Result<DisplayWallpaperState, String> {
    let screen = desktop::screen_for_stable_id(mtm, &a.display_id)
        .ok_or_else(|| format!("未找到显示器: {}", a.display_id))?;
    // 切静态前必须先停掉该显示器的视频会话
    destroy_sessions_for(mtm, &a.display_id);
    desktop::set_static_wallpaper(mtm, &screen, &a.path, a.fit_mode)?;
    // 以系统读取结果作为最终状态
    match desktop::current_static_wallpaper(mtm, &screen) {
        Some(read) if !read.is_empty() && !same_path(&read, &a.path) => Err(format!(
            "系统壁纸未生效（当前: {read}）"
        )),
        _ => Ok(state_of(&a.display_id, Phase::Static, Some(a.clone()), None)),
    }
}

fn start_video_main(
    mtm: &MainThreadMarker,
    a: &WallpaperAssignment,
) -> Result<DisplayWallpaperState, String> {
    let screen = desktop::screen_for_stable_id(mtm, &a.display_id)
        .ok_or_else(|| format!("未找到显示器: {}", a.display_id))?;

    // 首帧海报设为该屏系统静态壁纸：播放启动前避免黑屏，停止后仍保留合理背景
    let poster = poster_path(&a.path);
    match generate_poster(&a.path, &poster) {
        Ok(()) => {
            if let Err(e) = desktop::set_static_wallpaper(mtm, &screen, &poster.to_string_lossy(), FitMode::Fill) {
                eprintln!("[poster] 设置海报壁纸失败: {e}");
            }
        }
        Err(e) => eprintln!("[poster] {e}"),
    }

    let session = build_session(a, &screen)?;
    // 候补会话先隐藏，首帧就绪后再替换旧会话
    STAGED.with(|s| s.borrow_mut().insert(a.display_id.clone(), session));
    Ok(state_of(&a.display_id, Phase::Preparing, Some(a.clone()), None))
}

fn build_session(
    a: &WallpaperAssignment,
    screen: &objc2_app_kit::NSScreen,
) -> Result<Session, String> {
    let frame = screen.frame();
    let window: Retained<NSWindow> = unsafe {
        let cls = objc2::class!(NSWindow);
        let allocated: *mut NSWindow = objc2::msg_send![cls, alloc];
        let window: *mut NSWindow = objc2::msg_send![allocated, initWithContentRect: frame,
            styleMask: NSWindowStyleMask::Borderless,
            backing: NSBackingStoreType::Buffered,
            defer: false];
        Retained::from_raw(window).ok_or("桌面播放窗口创建失败")?
    };
    configure_window(&window, frame);

    let url = desktop::ns_url_for_path(&a.path);
    let asset = unsafe { AVAsset::assetWithURL(&url) };
    let item: Retained<AVPlayerItem> = unsafe {
        objc2::msg_send![objc2::class!(AVPlayerItem), playerItemWithAsset: &*asset]
    };
    let items = objc2_foundation::NSArray::from_slice(std::slice::from_ref(&&*item));
    let player: Retained<AVQueuePlayer> = unsafe {
        objc2::msg_send![objc2::class!(AVQueuePlayer), queuePlayerWithItems: &*items]
    };
    let looper = unsafe { AVPlayerLooper::playerLooperWithPlayer_templateItem(&player, &item) };
    let layer = unsafe { AVPlayerLayer::playerLayerWithPlayer(Some(&player)) };
    unsafe {
        let gravity = match a.fit_mode {
            FitMode::Fill => objc2_av_foundation::AVLayerVideoGravityResizeAspectFill
                .expect("AVLayerVideoGravityResizeAspectFill"),
            FitMode::Fit => objc2_av_foundation::AVLayerVideoGravityResizeAspect
                .expect("AVLayerVideoGravityResizeAspect"),
        };
        layer.setVideoGravity(gravity);
        player.setMuted(a.muted);
    }
    if let Some(view) = window.contentView() {
        view.setWantsLayer(true);
        view.setLayer(Some(&layer));
        layer.setFrame(view.bounds());
    }
    unsafe { player.play() };

    Ok(Session {
        window,
        player,
        item,
        _looper: looper,
        layer,
        assignment: a.clone(),
        phase: Phase::Preparing,
        user_paused: false,
        sleep_paused: false,
    })
}

fn configure_window(window: &NSWindow, frame: objc2_foundation::NSRect) {
    unsafe {
        // 桌面图标之下、系统桌面背景之上；不假定主屏原点为 (0,0)
        let level = objc2_core_graphics::CGWindowLevelForKey(
            objc2_core_graphics::CGWindowLevelKey::DesktopIconWindowLevelKey,
        );
        window.setLevel((level - 1) as isize);
    }
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    window.setOpaque(false);
    window.setHasShadow(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setIgnoresMouseEvents(true);
    window.setAcceptsMouseMovedEvents(false);
    window.setCanHide(false);
    unsafe { window.setReleasedWhenClosed(false) };
    window.setFrame_display(frame, false);
    // 注意：新会话先不 orderFront，首帧就绪后替换时再显示
}

fn close_session(session: &Session) {
    unsafe {
        session.player.pause();
        session.window.orderOut(None);
    }
}

fn destroy_sessions_for(_mtm: &MainThreadMarker, display_id: &str) {
    STAGED.with(|s| {
        if let Some(sess) = s.borrow_mut().remove(display_id) {
            close_session(&sess);
        }
    });
    SESSIONS.with(|s| {
        if let Some(sess) = s.borrow_mut().remove(display_id) {
            close_session(&sess);
        }
    });
}

/// 首帧就绪监视：就绪后替换旧会话并显示；失败/超时销毁候补、保留旧壁纸。
fn spawn_prepare_watch(display_id: String) {
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            std::thread::sleep(Duration::from_millis(200));
            let id = display_id.clone();
            let outcome = on_main(move |_mtm| watch_step(&id));
            let done = match outcome {
                Ok(WatchOutcome::Waiting) => {
                    if Instant::now() >= deadline {
                        Some(fail_staged(&display_id, "播放器准备超时（文件可能缺失或系统无法解码）"))
                    } else {
                        None
                    }
                }
                Ok(WatchOutcome::Ready(state)) => Some(state),
                Ok(WatchOutcome::Failed(msg)) => Some(fail_staged(&display_id, &msg)),
                Ok(WatchOutcome::Gone) | Err(_) => return,
            };
            if let Some(state) = done {
                publish(&state);
                if state.phase == Phase::Playing {
                    if let Some(a) = state.assignment.clone() {
                        settings::record_assignment(&a);
                    }
                }
                return;
            }
        }
    });
}

enum WatchOutcome {
    Waiting,
    Gone,
    Ready(DisplayWallpaperState),
    Failed(String),
}

fn watch_step(display_id: &str) -> WatchOutcome {
    let staged = STAGED.with(|s| s.borrow_mut().remove(display_id));
    let Some(session) = staged else { return WatchOutcome::Gone };
    let istatus = unsafe { session.item.status() };
    if istatus == AVPlayerItemStatus::Failed {
        let msg = unsafe { session.item.error() }
            .map(|e| format!("播放失败: {}", e.localizedDescription()))
            .unwrap_or_else(|| "播放器准备失败".to_string());
        close_session(&session);
        return WatchOutcome::Failed(msg);
    }
    let pstatus = unsafe { session.player.status() };
    if istatus == AVPlayerItemStatus::ReadyToPlay && pstatus == AVPlayerStatus::ReadyToPlay {
        // 首帧就绪：销毁旧会话，显示新会话
        let old = SESSIONS.with(|s| s.borrow_mut().insert(display_id.to_string(), session));
        if let Some(old) = old {
            close_session(&old);
        }
        SESSIONS.with(|s| {
            if let Some(sess) = s.borrow_mut().get_mut(display_id) {
                sess.phase = Phase::Playing;
                unsafe { sess.window.orderFront(None::<&AnyObject>) };
            }
        });
        let assignment = SESSIONS.with(|s| {
            s.borrow().get(display_id).map(|x| x.assignment.clone())
        });
        return WatchOutcome::Ready(state_of(
            display_id,
            Phase::Playing,
            assignment,
            None,
        ));
    }
    // 尚未就绪，放回候补
    STAGED.with(|s| s.borrow_mut().insert(display_id.to_string(), session));
    WatchOutcome::Waiting
}

fn fail_staged(display_id: &str, msg: &str) -> DisplayWallpaperState {
    let err = msg.to_string();
    let id = display_id.to_string();
    let _ = on_main(move |_mtm| {
        if let Some(sess) = STAGED.with(|s| s.borrow_mut().remove(&id)) {
            close_session(&sess);
        }
    });
    let prev = states().lock().ok().and_then(|m| m.get(display_id).cloned());
    let assignment = prev
        .as_ref()
        .and_then(|s| s.assignment.clone())
        .or_else(|| settings::get().assignments.get(display_id).cloned());
    state_of(display_id, Phase::Error, assignment, Some(format!("无法播放该视频：{err}")))
}

/// 暂停 / 恢复 / 停止（全部调度到主线程操作播放器对象）
pub fn control(display_id: String, action: ControlAction) -> Result<DisplayWallpaperState, String> {
    let state = on_main(move |_mtm| control_main(&display_id, action))??;
    publish(&state);
    match action {
        ControlAction::Pause => settings::set_display_paused(&state.display_id, true),
        ControlAction::Resume => settings::set_display_paused(&state.display_id, false),
        ControlAction::Stop => {
            settings::set_display_paused(&state.display_id, false);
            if let Some(a) = state.assignment.clone() {
                settings::record_assignment(&a);
            }
        }
    }
    Ok(state)
}

fn control_main(display_id: &str, action: ControlAction) -> Result<DisplayWallpaperState, String> {
    SESSIONS.with(|cell| {
        let mut map = cell.borrow_mut();
        let Some(session) = map.get_mut(display_id) else {
            return match action {
                ControlAction::Stop => Ok(state_of(display_id, Phase::Static, None, None)),
                _ => Err(format!("显示器 {display_id} 当前没有动态壁纸在播放")),
            };
        };
        match action {
            ControlAction::Pause => unsafe { session.player.pause() },
            ControlAction::Resume => unsafe { session.player.play() },
            ControlAction::Stop => {
                let sess = map.remove(display_id).expect("just borrowed");
                close_session(&sess);
                // 停止后系统壁纸仍是该视频的首帧海报
                return Ok(state_of(display_id, Phase::Static, Some(sess.assignment), None));
            }
        }
        session.user_paused = action == ControlAction::Pause;
        if action == ControlAction::Resume {
            session.sleep_paused = false;
        }
        session.phase = match action {
            ControlAction::Pause => Phase::Paused,
            _ => Phase::Playing,
        };
        Ok(state_of(display_id, session.phase, Some(session.assignment.clone()), None))
    })
}

fn bulk_control(action: ControlAction) {
    spawn_main(move |_mtm| {
        let ids = SESSIONS.with(|s| s.borrow().keys().cloned().collect::<Vec<_>>());
        for id in ids {
            if let Ok(state) = control_main(&id, action) {
                publish(&state);
                settings::set_display_paused(&id, action == ControlAction::Pause);
            }
        }
    });
}

pub fn pause_all() {
    bulk_control(ControlAction::Pause);
}

pub fn resume_all() {
    bulk_control(ControlAction::Resume);
}

pub fn stop_all() {
    bulk_control(ControlAction::Stop);
}

/// 退出时销毁全部播放会话（主线程调用）
pub fn teardown_all() {
    spawn_main(move |mtm| {
        destroy_sessions_for(mtm, "");
        let ids = SESSIONS.with(|s| s.borrow().keys().cloned().collect::<Vec<_>>());
        for id in ids {
            destroy_sessions_for(mtm, &id);
        }
    });
}

/// 启动时恢复逐屏配置；缺失媒体标记错误，不阻塞其他显示器。
pub fn restore_on_startup() {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(300));
        let s = settings::get();
        for assignment in s.assignments.values() {
            let display_id = &assignment.display_id;
            let a = assignment.clone();
            if displays::cg_id_for_stable(display_id).is_none() {
                publish(&state_of(
                    display_id,
                    Phase::Error,
                    Some(a),
                    Some("显示器未连接，重连后将自动恢复".to_string()),
                ));
                continue;
            }
            match apply(a.clone()) {
                Ok(state) => {
                    publish(&state);
                    if s.paused_displays.iter().any(|d| d == display_id)
                        && state.phase == Phase::Preparing
                    {
                        // 就绪后按持久化状态暂停（watch 完成后短暂延迟控制）
                        let id = display_id.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_millis(2500));
                            let _ = control(id, ControlAction::Pause);
                        });
                    }
                }
                Err(e) => {
                    publish(&state_of(display_id, Phase::Error, Some(a), Some(e)));
                }
            }
        }
    });
}

/// 显示器热插拔 + 休眠唤醒 + 分辨率变化监视线程。
pub fn spawn_monitor() {
    std::thread::spawn(|| {
        let mut seen: HashMap<String, ((f64, f64, f64, f64), bool)> = HashMap::new();
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let sessions = on_main(|_mtm| {
                SESSIONS.with(|s| s.borrow().keys().cloned().collect::<Vec<String>>())
            })
            .unwrap_or_default();

            for id in &sessions {
                let Some(cgid) = displays::cg_id_for_stable(id) else {
                    // 显示器拔出：销毁对应播放窗口
                    let did = id.clone();
                    spawn_main(move |mtm| destroy_sessions_for(mtm, &did));
                    let prev = states().lock().ok().and_then(|m| m.get(id).cloned());
                    publish(&state_of(
                        id,
                        Phase::Error,
                        prev.and_then(|s| s.assignment),
                        Some("显示器已断开".to_string()),
                    ));
                    continue;
                };
                let asleep = displays::is_asleep(cgid);
                let bounds = displays::logical_bounds(cgid);
                let changed = seen.get(id).is_some_and(|(b, _)| *b != bounds);
                seen.insert(id.clone(), (bounds, asleep));
                let did = id.clone();
                if asleep {
                    // 休眠前暂停播放器
                    spawn_main(move |_mtm| {
                        SESSIONS.with(|s| {
                            if let Some(sess) = s.borrow_mut().get_mut(&did) {
                                if !sess.user_paused {
                                    unsafe { sess.player.pause() };
                                    sess.sleep_paused = true;
                                }
                            }
                        });
                    });
                } else {
                    let resume = seen.get(id).is_some_and(|(_, a)| !*a) || changed;
                    if resume {
                        // 唤醒/重连/分辨率变化：重新定位窗口并恢复播放
                        let did2 = id.clone();
                        spawn_main(move |mtm| {
                            SESSIONS.with(|s| {
                                if let Some(sess) = s.borrow_mut().get_mut(&did2) {
                                    if let Some(screen) =
                                        desktop::screen_for_stable_id(mtm, &did2)
                                    {
                                        let frame = screen.frame();
                                        sess.window.setFrame_display(frame, true);
                                        if let Some(view) = sess.window.contentView() {
                                            sess.layer.setFrame(view.bounds());
                                        }
                                    }
                                    if !sess.user_paused && sess.sleep_paused {
                                        unsafe { sess.player.play() };
                                        sess.sleep_paused = false;
                                    }
                                }
                            });
                        });
                    }
                }
            }

            // 重新连接的显示器：按持久化配置恢复动态壁纸
            let s = settings::get();
            for (id, a) in &s.assignments {
                if a.kind == MediaKind::Video
                    && !sessions.contains(id)
                    && displays::cg_id_for_stable(id).is_some()
                {
                    let assignment = a.clone();
                    let id = id.clone();
                    std::thread::spawn(move || {
                        if let Ok(state) = apply(assignment) {
                            publish(&state);
                        } else if let Ok(mut m) = states().lock() {
                            if let Some(st) = m.get_mut(&id) {
                                st.phase = Phase::Static;
                            }
                        }
                    });
                }
            }
        }
    });
}
