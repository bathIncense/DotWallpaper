// DotWallpaper 壁纸工具 - Pinia 状态仓库（主入口；类型/常量/工具/效果/来源/收藏已拆分至 wallpaper/ 子模块）
import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { confirmDanger, toast } from "../lib/naive-host";
import { useEffectState } from "./wallpaper/effect";
import { useSourceState } from "./wallpaper/sources";
import { useFavoritesState } from "./wallpaper/favorites";
import { baseName, isRemoteSrc } from "./wallpaper/utils";
import {
  BING_DIR_KEY,
  DIR_STORAGE_KEY,
  PAGE_SIZE,
  type WallpaperEntryData,
  type ThumbnailUpdatedPayload,
  type WallpaperItem,
  type WallpaperKind,
  type WallpaperSource,
  type BingWallpaperData,
} from "./wallpaper/types";

// 类型、常量与纯工具函数从子模块统一再导出，调用方 import 路径保持不变
export * from "./wallpaper/types";
export * from "./wallpaper/utils";
export * from "./wallpaper/effect";

// 后台缩略图事件监听全局只注册一次（应用单页生命周期内复用）
let thumbnailListenerRegistered = false;

// ---------- Pinia Store ----------
export const useWallpaperStore = defineStore("wallpaper", () => {
  // ---- 组合式子模块状态 ----
  const { wallpaperEffect, setEffect } = useEffectState();
  const {
    sourceVisibility,
    sourceOrder,
    updateSourceVisibility,
    persistSourceVisibility,
    moveSourceOrder,
    moveSourceOrderTo,
  } = useSourceState();
  const { favorites, loadFavorites, isFavorite, addFavorite, removeFavorite } =
    useFavoritesState();

  // ---- 状态 ----
  const source = ref<WallpaperSource>("local"); // 当前选项卡来源
  const currentDir = ref(""); // 自定义壁纸目录（空 = 预设目录）
  const gridItems = ref<WallpaperItem[]>([]); // 左栏当前列表
  const currentWallpaper = ref<WallpaperItem | null>(null); // 右侧当前壁纸
  const previewItem = ref<WallpaperItem | null>(null); // 正在预览的壁纸（选中态，非当前桌面）
  const desktopStyle = ref<{ style: number; tile: boolean } | null>(null); // 桌面壁纸样式
  const isApplying = ref(false); // 应用壁纸 loading
  // 必应壁纸已下载到本地的路径记录（date → 本地绝对路径）：
  // bing 列表项的 path 是远程 URL，而桌面实际使用的是下载后的本地文件，
  // 靠这份记录才能把"桌面正在显示的壁纸"对应回列表中的必应卡片（绿框）
  const bingLocalPaths = ref<Record<string, string>>({});
  const loadingMore = ref(false);
  const allCount = ref(0);

  // 右键菜单状态
  const ctxVisible = ref(false);
  const ctxX = ref(0);
  const ctxY = ref(0);
  const ctxItem = ref<WallpaperItem | null>(null);
  const ctxReadOnly = ref(false); // 当前右键目标是否为只读来源（系统壁纸禁止删除）

  // 分页私有状态
  let loadedCount = 0;
  let allEntries: WallpaperEntryData[] = [];
  let loadingMoreLock = false;
  // hasMore 必须是响应式普通 ref（与 allCount 等一致，pinia 访问时解包为 boolean）：
  // 若用 computed 包普通 let 变量，let 变化不会让 computed 失效（永远 false）；
  // 若 computed getter 返回 ref，store.hasMore 拿到的又是 Ref 对象而非 boolean，
  // 模板 !store.hasMore 恒为 false，底部“已加载全部”永不显示、fill 补页判断失真。
  const hasMore = ref(false);

  // ---- Getter ----
  // 右侧大预览目标：优先"正在预览"，无预览时回退当前桌面壁纸
  const previewTarget = computed<WallpaperItem | null>(
    () => previewItem.value ?? currentWallpaper.value
  );

  // ---- 右键菜单 ----
  function openContextMenu(item: WallpaperItem, x: number, y: number) {
    ctxItem.value = item;
    ctxX.value = Math.min(x, window.innerWidth - 216);
    ctxY.value = Math.min(y, window.innerHeight - 220);
    ctxVisible.value = true;
    // 系统壁纸 / 收藏夹为只读视图：右键菜单不提供删除入口
    // （收藏夹是书签视图，删除文件入口仍由本地/系统来源提供，避免误删）；
    // 必应壁纸为在线图片，同样无本地文件可删
    ctxReadOnly.value =
      source.value === "system" ||
      source.value === "favorites" ||
      item.kind === "bing";
  }

  function closeContextMenu() {
    ctxVisible.value = false;
    ctxItem.value = null;
    ctxReadOnly.value = false;
  }

  // ---- 后台缩略图渐进更新 ----
  // 后端列表命令不再等待全量缩略图生成：缺失项由后台线程池渐进生成，
  // 每完成一张推送 "thumbnail-updated" { path, thumb }。命中当前列表条目时
  // 仅更新该条 thumb（响应式触发对应 <img> :src 重算加载新图），不整列表刷新；
  // 尚未被懒加载放行的条目无需处理，放行时自然取到最新 thumb。
  async function registerThumbnailListener() {
    if (thumbnailListenerRegistered) return;
    thumbnailListenerRegistered = true;
    await listen<ThumbnailUpdatedPayload>("thumbnail-updated", (ev) => {
      const { path, thumb } = ev.payload;
      if (!path || !thumb) return;
      const item = gridItems.value.find((it) => it.path === path);
      if (item && item.thumb !== thumb) {
        item.thumb = thumb;
      }
    });
  }
  void registerThumbnailListener();

  // ---- 选项卡切换 ----
  async function setSource(next: WallpaperSource) {
    if (source.value === next) return;
    source.value = next;
    gridItems.value = [];
    loadedCount = 0;
    allEntries = [];
    hasMore.value = false;
    allCount.value = 0;
    await loadWallpapers();
  }

  // 设置来源可见性：仅负责"隐藏当前选项卡时自动切回本地"的联动，
  // 可见性更新与持久化由 useSourceState.updateSourceVisibility 完成
  function setSourceVisibility(key: "system" | "favorites" | "bing", visible: boolean) {
    updateSourceVisibility(key, visible);
    if (!visible && source.value === key) {
      void setSource("local");
    }
  }

  // ---- 设置壁纸（核心） ----
  // 必应在线壁纸：先把原图下载到本地缓存目录（同一天重复设置直接复用已下载文件），
  // 再交给 set_wallpaper —— Win32 SPI_SETDESKWALLPAPER 只接受本地文件路径
  async function ensureLocalPath(item: WallpaperItem): Promise<string> {
    if (item.kind !== "bing" || !isRemoteSrc(item.path || "")) return item.path || "";
    // 下载目录由前端 localStorage 提供（未配置则传 null，后端退回默认目录）
    let bingDir = "";
    try { bingDir = localStorage.getItem(BING_DIR_KEY) || ""; } catch { /* ignore */ }
    const local = (await invoke("download_bing_wallpaper", {
      url: item.path || "",
      date: item.date || "",
      dir: bingDir || null,
    })) as string;
    if (item.date && local) {
      bingLocalPaths.value = { ...bingLocalPaths.value, [item.date]: local };
    }
    return local;
  }

  // 从本地缓存文件名反推必应日期（BingWallpaper_YYYYMMDD.jpg）：
  // 应用启动时据此恢复"当前壁纸 ↔ 必应卡片"的绿框对应关系
  function bingDateFromPath(path: string): string {
    const m = /BingWallpaper_(\d{8})\.jpg$/i.exec(path || "");
    return m ? m[1] : "";
  }

  // 某项是否为"当前已设置到桌面的壁纸"
  // （必应项 path 为远程 URL，需经下载记录比对；本地项直接比对路径）
  function isCurrentItem(item: WallpaperItem): boolean {
    const cur = currentWallpaper.value;
    if (!cur?.path) return false;
    if (item.kind === "bing") {
      const local = item.date ? bingLocalPaths.value[item.date] : "";
      return !!local && local === cur.path;
    }
    return !!item.path && item.path === cur.path;
  }

  async function doSetWallpaper(item: WallpaperItem): Promise<{ path: string }> {
    const payload = item.path || "";
    const result = (await invoke("set_wallpaper", {
      path: payload,
      dir: resolveDirArg(),
    })) as { path: string } | string;
    return typeof result === "string" ? { path: result } : result;
  }

  function resolveDirArg(): string | null {
    return currentDir.value && currentDir.value.trim()
      ? currentDir.value.trim()
      : null;
  }

  // 点击卡片：仅选中/预览，不改动桌面（选中态与当前桌面解耦）
  function selectItem(item: WallpaperItem) {
    previewItem.value = item;
  }

  // 读取系统当前壁纸样式（填充/适应等），供预览与"设为壁纸"使用
  async function loadDesktopStyle() {
    try {
      const s = (await invoke("get_wallpaper_style")) as { style: number; tile: boolean };
      desktopStyle.value = {
        style: Number(s.style) || 10,
        tile: Boolean(s.tile),
      };
    } catch (err: unknown) {
      console.error("读取壁纸样式失败:", err);
      desktopStyle.value = null;
    }
  }

  // 将某张壁纸设为桌面壁纸；可选同步应用桌面样式（仅当与系统当前样式不一致时写注册表）
  async function setItemAsDesktop(
    item: WallpaperItem,
    style?: { style: number; tile: boolean }
  ): Promise<boolean> {
    if (isApplying.value) return false;
    const path = item.path || "";
    if (!path) {
      toast("壁纸路径无效", "warning");
      return false;
    }
    isApplying.value = true;
    try {
      // 必应壁纸：先下载原图到本地缓存，再进入效果合成或直接设置
      // （apply_wallpaper_effect / set_wallpaper 均只接受本地文件路径）
      const localPath = await ensureLocalPath(item);
      if (!localPath) {
        toast("壁纸下载失败，无法设置", "error");
        return false;
      }
      // 启用模糊遮罩效果时：后端合成（模糊+遮罩）后设置，样式注册表一并写入
      if (wallpaperEffect.value.enabled) {
        const styleToApply = style ?? desktopStyle.value ?? { style: 10, tile: false };
        const result = (await invoke("apply_wallpaper_effect", {
          path: localPath,
          style: styleToApply.style,
          tile: styleToApply.tile,
          effect: {
            enabled: true,
            blur: wallpaperEffect.value.blur,
            opacity: wallpaperEffect.value.opacity,
            color: wallpaperEffect.value.color,
          },
        })) as { path: string };
        desktopStyle.value = { ...styleToApply };
        currentWallpaper.value = {
          key: "current_" + (result.path || ""),
          kind: "local",
          path: result.path,
          title: item.kind === "bing" ? item.title || undefined : undefined,
        };
        toast("壁纸设置成功", "success");
        return true;
      }

      if (
        style &&
        desktopStyle.value &&
        (style.style !== desktopStyle.value.style || style.tile !== desktopStyle.value.tile)
      ) {
        await invoke("set_desktop_style", { style: style.style, tile: style.tile });
        desktopStyle.value = { ...style };
      }
      const result = await doSetWallpaper({ ...item, path: localPath });
      currentWallpaper.value = {
        key: "current_" + (result.path || ""),
        kind: "local",
        path: result.path,
        // 保留必应中文标题，避免右侧"当前壁纸"只剩 BingWallpaper_20260911.jpg 这类文件名
        title: item.kind === "bing" ? item.title || undefined : undefined,
      };
      toast("壁纸设置成功", "success");
      return true;
    } catch (err: unknown) {
      toast("设置失败：" + ((err as Error)?.message || String(err)), "error");
      return false;
    } finally {
      isApplying.value = false;
    }
  }

  // 将右侧正在预览（无预览时为当前桌面）的壁纸设为桌面（Ctrl+S / 设为壁纸按钮）
  async function applyPreviewAsDesktop(
    style?: { style: number; tile: boolean }
  ): Promise<boolean> {
    const target = previewTarget.value;
    if (!target || !target.path) {
      toast("当前无可预览壁纸", "warning");
      return false;
    }
    return setItemAsDesktop(target, style);
  }

  // ---- 壁纸源加载（分页） ----
  async function loadWallpapers() {
    gridItems.value = [];
    loadedCount = 0;
    allEntries = [];
    try {
      // 后端列表返回：原图路径 + 缩略图路径（缩略图可能为空 → 列表显示占位）
      if (source.value === "system") {
        allEntries = (await invoke("list_system_wallpapers")) as WallpaperEntryData[];
      } else if (source.value === "favorites") {
        // 收藏夹：按收藏路径在本地/系统两个来源中取交集，复用缩略图缓存
        allEntries = await loadFavoriteEntries();
      } else if (source.value === "bing") {
        // 必应每日壁纸：仅拉在线列表（远程原图 URL + 远程缩略图 URL），
        // 列表阶段不下载原图，只有"设为壁纸"时才按需下载到本地
        const list = (await invoke("list_bing_wallpapers")) as BingWallpaperData[];
        allEntries = list.map((b) => ({
          path: b.url,
          thumb: b.thumb,
          title: b.title,
          date: b.date,
          kind: "bing" as WallpaperKind,
        }));
      } else {
        allEntries = (await invoke("list_local_wallpapers", {
          directory: resolveDirArg(),
        })) as WallpaperEntryData[];
      }
      allCount.value = allEntries.length;
      appendBatch();
    } catch (err: unknown) {
      toast("加载壁纸失败：" + ((err as Error)?.message || String(err)), "error");
    }
  }

  function appendBatch() {
    const batch = allEntries.slice(loadedCount, loadedCount + PAGE_SIZE);
    batch.forEach((e) => {
      // 条目可能自带 kind（必应在线壁纸）/ title / date，缺省按本地壁纸处理
      const kind = e.kind ?? "local";
      gridItems.value.push({
        key: `${kind}_${e.path}`,
        kind,
        path: e.path,
        title: e.title || baseName(e.path),
        thumb: e.thumb || undefined,
        date: e.date,
      });
      loadedCount++;
    });
    hasMore.value = loadedCount < allEntries.length;
  }

  async function loadMore() {
    if (loadingMoreLock || !hasMore.value) return;
    loadingMoreLock = true;
    loadingMore.value = true;
    try {
      await new Promise((r) => setTimeout(r, 200)); // 防抖
      appendBatch();
    } finally {
      loadingMoreLock = false;
      loadingMore.value = false;
    }
  }

  // 获取当前桌面壁纸文件（用于右侧展示）
  async function loadCurrentWallpaper() {
    try {
      const path = (await invoke("get_current_wallpaper")) as string;
      if (path) {
        currentWallpaper.value = { key: "current_" + path, kind: "local", path };
        // 桌面壁纸若来自必应缓存目录（BingWallpaper_YYYYMMDD.jpg），登记 date → 本地路径，
        // 保证启动后切到必应来源时对应卡片仍能显示"当前"绿框
        const bingDate = bingDateFromPath(path);
        if (bingDate) {
          bingLocalPaths.value = { ...bingLocalPaths.value, [bingDate]: path };
        }
      } else {
        currentWallpaper.value = null;
      }
    } catch (err: unknown) {
      toast("获取当前壁纸失败：" + ((err as Error)?.message || String(err)), "error");
      currentWallpaper.value = null;
    }
  }

  // ---- 目录持久化与选择 ----
  function restoreDir() {
    try {
      const saved = localStorage.getItem(DIR_STORAGE_KEY);
      if (saved) currentDir.value = saved;
    } catch {
      /* ignore */
    }
  }

  async function pickAndApplyDirectory() {
    try {
      const picked = (await invoke("pick_wallpaper_directory")) as string | null;
      if (picked) {
        currentDir.value = picked;
        try {
          localStorage.setItem(DIR_STORAGE_KEY, picked);
        } catch {
          /* ignore */
        }
        toast("已选择目录：" + picked, "success");
        await loadWallpapers();
      }
    } catch (err: unknown) {
      toast("选择目录失败：" + ((err as Error)?.message || String(err)), "error");
    }
  }

  // ---- 收藏夹 ----
  // 收藏页数据：直接按收藏路径向后端查询，不经过当前壁纸目录，
  // 因此切换壁纸目录后收藏依然完整；已被外部删除的失效路径由后端过滤不展示
  async function loadFavoriteEntries(): Promise<WallpaperEntryData[]> {
    const favPaths = [...favorites.value];
    if (!favPaths.length) return [];
    return (await invoke<WallpaperEntryData[]>("list_wallpapers_by_paths", {
      paths: favPaths,
    })) as WallpaperEntryData[];
  }

  // 从当前已加载列表移除某路径（收藏页取消收藏时即时消失，避免整表重载闪烁）
  function dropFavoriteFromGrid(path: string) {
    allEntries = allEntries.filter((e) => e.path !== path);
    gridItems.value = gridItems.value.filter((it) => it.path !== path);
    loadedCount = gridItems.value.length;
    allCount.value = allEntries.length;
    hasMore.value = loadedCount < allEntries.length;
  }

  // 增删收藏书签；返回是否已收藏（true = 刚加入）
  function toggleFavorite(path: string | undefined | null): boolean {
    if (!path) return false;
    // 在线壁纸（必应）不可收藏：其 path 是远程 URL，收藏夹按本地路径向后端检索，
    // 收藏它只会留下永远命中不了的失效书签
    if (isRemoteSrc(path)) return false;
    const had = isFavorite(path);
    if (had) removeFavorite(path);
    else addFavorite(path);

    // 在收藏夹页取消收藏：立即移除卡片并清空对应预览，避免幽灵项
    if (had && source.value === "favorites") {
      if (previewItem.value?.path === path) previewItem.value = null;
      dropFavoriteFromGrid(path);
    }
    return !had;
  }

  // ---- 外部拖入保存 ----
  // 将 Tauri 原生拖放事件给出的本地文件路径复制到壁纸目录并刷新列表
  async function saveDroppedPaths(paths: string[]) {
    if (!paths.length) return;
    try {
      const res = (await invoke("save_dropped_paths", {
        paths,
        dir: resolveDirArg(),
      })) as { saved: string[]; skipped: string[] };

      if (res.saved.length) {
        toast(
          `已保存 ${res.saved.length} 张壁纸` +
            (res.skipped.length ? `，跳过 ${res.skipped.length} 个` : ""),
          "success"
        );
        await loadWallpapers();
      } else if (res.skipped.length) {
        toast("没有可保存的图片：" + res.skipped[0], "warning");
      }
    } catch (err: unknown) {
      toast("保存失败：" + ((err as Error)?.message || String(err)), "error");
    }
  }

  // ---- 右键菜单动作 ----
  async function handleContextAction(action: string, item: WallpaperItem | null) {
    closeContextMenu();
    if (!item) return;

    if (action === "set-wallpaper") {
      // 设置桌面不改变"正在预览"的选中态：蓝（预览）与绿（桌面）独立
      await setItemAsDesktop(item);
      return;
    }

    if (action === "toggle-favorite") {
      // 收藏夹以本地文件路径为书签，必应在线壁纸（远程 URL）不适用
      if (item.kind === "bing") {
        toast("必应在线壁纸暂不支持收藏", "warning");
        return;
      }
      // 收藏 / 取消收藏：仅书签标记，不删文件
      const fav = toggleFavorite(item.path);
      if (fav) toast("已收藏：" + baseName(item.path || ""), "success");
      else toast("已取消收藏", "warning");
      return;
    }

    if (action === "reveal-folder") {
      // 在系统资源管理器中打开文件所在目录并定位（本地/系统壁纸通用）；
      // 必应壁纸需先"设为桌面"下载到本地后才能定位，未下载时给引导提示
      const localPath =
        item.kind === "bing"
          ? (item.date && bingLocalPaths.value[item.date]) || ""
          : item.path || "";
      if (!localPath) {
        toast(
          item.kind === "bing"
            ? "必应壁纸为在线图片，设为桌面后会下载到本地"
            : "当前壁纸无本地路径",
          "warning"
        );
        return;
      }
      try {
        await invoke("reveal_in_explorer", { path: localPath });
      } catch (err: unknown) {
        toast("打开目录失败：" + ((err as Error)?.message || String(err)), "error");
      }
      return;
    }

    if (action === "delete-wallpaper" && item.kind !== "current") {
      // 系统壁纸来源只读：纵深防御，禁止进入删除流程；
      // 必应壁纸是远程图片、本地无对应文件，同样禁止删除
      if (source.value === "system" || item.kind === "bing") return;
      await removeWallpaper(item);
    }
  }

  // 右键删除：本地磁盘永久删除壁纸文件
  async function removeWallpaper(item: WallpaperItem): Promise<void> {
    if (!item.path) return;
    // 只读保护：系统壁纸来源不允许删除（同时后端 delete_wallpaper 亦拦截系统路径）
    if (source.value === "system") return;
    const name = baseName(item.path);

    const confirmed = await confirmDanger({
      title: "删除壁纸",
      content: `确定要删除壁纸「${name}」吗？\n该文件会从本地磁盘永久删除，无法恢复。`,
      positiveText: "删除",
    });
    if (!confirmed) return;

    try {
      await invoke("delete_wallpaper", { path: item.path });
      // 文件已物理删除：同步清理收藏书签，避免收藏夹出现失效路径
      if (isFavorite(item.path)) removeFavorite(item.path);
      toast("已删除壁纸：" + name, "success");
      await loadWallpapers();
      // 删除的正是当前桌面壁纸：同步刷新桌面真值
      if (currentWallpaper.value?.path === item.path) {
        await loadCurrentWallpaper();
      }
      // 删除的正是正在预览的壁纸：清空选中态，大预览回退到当前桌面
      if (previewItem.value?.path === item.path) {
        previewItem.value = null;
      }
    } catch (err: unknown) {
      toast("删除失败：" + ((err as Error)?.message || String(err)), "error");
    }
  }

  return {
    // state
    source,
    currentDir,
    gridItems,
    currentWallpaper,
    previewItem,
    desktopStyle,
    isApplying,
    loadingMore,
    allCount,
    wallpaperEffect,
    ctxVisible,
    ctxX,
    ctxY,
    ctxItem,
    ctxReadOnly,
    sourceVisibility,
    sourceOrder,
    // getters
    hasMore,
    previewTarget,
    isCurrentItem,
    // actions
    setSource,
    openContextMenu,
    closeContextMenu,
    selectItem,
    loadFavorites,
    isFavorite,
    toggleFavorite,
    setItemAsDesktop,
    applyPreviewAsDesktop,
    loadDesktopStyle,
    loadWallpapers,
    loadMore,
    loadCurrentWallpaper,
    restoreDir,
    pickAndApplyDirectory,
    saveDroppedPaths,
    handleContextAction,
    removeWallpaper,
    setSourceVisibility,
    persistSourceVisibility,
    setEffect,
    moveSourceOrder,
    moveSourceOrderTo,
  };
});
