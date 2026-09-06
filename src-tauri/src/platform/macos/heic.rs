// macOS 动态 HEIC 检测与处理
// 使用 ImageIO 检测 HEIC 文件的动态元数据
// 注意：此模块为 P0 原型结构，实际元数据检测需要根据 objc2 API 调整

use std::path::Path;

/// HEIC 动态类型
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HeicDynamicType {
    /// 普通静态 HEIC
    Static,
    /// 基于太阳位置的动态壁纸
    Solar,
    /// 基于 24 小时时间线的动态壁纸
    H24,
    /// 基于外观（亮/暗）的动态壁纸
    Appearance,
    /// 未知类型
    Unknown,
}

impl std::fmt::Display for HeicDynamicType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeicDynamicType::Static => write!(f, "静态"),
            HeicDynamicType::Solar => write!(f, "太阳位置动态"),
            HeicDynamicType::H24 => write!(f, "24小时动态"),
            HeicDynamicType::Appearance => write!(f, "外观动态"),
            HeicDynamicType::Unknown => write!(f, "未知"),
        }
    }
}

/// 检测 HEIC 文件的动态类型
/// 注意：此函数为原型结构，实际实现需要使用 ImageIO 读取 XMP 元数据
pub fn detect_heic_type(path: &str) -> HeicDynamicType {
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return HeicDynamicType::Unknown;
    }

    // TODO: 使用 ImageIO 读取 XMP 元数据
    // 1. 创建 CGImageSource
    // 2. 读取 kCGImagePropertyDictionary
    // 3. 检查 apple_desktop:solar / h24 / apr 键

    HeicDynamicType::Static
}

/// 获取 HEIC 帧数
pub fn get_heic_frame_count(path: &str) -> u32 {
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return 0;
    }

    // TODO: 使用 CGImageSourceGetCount 获取帧数
    1
}

/// 检查文件是否为 HEIC 格式
pub fn is_heic_file(path: &str) -> bool {
    let path_obj = Path::new(path);
    path_obj
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("heic"))
        .unwrap_or(false)
}

/// 检查文件是否为 GIF 格式
pub fn is_gif_file(path: &str) -> bool {
    let path_obj = Path::new(path);
    path_obj
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gif"))
        .unwrap_or(false)
}

/// 检查文件是否为视频格式
pub fn is_video_file(path: &str) -> bool {
    let path_obj = Path::new(path);
    let video_exts = ["mp4", "mov", "m4v", "avi", "mkv", "webm"];
    path_obj
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| video_exts.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}
