// 主线程调度：所有 AppKit/AVFoundation 对象只在 macOS 主线程访问。
// 使用 GCD dispatch 在主线程执行并等待结果。

use objc2_foundation::MainThreadMarker;

/// 初始化 runtime（GCD-based，无需额外初始化）
pub fn init() {}

/// 在主线程执行并等待结果。已在主线程时直接内联执行（避免死锁）。
pub fn on_main<T: Send + 'static>(
    f: impl FnOnce(&MainThreadMarker) -> T + Send + 'static,
) -> Result<T, String> {
    if let Some(mtm) = MainThreadMarker::new() {
        return Ok(f(&mtm));
    }

    // Rust unit tests do not start NSApplication's main run loop. Keep their
    // pure validation paths synchronous instead of waiting forever on a main
    // queue that is not being serviced. The production path below always uses
    // the real macOS main queue.
    #[cfg(test)]
    {
        // SAFETY: test-only callers exercise logic that does not retain or
        // access AppKit objects after returning. Production code never uses
        // this branch.
        Ok(f(unsafe { &MainThreadMarker::new_unchecked() }))
    }

    #[cfg(not(test))]
    {
        let (tx, rx) = std::sync::mpsc::channel();

        let task = Box::new(move || {
            let mtm = MainThreadMarker::new().expect("must be on main thread");
            let result = f(&mtm);
            let _ = tx.send(result);
        });

        // dispatch_sync(main) can execute its block on the calling thread as
        // an optimization, even though it owns the main queue. AppKit needs
        // the actual main thread; enqueue asynchronously, then wait here.
        dispatch2::DispatchQueue::main().exec_async(task);

        rx.recv().map_err(|e| format!("主线程结果丢失: {e}"))
    }
}

/// 异步在主线程执行（不等待结果）
pub fn on_main_async(f: impl FnOnce(&MainThreadMarker) + Send + 'static) {
    if let Some(mtm) = MainThreadMarker::new() {
        f(&mtm);
        return;
    }

    #[cfg(test)]
    {
        // SAFETY: see the synchronous test-only path in `on_main`.
        f(unsafe { &MainThreadMarker::new_unchecked() });
    }

    #[cfg(not(test))]
    {
        // Do not use a global queue here: AppKit/AVFoundation objects must be
        // created and touched on the actual macOS main thread.
        dispatch2::DispatchQueue::main().exec_async(move || {
            let mtm = MainThreadMarker::new().expect("must be on main thread");
            f(&mtm);
        });
    }
}

/// 获取应用支持目录路径
pub fn app_support_dir() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "无法获取 HOME 目录")?;
    let mut path = std::path::PathBuf::from(home);
    path.push("Library/Application Support/com.dot.wallpaper");
    Ok(path)
}

/// 获取缓存目录路径
pub fn app_cache_dir() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "无法获取 HOME 目录")?;
    let mut path = std::path::PathBuf::from(home);
    path.push("Library/Caches/com.dot.wallpaper");
    Ok(path)
}

/// 获取图片目录路径
pub fn picture_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let pictures = std::path::PathBuf::from(&home).join("Pictures");
    if pictures.is_dir() {
        pictures
    } else {
        std::path::PathBuf::from(&home)
    }
}
