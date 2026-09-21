// DotWallpaper 前端类型定义 —— 与后端（Tauri）命令契约一一对应，字段全部 camelCase

/// 媒体类型
export type MediaKind = "image" | "video";

/// 壁纸填充模式
export type FitMode = "fill" | "fit";

/// 显示器壁纸阶段
export type PlaybackPhase = "static" | "preparing" | "playing" | "paused" | "error";

/// 播放控制动作
export type PlaybackAction = "pause" | "resume" | "stop";

/// 一次壁纸分配（应用到某台显示器）
export interface WallpaperAssignment {
  displayId: string;
  path: string;
  kind: MediaKind;
  fitMode: FitMode;
  muted: boolean;
}

/// 某台显示器的当前壁纸状态
export interface DisplayWallpaperState {
  displayId: string;
  phase: PlaybackPhase;
  assignment: WallpaperAssignment | null;
  error: string | null;
}

/// 显示器信息
export interface DisplayInfo {
  id: string;
  name: string;
  logicalBounds: [number, number, number, number];
  scaleFactor: number;
  primary: boolean;
  mirrored: boolean;
}

/// 媒体库条目：thumb 也是本地文件路径
export interface MediaItem {
  path: string;
  name: string;
  kind: MediaKind;
  thumb: string;
}

/// 首次加载快照
export interface AppSnapshot {
  libraryDir: string;
  displays: DisplayInfo[];
  states: DisplayWallpaperState[];
}

/// 极简设置
export interface Settings {
  libraryDir: string;
  fitMode: FitMode;
  muted: boolean;
}

/// 导入结果
export interface ImportResult {
  saved: string[];
  skipped: string[];
}
