// 改造计划 §4 自动化测试（可离线运行的部分）。
// 依赖真实屏幕/播放器的用例（UUID 屏幕映射、会话状态迁移、真机验收）不在单测范围。

#[cfg(test)]
mod tests {
    use crate::media;
    use crate::settings::{self, AppSettings};
    use crate::types::{
        DisplayWallpaperState, FitMode, MediaKind, Phase, WallpaperAssignment,
    };
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// settings 为进程级全局，触及其测试需串行执行
    fn serialize() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dotwallpaper-test-{tag}-{:?}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn media_extension_classification() {
        for ext in ["jpg", "JPG", "jpeg", "png", "bmp", "webp", "heic"] {
            assert_eq!(media::kind_of_ext(ext), Some(MediaKind::Image), "{ext}");
        }
        for ext in ["mp4", "MOV", "mov"] {
            assert_eq!(media::kind_of_ext(ext), Some(MediaKind::Video), "{ext}");
        }
        // 首版明确拒绝：GIF、网页、m4v 等
        for ext in ["gif", "html", "m4v", "txt", ""] {
            assert_eq!(media::kind_of_ext(ext), None, "{ext}");
        }
    }

    #[test]
    fn apply_validation_enforces_kind_and_boundary() {
        let _guard = serialize();
        let lib = scratch_dir("lib");
        let inside = lib.join("photo.JPG");
        std::fs::write(&inside, b"fake").unwrap();
        let video = lib.join("clip.mp4");
        std::fs::write(&video, b"fake").unwrap();
        let gif = lib.join("anim.gif");
        std::fs::write(&gif, b"fake").unwrap();

        settings::init_for_test(PathBuf::from("unused.json"), AppSettings {
            library_dir: lib.to_string_lossy().to_string(),
            ..Default::default()
        });

        let ok = media::validate_for_apply(&inside.to_string_lossy(), MediaKind::Image);
        assert!(ok.is_ok(), "{ok:?}");
        // 声明类型与实际扩展名不符
        assert!(media::validate_for_apply(&inside.to_string_lossy(), MediaKind::Video).is_err());
        assert!(media::validate_for_apply(&video.to_string_lossy(), MediaKind::Image).is_err());
        // GIF 拒绝；不存在的路径拒绝；目录外文件（系统文件）拒绝
        assert!(media::validate_for_apply(&gif.to_string_lossy(), MediaKind::Image).is_err());
        assert!(media::validate_for_apply("/no/such.png", MediaKind::Image).is_err());
        assert!(media::validate_for_apply("/etc/hosts", MediaKind::Image).is_err());
    }

    #[test]
    fn delete_only_within_library_dir() {
        let _guard = serialize();
        let lib = scratch_dir("del");
        let inside = lib.join("a.png");
        std::fs::write(&inside, b"x").unwrap();
        settings::init_for_test(PathBuf::from("unused.json"), AppSettings {
            library_dir: lib.to_string_lossy().to_string(),
            ..Default::default()
        });
        assert!(media::delete("/etc/hosts").is_err());
        assert!(media::delete(&format!("{}/../escape.png", lib.display())).is_err());
        assert!(media::delete(&inside.to_string_lossy()).is_ok());
        assert!(!inside.exists());
    }

    #[test]
    fn settings_roundtrip_corruption_and_version() {
        let _guard = serialize();
        let dir = scratch_dir("settings");
        let path = dir.join("settings.json");
        settings::init_for_test(
            path.clone(),
            AppSettings {
                library_dir: dir.to_string_lossy().to_string(),
                ..Default::default()
            },
        );
        let a = WallpaperAssignment {
            display_id: "builtin".into(),
            path: "/tmp/x.jpg".into(),
            kind: MediaKind::Image,
            fit_mode: FitMode::Fit,
            muted: false,
        };
        settings::record_assignment(&a);
        let reloaded = settings::load_from(&path);
        assert_eq!(reloaded.assignments.get("builtin"), Some(&a));

        // 损坏配置：回退默认
        std::fs::write(&path, b"{ not json").unwrap();
        let fallback = settings::load_from(&path);
        assert_eq!(fallback, AppSettings::default());
        assert!(path.with_extension("json.bak").exists());

        // 未知版本：回退默认
        std::fs::write(
            &path,
            br#"{"version":999,"libraryDir":"/x","defaultFitMode":"fill","defaultMuted":true,"assignments":{},"pausedDisplays":[]}"#,
        )
        .unwrap();
        assert_eq!(settings::load_from(&path), AppSettings::default());
    }

    #[test]
    fn wire_format_field_consistency() {
        // 前端契约：camelCase 字段 + snake_case 枚举值
        let a = WallpaperAssignment {
            display_id: "d1".into(),
            path: "/m.mp4".into(),
            kind: MediaKind::Video,
            fit_mode: FitMode::Fill,
            muted: true,
        };
        let json = serde_json::to_value(&a).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "displayId": "d1",
                "path": "/m.mp4",
                "kind": "video",
                "fitMode": "fill",
                "muted": true,
            })
        );
        let state = DisplayWallpaperState {
            display_id: "d1".into(),
            phase: Phase::Paused,
            assignment: Some(a),
            error: None,
        };
        let text = serde_json::to_string(&state).unwrap();
        let back: DisplayWallpaperState = serde_json::from_str(&text).unwrap();
        assert_eq!(back.phase, Phase::Paused);
        assert!(text.contains("\"phase\":\"paused\""));
        // 非法阶段必须解析失败（前后端枚举一致性护栏）
        assert!(serde_json::from_str::<DisplayWallpaperState>(
            r#"{"displayId":"d1","phase":"bogus","assignment":null,"error":null}"#
        )
        .is_err());
    }
}
