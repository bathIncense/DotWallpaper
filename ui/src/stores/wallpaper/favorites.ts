// DotWallpaper 壁纸工具 - 收藏夹书签集合状态（stores/wallpaper.ts 拆分子模块）
import { ref } from "vue";
import { FAVORITES_KEY } from "./types";

// 收藏夹书签集合（组合式模块：纯前端 localStorage 持久化，后端零改动；
// loadFavorites 由主 store 暴露给外部调用方在初始化时手动触发）
export function useFavoritesState() {
  const favorites = ref<Set<string>>(new Set());

  function loadFavorites() {
    try {
      const raw = localStorage.getItem(FAVORITES_KEY);
      const arr: unknown = raw ? JSON.parse(raw) : [];
      favorites.value = new Set(
        Array.isArray(arr) ? arr.filter((x): x is string => typeof x === "string") : []
      );
    } catch {
      favorites.value = new Set();
    }
  }

  function persistFavorites() {
    try {
      localStorage.setItem(FAVORITES_KEY, JSON.stringify([...favorites.value]));
    } catch { /* ignore */ }
  }

  function isFavorite(path: string | undefined | null): boolean {
    return !!path && favorites.value.has(path);
  }

  function addFavorite(path: string) {
    favorites.value.add(path);
    persistFavorites();
  }

  function removeFavorite(path: string) {
    favorites.value.delete(path);
    persistFavorites();
  }

  return { favorites, loadFavorites, isFavorite, addFavorite, removeFavorite };
}
