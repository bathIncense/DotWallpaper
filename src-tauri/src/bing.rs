// 必应每日壁纸后端实现：
// - fetch_wallpapers：调用 Bing HPImageArchive 接口拉取每日壁纸列表（idx 0 / 8 两批，共 16 天）
// - download_wallpaper：设为桌面壁纸时把原图下载到本地缓存目录，返回本地路径
//
// 列表阶段不下载原图：前端直接用远程 URL 展示（列表用 400x240 小图变体，
// 仅约 30KB/张），只有点击"设为壁纸"才拉取 1920x1080 原图 —— 既省流量也省磁盘。

use serde::Serialize;
use std::path::{Path, PathBuf};
use crate::resolve_save_dir;

/// 必应壁纸归档接口（n 上限 8，通过 idx 分批取更多历史）
const API_TMPL: &str = "https://www.bing.com/HPImageArchive.aspx?format=js&idx={idx}&n=8&mkt=zh-CN";
/// 图片资源站前缀（接口返回的 url 为 /th?id=... 相对路径）
const ORIGIN: &str = "https://www.bing.com";

/// 列表条目：前端直接用 url / thumb 远程展示
#[derive(Serialize, Clone)]
pub struct BingWallpaper {
    /// 日期标识 YYYYMMDD（必应每日一张，同时作为本地缓存文件名）
    pub date: String,
    /// 图片标题（中文简述）
    pub title: String,
    /// 版权说明（含作者）
    pub copyright: String,
    /// 1920x1080 原图 URL（设为桌面时下载）
    pub url: String,
    /// 400x240 缩略图 URL（列表展示用）
    pub thumb: String,
}

/// 统一的阻塞式 GET（复用连接池，带超时与 UA，避免被必应拦截）
fn http_get(url: &str) -> Result<reqwest::blocking::Response, String> {
    static CLIENT: std::sync::OnceLock<reqwest::blocking::Client> = std::sync::OnceLock::new();
    let client = CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            // 默认 UA 会被必应返回 403，伪装成浏览器 UA
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
            )
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    });
    client
        .get(url)
        .send()
        .map_err(|e| format!("请求必应壁纸接口失败：{e}"))
}

/// 把 1920x1080 原图 URL 改写为 400x240 缩略图 URL（失败时回退原 URL）
///
/// 必应 th?id 接口支持同一 id 的多档分辨率（尺寸体现在文件名中），
/// 因此直接改写尺寸段即可拿到小图，无需服务端参数。
fn to_thumb_url(full: &str) -> String {
    let replaced = full
        .replace("_1920x1080.jpg", "_400x240.jpg")
        .replace("_1920x1200.jpg", "_400x240.jpg")
        .replace("_UHD.jpg", "_400x240.jpg");
    if replaced == full {
        full.to_string()
    } else {
        replaced
    }
}

/// 拉取单批壁纸（idx 为起始偏移）
fn fetch_batch(idx: usize) -> Result<Vec<BingWallpaper>, String> {
    let url = API_TMPL.replace("{idx}", &idx.to_string());
    let resp = http_get(&url)?;
    if !resp.status().is_success() {
        return Err(format!("必应壁纸接口返回 HTTP {}", resp.status()));
    }
    let text = resp
        .text()
        .map_err(|e| format!("读取必应壁纸响应失败：{e}"))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("解析必应壁纸响应失败：{e}"))?;

    let mut list: Vec<BingWallpaper> = Vec::new();
    let Some(images) = json.get("images").and_then(|v| v.as_array()) else {
        return Ok(list);
    };
    for img in images {
        let raw_url = img.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let date = img.get("enddate").and_then(|v| v.as_str()).unwrap_or("");
        if raw_url.is_empty() || date.is_empty() {
            continue;
        }
        let full = if raw_url.starts_with("http") {
            raw_url.to_string()
        } else {
            format!("{ORIGIN}{raw_url}")
        };
        let title = img
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("必应壁纸")
            .trim()
            .to_string();
        let copyright = img
            .get("copyright")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        list.push(BingWallpaper {
            date: date.to_string(),
            title,
            copyright,
            thumb: to_thumb_url(&full),
            url: full,
        });
    }
    Ok(list)
}

/// 拉取必应每日壁纸列表（idx 0 + idx 8 两批；第二批失败不影响第一批结果）
pub fn fetch_wallpapers() -> Result<Vec<BingWallpaper>, String> {
    let mut out: Vec<BingWallpaper> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut first_err: Option<String> = None;

    for idx in [0usize, 8] {
        match fetch_batch(idx) {
            Ok(list) => {
                for w in list {
                    if seen.insert(w.date.clone()) {
                        out.push(w);
                    }
                }
            }
            // 首批失败说明网络/接口不可用，需向上报错；后续批次失败则静默忽略
            Err(e) => {
                if idx == 0 {
                    first_err = Some(e);
                }
            }
        }
    }

    if out.is_empty() {
        return Err(first_err.unwrap_or_else(|| "未获取到必应壁纸".into()));
    }
    Ok(out)
}

/// 把必应原图下载到 `dir` 并返回本地绝对路径。
///
/// 如果 `dir` 为空则默认使用 `图片目录\\BingWallpaper`；同一天重复点击"设为壁纸"时
/// 直接复用已下载文件，不重复走网络。先写 `.part` 再改名，避免半截文件
/// 被后续流程当成完整图片使用。
pub fn download_wallpaper(url: &str, date: &str, dir: &Path) -> Result<String, String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("壁纸地址无效".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("创建下载目录失败 {}: {e}", dir.display()))?;

    let safe_date = if !date.is_empty() && date.chars().all(|c| c.is_ascii_digit()) {
        date.to_string()
    } else {
        "latest".to_string()
    };
    let dest: PathBuf = dir.join(format!("BingWallpaper_{safe_date}.jpg"));

    // 已下载且非空：直接复用（同一天多次设置壁纸不重复下载）
    if dest.is_file() && std::fs::metadata(&dest).map(|m| m.len() > 0).unwrap_or(false) {
        return Ok(normalize(&dest));
    }

    let resp = http_get(url)?;
    if !resp.status().is_success() {
        return Err(format!("下载必应壁纸失败：HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .map_err(|e| format!("读取必应壁纸数据失败：{e}"))?;
    // 正常原图数百 KB，小于 4KB 视为异常响应（错误页/占位）
    if bytes.len() < 4096 {
        return Err("下载的图片数据异常（文件过小）".into());
    }

    let tmp = dir.join(format!("BingWallpaper_{safe_date}.jpg.part"));
    std::fs::write(&tmp, &bytes).map_err(|e| format!("写入壁纸文件失败：{e}"))?;
    std::fs::rename(&tmp, &dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("保存壁纸文件失败：{e}")
    })?;

    Ok(normalize(&dest))
}

/// 路径统一为 Windows 反斜杠风格（与项目其它返回保持一致）
fn normalize(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\")
}

/// 返回必应壁纸下载目录的默认路径（`图片目录\\BingWallpaper`）
pub fn default_bing_dir() -> Result<String, String> {
    let pics = resolve_save_dir(None);
    let bing = pics.join("BingWallpaper");
    Ok(normalize(&bing))
}
