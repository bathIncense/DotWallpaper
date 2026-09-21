// 缩略图管线：使用 macOS ImageIO（见 cfmedia.rs），不引入 Rust 编解码库。
// - 缓存目录：app_cache_dir/thumbnails
// - 命名：原图绝对路径 FNV-1a 64 稳定哈希，跨进程可复用
// - 图片：CGImageSourceCreateThumbnailAtIndex -> JPEG q85
// - 视频：主线程 AVAssetImageGenerator 抽取首帧海报（posters/），再 ImageIO 缩放
// - 列表命令仅等待首屏窗口，其余后台渐进生成

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

use crate::cfmedia;
use crate::engine;
use crate::runtime;
use crate::types::{MediaItem, MediaKind};

const THUMB_SIZE: u32 = 256;
const THUMB_EXT: &str = "jpg";
const PREFETCH_N: usize = 36;
const PREFETCH_BUDGET_MS: u64 = 1500;
const MAX_WORKERS: usize = 6;

fn fnv1a64(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub fn hashed_file_name(src: &str, ext: &str) -> String {
    format!("{:016x}.{}", fnv1a64(src), ext)
}

fn app_path(join: fn(PathBuf) -> PathBuf) -> Result<PathBuf, String> {
    let app = runtime::app()?;
    let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    Ok(join(dir))
}

pub fn cache_dir() -> Result<PathBuf, String> {
    app_path(|d| d.join("thumbnails"))
}

pub fn poster_dir() -> PathBuf {
    app_path(|d| d.join("posters")).unwrap_or_else(|_| std::env::temp_dir())
}

fn cached_thumb(src: &str, cache: &Path) -> Option<String> {
    let p = cache.join(hashed_file_name(src, THUMB_EXT));
    p.is_file().then(|| p.to_string_lossy().to_string())
}

fn ensure_poster(src: &str) -> Option<String> {
    let poster = engine::poster_path(src);
    if poster.is_file() {
        return Some(poster.to_string_lossy().to_string());
    }
    engine::generate_poster(src, &poster).ok()?;
    Some(poster.to_string_lossy().to_string())
}

/// 为单个媒体生成（或复用）缩略图。视频走海报两级缩放。
fn ensure_thumb(src: &str, kind: MediaKind, cache: &Path) -> Option<String> {
    if let Some(t) = cached_thumb(src, cache) {
        return Some(t);
    }
    let _ = std::fs::create_dir_all(cache);
    let source_image = match kind {
        MediaKind::Image => {
            // 小图直接以原图作为缩略图，避免多余落盘
            if let Some((w, h)) = cfmedia::image_size(src) {
                if w.max(h) <= THUMB_SIZE as usize {
                    return Some(src.to_string());
                }
            }
            src.to_string()
        }
        MediaKind::Video => ensure_poster(src)?,
    };
    let target = cache.join(hashed_file_name(src, THUMB_EXT));
    cfmedia::make_thumbnail_jpeg(&source_image, &target.to_string_lossy(), THUMB_SIZE).ok()?;
    Some(target.to_string_lossy().to_string())
}

fn inflight_set() -> &'static Mutex<HashSet<String>> {
    static INFLIGHT: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    INFLIGHT.get_or_init(|| Mutex::new(HashSet::new()))
}

/// 构建媒体列表条目：缓存命中即用，缺失项在预算内生成首屏，其余交后台线程。
pub fn make_entries(items: Vec<(String, MediaKind)>) -> Vec<MediaItem> {
    let cache = cache_dir().unwrap_or_default();
    if items.is_empty() {
        return Vec::new();
    }
    let mut thumbs: HashMap<String, String> = HashMap::new();
    let mut missing: Vec<(String, MediaKind)> = Vec::new();
    if !cache.as_os_str().is_empty() {
        for (path, kind) in &items {
            match cached_thumb(path, &cache) {
                Some(t) => {
                    thumbs.insert(path.clone(), t);
                }
                None => missing.push((path.clone(), *kind)),
            }
        }
    } else {
        missing = items.clone();
    }

    let mut batch: Vec<(String, MediaKind)> = Vec::new();
    {
        let mut inflight = inflight_set().lock().unwrap();
        for (p, k) in missing {
            if inflight.insert(p.clone()) {
                batch.push((p, k));
            }
        }
    }

    if !batch.is_empty() {
        let results: Arc<Mutex<HashMap<String, Option<String>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let queue: Arc<Mutex<std::collections::VecDeque<(String, MediaKind)>>> =
            Arc::new(Mutex::new(batch.iter().cloned().collect()));
        let concurrency = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(1, MAX_WORKERS);
        let cache_for_thread = cache.clone();
        for _ in 0..concurrency {
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results);
            let cache_t = cache_for_thread.clone();
            std::thread::spawn(move || loop {
                let next = queue.lock().unwrap().pop_front();
                let Some((src, kind)) = next else { break };
                let thumb = ensure_thumb(&src, kind, &cache_t);
                if let Some(t) = &thumb {
                    if let Ok(app) = runtime::app() {
                        let _ = app.emit(
                            "thumbnail-ready",
                            serde_json::json!({ "path": src, "thumb": t }),
                        );
                    }
                }
                results.lock().unwrap().insert(src.clone(), thumb);
                inflight_set().lock().unwrap().remove(&src);
            });
        }
        // 首屏预算等待
        let prefetch: HashSet<String> =
            batch.iter().take(PREFETCH_N).map(|(p, _)| p.clone()).collect();
        let deadline = Instant::now() + Duration::from_millis(PREFETCH_BUDGET_MS);
        loop {
            let done = {
                let r = results.lock().unwrap();
                prefetch.iter().all(|p| r.contains_key(p))
            };
            if done || Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(60));
        }
        let r = results.lock().unwrap();
        for (p, t) in r.iter() {
            if let Some(t) = t {
                thumbs.insert(p.clone(), t.clone());
            }
        }
    }

    items
        .into_iter()
        .map(|(path, kind)| {
            let name = Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            MediaItem {
                thumb: thumbs.get(&path).cloned().unwrap_or_default(),
                path,
                name,
                kind,
            }
        })
        .collect()
}

pub fn delete_thumb(src: &str, cache: &Path) {
    if !cache.is_dir() {
        return;
    }
    let p = cache.join(hashed_file_name(src, THUMB_EXT));
    if p.is_file() {
        let _ = std::fs::remove_file(p);
    }
    let poster = engine::poster_path(src);
    if poster.is_file() {
        let _ = std::fs::remove_file(poster);
    }
}
