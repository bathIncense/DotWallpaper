// DotWallpaper 壁纸工具 - 来源选项卡可见性与顺序状态（stores/wallpaper.ts 拆分子模块）
import { ref } from "vue";
import {
  SOURCE_VIS_KEY,
  SOURCE_ORDER_KEY,
  SOURCE_ORDER_DEFAULT,
  type WallpaperSource,
  type SourceVisibility,
} from "./types";

// 来源选项卡可见性与展示顺序（组合式模块：localStorage 持久化）
export function useSourceState() {
  // 各来源选项卡的可见性：local 默认 true、不可关闭；system 默认 false（可在设置开启）；favorites 默认 true
  const sourceVisibility = ref<SourceVisibility>({ local: true, system: false, favorites: true });

  function restoreSourceVisibility() {
    try {
      const raw = localStorage.getItem(SOURCE_VIS_KEY);
      if (raw) {
        const parsed = JSON.parse(raw) as Partial<SourceVisibility>;
        sourceVisibility.value.system = parsed.system ?? false;
        sourceVisibility.value.favorites = parsed.favorites ?? true;
      }
    } catch { /* ignore */ }
  }

  function persistSourceVisibility() {
    try {
      localStorage.setItem(SOURCE_VIS_KEY, JSON.stringify({
        system: sourceVisibility.value.system,
        favorites: sourceVisibility.value.favorites,
      }));
    } catch { /* ignore */ }
  }

  // 仅更新可见性并持久化；"隐藏当前选项卡时自动切回本地"的联动由主 store 的 setSourceVisibility 处理
  function updateSourceVisibility(key: "system" | "favorites", visible: boolean) {
    sourceVisibility.value[key] = visible;
    persistSourceVisibility();
  }

  // ---- 选项卡展示顺序（持久化；local 恒可见但仍可参与排序） ----
  const sourceOrder = ref<WallpaperSource[]>([...SOURCE_ORDER_DEFAULT]);

  function restoreSourceOrder() {
    try {
      const raw = localStorage.getItem(SOURCE_ORDER_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as unknown;
      if (!Array.isArray(parsed)) return;
      const list = parsed.filter(
        (s): s is WallpaperSource =>
          s === "local" || s === "system" || s === "favorites"
      );
      // 去重并补全缺失来源，保证数组恰好包含全部三个来源
      const seen = new Set<WallpaperSource>(list);
      for (const s of SOURCE_ORDER_DEFAULT) {
        if (!seen.has(s)) {
          list.push(s);
          seen.add(s);
        }
      }
      sourceOrder.value = list;
    } catch { /* ignore */ }
  }

  function persistSourceOrder() {
    try {
      localStorage.setItem(SOURCE_ORDER_KEY, JSON.stringify(sourceOrder.value));
    } catch { /* ignore */ }
  }

  // 将指定来源在展示顺序中上移 / 下移（dir: -1 上移，1 下移）
  function moveSourceOrder(key: WallpaperSource, dir: -1 | 1) {
    const idx = sourceOrder.value.indexOf(key);
    const target = idx + dir;
    if (idx < 0 || target < 0 || target >= sourceOrder.value.length) return;
    const next = [...sourceOrder.value];
    [next[idx], next[target]] = [next[target], next[idx]];
    sourceOrder.value = next;
    persistSourceOrder();
  }

  // 拖拽排序：将指定来源移动到目标来源所在的槽位（就地替换式插入）
  function moveSourceOrderTo(key: WallpaperSource, targetKey: WallpaperSource) {
    const from = sourceOrder.value.indexOf(key);
    const to = sourceOrder.value.indexOf(targetKey);
    if (from < 0 || to < 0 || from === to) return;
    const next = [...sourceOrder.value];
    next.splice(from, 1);
    const insertAt = next.indexOf(targetKey) + (from < to ? 1 : 0);
    next.splice(insertAt, 0, key);
    sourceOrder.value = next;
    persistSourceOrder();
  }

  restoreSourceVisibility();
  restoreSourceOrder();
  return {
    sourceVisibility,
    sourceOrder,
    updateSourceVisibility,
    persistSourceVisibility,
    moveSourceOrder,
    moveSourceOrderTo,
  };
}
