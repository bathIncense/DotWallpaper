// DotWallpaper 壁纸工具 - Tauri 后端入口（macOS）
// 命令边界收敛为改造计划 §2.1 定义的 9 个命令 + 文件选择扩展。

mod cfmedia;
mod desktop;
mod displays;
mod engine;
mod media;
mod runtime;
mod settings;
mod thumbs;
mod types;
mod unit_tests;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use types::{
    ControlAction, DisplayInfo, DisplayWallpaperState, FitMode, MediaItem, WallpaperAssignment,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSnapshot {
    library_dir: String,
    displays: Vec<DisplayInfo>,
    states: Vec<DisplayWallpaperState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    saved: Vec<String>,
    skipped: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrontendSettings {
    library_dir: String,
    default_fit_mode: FitMode,
    default_muted: bool,
}

fn snapshot() -> AppSnapshot {
    AppSnapshot {
        library_dir: settings::get().library_dir,
        displays: displays::enumerate(),
        states: engine::states_snapshot(),
    }
}

fn apply_library_dir(dir: &str) {
    let path = PathBuf::from(dir.trim());
    if path.is_dir() {
        media::ensure_asset_scope(&path);
    }
}

#[tauri::command]
fn get_app_snapshot() -> AppSnapshot {
    snapshot()
}

#[tauri::command]
async fn list_media() -> Result<Vec<MediaItem>, String> {
    let dir = settings::get().library_dir;
    tauri::async_runtime::spawn_blocking(move || -> Vec<MediaItem> {
        let items = media::scan(&dir);
        thumbs::make_entries(items)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_displays() -> Vec<DisplayInfo> {
    displays::enumerate()
}

#[tauri::command]
async fn apply_wallpaper(assignment: WallpaperAssignment) -> Result<DisplayWallpaperState, String> {
    tauri::async_runtime::spawn_blocking(move || engine::apply(assignment))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn control_playback(
    display_id: String,
    action: ControlAction,
) -> Result<DisplayWallpaperState, String> {
    tauri::async_runtime::spawn_blocking(move || engine::control(display_id, action))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pick_library_directory() -> Result<Option<String>, String> {
    let picked = runtime::on_main(|mtm| desktop::pick_directory(mtm))?;
    if let Some(dir) = &picked {
        apply_library_dir(dir);
        settings::update(|s| s.library_dir = dir.clone());
    }
    Ok(picked)
}

#[tauri::command]
async fn pick_media_files() -> Result<Vec<String>, String> {
    let picked = runtime::on_main(|mtm| Ok::<_, String>(desktop::pick_files(mtm)))??;
    Ok(picked)
}

#[tauri::command]
async fn import_media(paths: Vec<String>) -> Result<ImportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (saved, skipped) = media::import(&paths);
        ImportResult { saved, skipped }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_media(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || media::delete(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn update_settings(new_settings: FrontendSettings) -> Result<(), String> {
    let dir = new_settings.library_dir.clone();
    settings::update(|s| {
        s.library_dir = new_settings.library_dir;
        s.default_fit_mode = new_settings.default_fit_mode;
        s.default_muted = new_settings.default_muted;
    });
    apply_library_dir(&dir);
    Ok(())
}

/// 关闭管理窗口 = 隐藏窗口，动态壁纸继续运行；仅托盘/退出动作结束进程。
fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "main" {
            api.prevent_close();
            let _ = window.hide();
        }
    }
}

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let open = MenuItem::with_id(app, "open", "打开 DotWallpaper", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause_all", "暂停全部动态壁纸", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume_all", "恢复全部", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop_all", "停止全部动态壁纸", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = PredefinedMenuItem::quit(app, Some("退出"))?;
    let menu = Menu::with_items(app, &[&open, &sep, &pause, &resume, &stop, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("DotWallpaper")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.unminimize();
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "pause_all" => engine::pause_all(),
            "resume_all" => engine::resume_all(),
            "stop_all" => engine::stop_all(),
            "quit" => {
                engine::stop_all();
                engine::teardown_all();
                app.exit(0);
            }
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            runtime::init(handle);
            settings::init(handle);

            // asset protocol scope：缓存目录 + 海报目录 + 壁纸库目录
            for dir in [thumbs::cache_dir().ok(), Some(thumbs::poster_dir()), {
                let d = PathBuf::from(settings::get().library_dir);
                d.is_dir().then_some(d)
            }]
            .into_iter()
            .flatten()
            {
                let _ = std::fs::create_dir_all(&dir);
                media::ensure_asset_scope(&dir);
            }

            build_tray(handle)?;
            engine::restore_on_startup();
            engine::spawn_monitor();
            Ok(())
        })
        .on_window_event(handle_window_event)
        .invoke_handler(tauri::generate_handler![
            get_app_snapshot,
            list_media,
            list_displays,
            apply_wallpaper,
            control_playback,
            pick_library_directory,
            pick_media_files,
            import_media,
            delete_media,
            update_settings
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("Tauri 应用启动失败: {e}");
            std::process::exit(1);
        });
}
