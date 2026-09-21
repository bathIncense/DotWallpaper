// 主线程调度：所有 AppKit/AVFoundation 对象只在 macOS 主线程访问。
// 工作线程通过 on_main 同步取结果；主线程回调（托盘事件等）用 spawn_main 投递。

use std::sync::OnceLock;

use objc2_foundation::MainThreadMarker;
use tauri::AppHandle;

static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

pub fn app() -> Result<AppHandle, String> {
    APP.get().cloned().ok_or_else(|| "应用尚未初始化".to_string())
}

/// 在主线程执行并等待结果。已在主线程时直接内联执行（避免自锁）。
pub fn on_main<T: Send + 'static>(
    f: impl FnOnce(&MainThreadMarker) -> T + Send + 'static,
) -> Result<T, String> {
    if let Some(mtm) = MainThreadMarker::new() {
        return Ok(f(&mtm));
    }
    let (tx, rx) = std::sync::mpsc::channel();
    app()?
        .run_on_main_thread(move || {
            let mtm = MainThreadMarker::new().expect("run_on_main_thread 回调必须在主线程");
            let _ = tx.send(f(&mtm));
        })
        .map_err(|e| format!("主线程调度失败: {e}"))?;
    rx.recv().map_err(|e| format!("主线程结果丢失: {e}"))
}

/// 投递到主线程执行，不等待结果。
pub fn spawn_main(f: impl FnOnce(&MainThreadMarker) + Send + 'static) {
    if let Some(mtm) = MainThreadMarker::new() {
        f(&mtm);
        return;
    }
    if let Ok(app) = app() {
        let _ = app.run_on_main_thread(move || {
            let mtm = MainThreadMarker::new().expect("run_on_main_thread 回调必须在主线程");
            f(&mtm);
        });
    }
}
