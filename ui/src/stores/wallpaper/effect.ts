// DotWallpaper 壁纸工具 - 壁纸模糊遮罩效果状态（stores/wallpaper.ts 拆分子模块）
import { ref } from "vue";
import { EFFECT_STORAGE_KEY, type WallpaperEffect } from "./types";

export const DEFAULT_EFFECT: WallpaperEffect = {
  enabled: false,
  blur: 12,
  opacity: 40,
  color: "#000000",
};

// 校验 #RRGGBB（前端兜底，非法回退默认）
export function normalizeEffectColor(c: string): string {
  return /^#[0-9a-fA-F]{6}$/.test(c) ? c.toLowerCase() : DEFAULT_EFFECT.color;
}

// 壁纸模糊遮罩效果状态（组合式模块：localStorage 持久化；预览与设为壁纸共用）
export function useEffectState() {
  const wallpaperEffect = ref<WallpaperEffect>({ ...DEFAULT_EFFECT });

  function restoreEffect() {
    try {
      const raw = localStorage.getItem(EFFECT_STORAGE_KEY);
      if (raw) {
        const p = JSON.parse(raw) as Partial<WallpaperEffect>;
        wallpaperEffect.value = {
          enabled: Boolean(p.enabled),
          blur: Math.min(30, Math.max(0, Number(p.blur) || 0)),
          opacity: Math.min(80, Math.max(0, Number(p.opacity) || 0)),
          color: normalizeEffectColor(
            typeof p.color === "string" ? p.color : DEFAULT_EFFECT.color
          ),
        };
      }
    } catch { /* ignore */ }
  }

  function persistEffect() {
    try {
      localStorage.setItem(EFFECT_STORAGE_KEY, JSON.stringify(wallpaperEffect.value));
    } catch { /* ignore */ }
  }

  // 更新效果参数并立即持久化（EffectPanel 滑块/开关调用）
  function setEffect(patch: Partial<WallpaperEffect>) {
    const next = { ...wallpaperEffect.value, ...patch };
    if (typeof next.color === "string") next.color = normalizeEffectColor(next.color);
    wallpaperEffect.value = next;
    persistEffect();
  }

  restoreEffect();
  return { wallpaperEffect, setEffect };
}
