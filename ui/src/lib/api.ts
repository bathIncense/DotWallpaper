// Tauri 命令封装：前端唯一的后端调用出口
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type {
  AppSnapshot,
  DisplayInfo,
  DisplayWallpaperState,
  ImportResult,
  MediaItem,
  PlaybackAction,
  Settings,
  WallpaperAssignment,
} from "../types";

export const api = {
  getAppSnapshot: () => invoke<AppSnapshot>("get_app_snapshot"),
  listMedia: () => invoke<MediaItem[]>("list_media"),
  listDisplays: () => invoke<DisplayInfo[]>("list_displays"),
  applyWallpaper: (assignment: WallpaperAssignment) =>
    invoke<DisplayWallpaperState>("apply_wallpaper", { assignment }),
  controlPlayback: (displayId: string, action: PlaybackAction) =>
    invoke<DisplayWallpaperState>("control_playback", { displayId, action }),
  pickLibraryDirectory: () => invoke<string | null>("pick_library_directory"),
  pickMediaFiles: () => invoke<string[]>("pick_media_files"),
  importMedia: (paths: string[]) =>
    invoke<ImportResult>("import_media", { paths }),
  deleteMedia: (path: string) => invoke<null>("delete_media", { path }),
  updateSettings: (settings: Settings) =>
    invoke<null>("update_settings", { settings }),
};

/// 本地绝对路径 → WebView 可加载的资源地址
export function localSrc(path?: string | null): string {
  return path ? convertFileSrc(path) : "";
}

/// 统一提取错误文案
export function errMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}
