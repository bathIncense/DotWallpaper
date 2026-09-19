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

// 是否为远程 URL（在线壁纸如必应）；远程路径不做本地 convertFileSrc
export function isRemoteSrc(path: string): boolean {
  return /^https?:\/\//i.test(path);
}

// 图片能用于展示的地址：本地路径走 convertFileSrc；远程 URL（在线壁纸如必应）原样返回
export function displaySrc(item: WallpaperItem): string {
  if (!item.path) return "";
  return isRemoteSrc(item.path) ? item.path : convertFileSrc(item.path);
}

// 缩略图地址：本地路径走 convertFileSrc；远程 URL 原样返回
export function thumbSrc(item: WallpaperItem): string {
  if (!item.thumb) return "";
  return isRemoteSrc(item.thumb) ? item.thumb : convertFileSrc(item.thumb);
}

// 徽标文案（当前仅本地）
export function kindBadgeText(kind: WallpaperKind): string {
  return kind === "local" ? "本地" : "";
}
