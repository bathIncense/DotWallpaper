// 版本化 JSON 设置：壁纸目录、逐显示器分配、默认填充/静音、暂停状态。
// 单文件存储于应用配置目录，不引入 SQLite。损坏时回退默认并备份。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::types::{FitMode, WallpaperAssignment};

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub version: u32,
    pub library_dir: String,
    pub default_fit_mode: FitMode,
    pub default_muted: bool,
    #[serde(default)]
    pub assignments: HashMap<String, WallpaperAssignment>,
    /// 处于用户暂停状态的显示器 ID
    #[serde(default)]
    pub paused_displays: Vec<String>,
}

fn default_library_dir() -> String {
    match std::env::var("HOME") {
        Ok(home) => format!("{home}/Pictures"),
        Err(_) => std::env::temp_dir().display().to_string(),
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            library_dir: default_library_dir(),
            default_fit_mode: FitMode::Fill,
            default_muted: true,
            assignments: HashMap::new(),
            paused_displays: Vec::new(),
        }
    }
}

struct Store {
    path: PathBuf,
    settings: AppSettings,
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);

/// 启动时加载设置；文件缺失用默认值，解析失败或版本不受支持则备份后回退默认。
pub fn init(app: &tauri::AppHandle) {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| default_library_dir().into());
    let path = dir.join("settings.json");
    let settings = load_from(&path);
    let mut store = STORE.lock().expect("settings lock");
    *store = Some(Store { path, settings });
    save_locked(store.as_mut().expect("just initialized"));
}

pub(crate) fn load_from(path: &std::path::Path) -> AppSettings {
    match std::fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<AppSettings>(&text) {
            Ok(s) if s.version == SETTINGS_VERSION => s,
            Ok(s) => {
                eprintln!("[settings] 未知配置版本 {}，回退默认并备份", s.version);
                let _ = std::fs::rename(path, path.with_extension("json.bak"));
                AppSettings::default()
            }
            Err(e) => {
                eprintln!("[settings] 配置损坏，回退默认并备份: {e}");
                let _ = std::fs::rename(path, path.with_extension("json.bak"));
                AppSettings::default()
            }
        },
        Err(_) => AppSettings::default(),
    }
}

pub fn get() -> AppSettings {
    STORE
        .lock()
        .expect("settings lock")
        .as_ref()
        .map(|s| s.settings.clone())
        .unwrap_or_default()
}

fn save_locked(store: &mut Store) {
    if let Some(parent) = store.path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&store.settings) {
        if let Err(e) = std::fs::write(&store.path, text) {
            eprintln!("[settings] 保存失败: {e}");
        }
    }
}

pub fn update<F: FnOnce(&mut AppSettings)>(f: F) -> AppSettings {
    let mut guard = STORE.lock().expect("settings lock");
    let store = guard.as_mut().expect("settings not init");
    f(&mut store.settings);
    store.settings.version = SETTINGS_VERSION;
    save_locked(store);
    store.settings.clone()
}

pub fn record_assignment(assignment: &WallpaperAssignment) {
    update(|s| {
        s.assignments.insert(assignment.display_id.clone(), assignment.clone());
    });
}

#[cfg(test)]
pub fn init_for_test(path: PathBuf, settings: AppSettings) {
    let mut store = STORE.lock().expect("settings lock");
    *store = Some(Store { path, settings });
}

pub fn set_display_paused(display_id: &str, paused: bool) {
    update(|s| {
        let mut set: HashSet<String> = s.paused_displays.iter().cloned().collect();
        if paused {
            set.insert(display_id.to_string());
        } else {
            set.remove(display_id);
        }
        s.paused_displays = set.into_iter().collect();
    });
}
