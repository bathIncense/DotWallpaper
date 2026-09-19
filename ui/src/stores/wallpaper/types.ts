// DotWallpaper 壁纸工具 - 共享类型与常量（stores/wallpaper.ts 拆分子模块）
export type WallpaperKind = "local" | "current" | "bing";

/// 壁纸来源选项卡：local = 本地壁纸（可增删），system = Windows 自带系统壁纸（只读），
/// favorites = 收藏夹（书签视图：跨本地/系统来源，仅标记不删文件），
/// bing = 必应在线壁纸（列表远程 URL，设为壁纸时才下载本地）
export type WallpaperSource = "local" | "system" | "favorites" | "bing";

/// 各来源选项卡的可见性：local 默认强制开启、不可关闭
export const SOURCE_VIS_KEY = "dot-wallpaper-source-visibility";
export type SourceVisibility = {
  local: true;
  system: boolean;
  favorites: boolean;
  bing: boolean;
};
// 选项卡展示顺序（独立于可见性：顺序控制渲染次序，可见性控制是否显示）
export const SOURCE_ORDER_KEY = "dot-wallpaper-source-order";
export const SOURCE_ORDER_DEFAULT: WallpaperSource[] = [
  "local",
  "favorites",
  "system",
  "bing",
];

export interface WallpaperItem {
  key: string;
  kind: WallpaperKind; // 类型：本地 / 当前壁纸（右键目标）
  path?: string; // 本地绝对路径（原图）或必应远程 URL
  title?: string;
  thumb?: string; // 缩略图绝对路径（列表加载用；无缩略图时为空）
  date?: string; // 必应壁纸日期（YYYYMMDD），作为下载缓存与卡片对应键
  applying?: boolean; // 是否正在设置中
}

/// 后端列表命令返回条目：原图路径 + 缩略图路径（可能为空）
/// 必应在线壁纸列表额外携带 kind/title/date（其余来源可缺省）
export interface WallpaperEntryData {
  path: string;
  thumb: string;
  kind?: WallpaperKind; // 来源类型（必应在线壁纸为 "bing"；缺省按本地处理）
  title?: string; // 壁纸标题（必应在线壁纸自带；缺省用文件名）
  date?: string; // 必应壁纸日期（YYYYMMDD）
}

/// 后端后台缩略图完成事件 payload：原图路径 + 缩略图路径
export interface ThumbnailUpdatedPayload {
  path: string;
  thumb: string;
}

// ---------- 常量 ----------
export const DIR_STORAGE_KEY = "dot-wallpaper-dir"; // localStorage 持久化键
export const BING_DIR_KEY = "dot-wallpaper-bing-dir"; // localStorage 必应壁纸目录键
export const FAVORITES_KEY = "dot-wallpaper-favorites"; // localStorage 收藏书签集合键
export const EFFECT_STORAGE_KEY = "dot-wallpaper-effect"; // localStorage 壁纸效果键
export const PAGE_SIZE = 12; // 每页加载张数

/// 必应在线壁纸列表条目（对应后端 bing.rs BingWallpaper）
export interface BingWallpaperData {
  url: string; // 远程原图完整 URL（https://www.bing.com/...）
  thumb: string; // 远程缩略图 URL（全尺寸 URL 追加 &w=... 参数）
  title: string; // 壁纸标题
  date: string; // 日期（YYYYMMDD），作为下载缓存文件名与卡片对应键
}

/// 壁纸模糊遮罩效果参数（与后端 wallpaper::WallpaperEffect 对齐）
export interface WallpaperEffect {
  enabled: boolean; // 是否启用效果
  blur: number; // 高斯模糊强度（0~30）
  opacity: number; // 遮罩不透明度百分比（0~80）
  color: string; // 遮罩颜色 #RRGGBB（默认黑色 #000000）
}
