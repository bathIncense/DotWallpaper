// DotWallpaper 壁纸工具 - 纯工具函数（stores/wallpaper.ts 拆分子模块）
import { convertFileSrc } from "@tauri-apps/api/core";
import type { WallpaperItem, WallpaperKind } from "./types";

export function shuffle<T>(arr: T[]): T[] {
  const a = [...arr];
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

export function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1];
}

// 图片能用于展示的地址：本地路径走 convertFileSrc
export function displaySrc(item: WallpaperItem): string {
  return item.path ? convertFileSrc(item.path) : "";
}

// 列表缩略图 URL：基于后端生成的缩略图缓存路径（无缩略图时返回空串，
// 由调用方显示占位；不回退原图，保证大图不进入列表加载链路）
export function thumbSrc(item: WallpaperItem): string {
  return item.thumb ? convertFileSrc(item.thumb) : "";
}

// 徽标文案（当前仅本地）
export function kindBadgeText(kind: WallpaperKind): string {
  return kind === "local" ? "本地" : "";
}
