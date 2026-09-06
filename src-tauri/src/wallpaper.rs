// 壁纸相关后端实现：
// - macOS：扫描本地壁纸目录 + 用户图片文件夹
// - 支持静态图片、视频、GIF、HEIC

use std::path::PathBuf;

/// 支持的壁纸图片扩展名（列表扫描 / 拖入导入 / 删除共用同一权威列表）
const SUPPORTED_IMAGE_EXTS: [&str; 7] = ["jpg", "jpeg", "png", "bmp", "webp", "heic", "heif"];

/// 支持的视频扩展名
const SUPPORTED_VIDEO_EXTS: [&str; 3] = ["mp4", "mov", "m4v"];

/// GIF 扩展名
const SUPPORTED_GIF_EXTS: [&str; 1] = ["gif"];

/// 是否受支持的壁纸图片扩展名（大小写不敏感）
pub fn is_supported_image_ext(ext: &str) -> bool {
    SUPPORTED_IMAGE_EXTS.iter().any(|s| s.eq_ignore_ascii_case(ext))
}

/// 是否受支持的视频扩展名
pub fn is_supported_video_ext(ext: &str) -> bool {
    SUPPORTED_VIDEO_EXTS.iter().any(|s| s.eq_ignore_ascii_case(ext))
}

/// 是否受支持的 GIF 扩展名
pub fn is_supported_gif_ext(ext: &str) -> bool {
    SUPPORTED_GIF_EXTS.iter().any(|s| s.eq_ignore_ascii_case(ext))
}

/// 是否受支持的媒体扩展名（图片、视频、GIF）
pub fn is_supported_media_ext(ext: &str) -> bool {
    is_supported_image_ext(ext) || is_supported_video_ext(ext) || is_supported_gif_ext(ext)
}

/// macOS 系统壁纸目录（只读展示）
const SYSTEM_WALLPAPER_DIR: &str = "/System/Library/Desktop Pictures";

/// macOS 额外的用户壁纸目录
const SYSTEM_WALLPAPER_DIR_LIBRARY: &str = "/Library/Desktop Pictures";

/// 获取系统壁纸目录列表
fn system_wallpaper_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    let system_dir = PathBuf::from(SYSTEM_WALLPAPER_DIR);
    if system_dir.is_dir() {
        dirs.push(system_dir);
    }

    let library_dir = PathBuf::from(SYSTEM_WALLPAPER_DIR_LIBRARY);
    if library_dir.is_dir() {
        dirs.push(library_dir);
    }

    dirs
}

/// 本地壁纸目录列表（用户图片文件夹）
fn wallpaper_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(&home).join("Pictures"));
    }

    dirs
}

/// 扫描壁纸目录，返回全部壁纸文件路径列表。
///
/// 不设硬性条数上限：大批量图片由前端分页 + 缩略图后台渐进生成承载。
/// 传入 `custom_dir`（Some 且非空）时只扫描该目录；否则使用预设目录。
pub fn scan_local_wallpapers(custom_dir: Option<String>) -> Result<Vec<String>, String> {
    let mut results: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 自定义目录优先：仅扫描用户指定目录
    if let Some(dir) = custom_dir {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            let dir = PathBuf::from(trimmed);
            if !dir.is_dir() {
                return Err(format!("目录不存在或不可访问：{trimmed}"));
            }
            if let Ok(entries) = walk_dir(&dir) {
                for path in entries {
                    if seen.insert(path.clone()) {
                        results.push(path);
                    }
                }
            }
            return Ok(results);
        }
    }

    for dir in wallpaper_dirs() {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = walk_dir(&dir) {
            for path in entries {
                if seen.insert(path.clone()) {
                    results.push(path);
                }
            }
        }
    }

    Ok(results)
}

/// 扫描系统壁纸目录，返回壁纸文件路径列表。
///
/// 仅供"系统壁纸"选项卡只读展示；调用方不得对返回路径执行删除/写入。
pub fn scan_system_wallpapers() -> Result<Vec<String>, String> {
    let dirs = system_wallpaper_dirs();
    if dirs.is_empty() {
        // 系统壁纸目录不存在时返回空列表而非报错
        return Ok(Vec::new());
    }

    let mut results: Vec<String> = Vec::new();
    for dir in dirs {
        if let Ok(entries) = walk_dir(&dir) {
            for path in entries {
                results.push(path);
            }
        }
    }
    Ok(results)
}

/// 判断路径是否位于系统目录下（只读保护）
/// macOS: /System/Library/Desktop Pictures, /Library/Desktop Pictures
fn is_under_system_dir(path: &std::path::Path) -> bool {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    let norm = abs.canonicalize().unwrap_or(abs);
    let s = norm.to_string_lossy().to_lowercase();

    s.starts_with("/system/library/desktop pictures")
        || s.starts_with("/library/desktop pictures")
}

/// 递归遍历目录，收集全部支持的媒体文件
fn walk_dir(dir: &PathBuf) -> std::io::Result<Vec<String>> {
    let mut found: Vec<String> = Vec::new();
    let mut stack = vec![dir.clone()];

    while let Some(current) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if is_supported_media_ext(ext) {
                    if let Some(p) = path.to_str() {
                        found.push(p.to_string());
                    }
                }
            }
        }
    }

    Ok(found)
}

/// 主屏幕元信息（逻辑分辨率 + 缩放比），用于按真实电脑屏幕效果预览
#[derive(serde::Serialize, Clone, Copy)]
pub struct ScreenMeta {
    /// 逻辑宽度（物理像素 ÷ 缩放比）
    pub logical_width: f64,
    /// 逻辑高度
    pub logical_height: f64,
    /// 缩放比（如 1.25 / 1.5 / 2.0）
    pub scale_factor: f64,
}

/// 读取当前主监视器的逻辑分辨率与缩放比
pub fn get_primary_screen_meta(app: &tauri::AppHandle) -> Result<ScreenMeta, String> {
    let mon = app
        .primary_monitor()
        .map_err(|e| format!("读取屏幕信息失败：{e}"))?
        .ok_or_else(|| "未检测到显示器".to_string())?;
    let scale = mon.scale_factor();
    if scale <= 0.0 {
        return Err("屏幕缩放比异常".into());
    }
    let w = mon.size().width as f64 / scale;
    let h = mon.size().height as f64 / scale;
    if w <= 0.0 || h <= 0.0 {
        return Err("屏幕分辨率异常".into());
    }
    Ok(ScreenMeta {
        logical_width: w,
        logical_height: h,
        scale_factor: scale,
    })
}

/// 从本地磁盘永久删除壁纸文件（仅限支持的图片扩展名）
///
/// 安全约束：系统路径下的壁纸一律只读，禁止删除。
pub fn delete_wallpaper_file(path: &str) -> Result<(), String> {
    let p = PathBuf::from(path);
    if is_under_system_dir(&p) {
        return Err("系统壁纸只读，禁止删除系统目录下的文件".into());
    }
    if p.is_dir() {
        return Err("不能删除目录".into());
    }
    if !p.exists() {
        return Err("文件不存在或已被移动".into());
    }

    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if !is_supported_media_ext(&ext) {
        return Err("不支持的壁纸文件类型".into());
    }

    std::fs::remove_file(&p).map_err(|e| format!("删除失败：{e}"))?;
    Ok(())
}
