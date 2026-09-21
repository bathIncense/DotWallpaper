// useApp —— 全局状态单例（替代原 pinia store）
// 模块级 ref/computed + 导出单例：任何组件调用 useApp() 都拿到同一份状态。
import { computed, ref } from "vue";
import { api, errMessage, localSrc } from "../lib/api";
import type {
  DisplayInfo,
  DisplayWallpaperState,
  FitMode,
  MediaItem,
  PlaybackAction,
  PlaybackPhase,
} from "../types";

// ---------- 提示 ----------
export type ToastKind = "info" | "success" | "error" | "warning";
export interface Toast {
  id: number;
  text: string;
  kind: ToastKind;
}

// ---------- 单例状态 ----------
const libraryDir = ref("");
const media = ref<MediaItem[]>([]);
const displays = ref<DisplayInfo[]>([]);
const states = ref<DisplayWallpaperState[]>([]);
const selectedPath = ref("");
const selectedDisplayId = ref("");
const fitMode = ref<FitMode>("fill");
const muted = ref(true);
const loadingMedia = ref(false);
const applying = ref(false);
const dragging = ref(false);
const toasts = ref<Toast[]>([]);
let toastSeq = 0;

// ---------- 私有工具 ----------
function upsertState(next: DisplayWallpaperState) {
  const idx = states.value.findIndex((s) => s.displayId === next.displayId);
  if (idx >= 0) states.value.splice(idx, 1, next);
  else states.value.push(next);
}

function dismiss(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id);
}

function toast(text: string, kind: ToastKind = "info") {
  const id = ++toastSeq;
  toasts.value.push({ id, text, kind });
  window.setTimeout(() => dismiss(id), kind === "error" ? 4200 : 2400);
}

function errToast(prefix: string, err: unknown) {
  toast(`${prefix}：${errMessage(err)}`, "error");
}

// ---------- 组合函数 ----------
export function useApp() {
  // ---- 派生状态 ----
  const selected = computed<MediaItem | null>(
    () => media.value.find((m) => m.path === selectedPath.value) ?? null
  );

  const previewSrc = computed(() => localSrc(selected.value?.path));

  const selectedDisplay = computed<DisplayInfo | null>(
    () => displays.value.find((d) => d.id === selectedDisplayId.value) ?? null
  );

  const selectedState = computed<DisplayWallpaperState | null>(
    () => states.value.find((s) => s.displayId === selectedDisplayId.value) ?? null
  );

  const selectedStatePhase = computed<PlaybackPhase>(
    () => selectedState.value?.phase ?? "static"
  );

  // 播放控制可用性：该显示器已有非静态壁纸会话
  const canControlPlayback = computed(() => {
    const s = selectedState.value;
    return !!s && s.phase !== "static" && s.phase !== "error";
  });

  // 暂停 / 恢复仅对视频壁纸有意义
  const canControlVideo = computed(
    () => canControlPlayback.value && selectedState.value?.assignment?.kind === "video"
  );

  const hasMedia = computed(() => media.value.length > 0);

  const dirName = computed(() => {
    const p = libraryDir.value;
    if (!p) return "未选择";
    const parts = p.split(/[\\/]/);
    return parts[parts.length - 1] || p;
  });

  // ---- 数据加载 ----
  async function loadSnapshot() {
    try {
      const snap = await api.getAppSnapshot();
      libraryDir.value = snap.libraryDir ?? "";
      displays.value = snap.displays ?? [];
      states.value = snap.states ?? [];
      if (!selectedDisplayId.value) {
        const primary = displays.value.find((d) => d.primary);
        selectedDisplayId.value = (primary ?? displays.value[0])?.id ?? "";
      }
    } catch (err) {
      errToast("读取应用状态失败", err);
    }
  }

  async function loadMedia(keepSelection = true) {
    loadingMedia.value = true;
    try {
      media.value = await api.listMedia();
      if (!keepSelection || !media.value.some((m) => m.path === selectedPath.value)) {
        selectedPath.value = media.value[0]?.path ?? "";
      }
    } catch (err) {
      media.value = [];
      errToast("读取媒体列表失败", err);
    } finally {
      loadingMedia.value = false;
    }
  }

  async function refreshAll() {
    await loadSnapshot();
    await loadMedia();
  }

  // ---- 目录 / 导入 / 删除 ----
  async function saveSettings() {
    try {
      await api.updateSettings({
        libraryDir: libraryDir.value,
        fitMode: fitMode.value,
        muted: muted.value,
      });
    } catch (err) {
      errToast("保存设置失败", err);
    }
  }

  async function pickFolder() {
    try {
      const dir = await api.pickLibraryDirectory();
      if (!dir) return;
      libraryDir.value = dir;
      await saveSettings();
      await loadMedia(false);
      toast(`已切换到 ${dir}`, "success");
    } catch (err) {
      errToast("选择文件夹失败", err);
    }
  }

  async function importPaths(paths: string[]) {
    if (!paths.length) return;
    try {
      const res = await api.importMedia(paths);
      const saved = res?.saved?.length ?? 0;
      const skipped = res?.skipped?.length ?? 0;
      if (saved) {
        toast(`已导入 ${saved} 个文件${skipped ? `，跳过 ${skipped} 个` : ""}`, "success");
        await loadMedia();
        if (res.saved[0]) selectedPath.value = res.saved[0];
      } else {
        toast(skipped ? "没有可导入的图片或视频" : "后端未导入任何文件", "warning");
      }
    } catch (err) {
      errToast("导入失败", err);
    }
  }

  async function removeMedia(path: string) {
    if (!path) return;
    try {
      await api.deleteMedia(path);
      toast("已删除", "success");
      if (selectedPath.value === path) selectedPath.value = "";
      await loadMedia();
    } catch (err) {
      errToast("删除失败", err);
    }
  }

  // ---- 壁纸应用与控制 ----
  async function applySelected() {
    const item = selected.value;
    if (!item) {
      toast("请先选择一个媒体", "warning");
      return;
    }
    if (!selectedDisplayId.value) {
      toast("请先选择显示器", "warning");
      return;
    }
    applying.value = true;
    try {
      const state = await api.applyWallpaper({
        displayId: selectedDisplayId.value,
        path: item.path,
        kind: item.kind,
        fitMode: fitMode.value,
        muted: muted.value,
      });
      upsertState(state);
      await saveSettings();
      toast(`已应用到 ${selectedDisplay.value?.name ?? "显示器"}`, "success");
    } catch (err) {
      errToast("应用失败", err);
    } finally {
      applying.value = false;
    }
  }

  async function controlPlayback(action: PlaybackAction) {
    const id = selectedDisplayId.value;
    if (!id) return;
    try {
      const state = await api.controlPlayback(id, action);
      upsertState(state);
    } catch (err) {
      errToast(action === "pause" ? "暂停失败" : action === "resume" ? "恢复失败" : "停止失败", err);
    }
  }

  function selectItem(path: string) {
    selectedPath.value = path;
  }

  function selectDisplay(id: string) {
    selectedDisplayId.value = id;
  }

  function setFitMode(mode: FitMode) {
    fitMode.value = mode;
    void saveSettings();
  }

  function toggleMuted() {
    muted.value = !muted.value;
    void saveSettings();
  }

  function setDragging(v: boolean) {
    dragging.value = v;
  }

  return {
    // state
    libraryDir,
    media,
    displays,
    states,
    selectedPath,
    selectedDisplayId,
    fitMode,
    muted,
    loadingMedia,
    applying,
    dragging,
    toasts,
    // getters
    selected,
    previewSrc,
    selectedDisplay,
    selectedState,
    selectedStatePhase,
    canControlPlayback,
    canControlVideo,
    hasMedia,
    dirName,
    // actions
    loadSnapshot,
    loadMedia,
    refreshAll,
    pickFolder,
    importPaths,
    removeMedia,
    applySelected,
    controlPlayback,
    selectItem,
    selectDisplay,
    setFitMode,
    toggleMuted,
    setDragging,
    toast,
    dismiss,
  };
}

export type AppStore = ReturnType<typeof useApp>;

/// 显示器分辨率文案（逻辑宽高）
export function boundsText(d: DisplayInfo): string {
  const [, , w, h] = d.logicalBounds ?? [0, 0, 0, 0];
  if (!w || !h) return "";
  return `${Math.round(w)} × ${Math.round(h)}`;
}

/// 阶段 → 中文标签 + 语义色（供状态徽章使用）
export const phaseMeta: Record<PlaybackPhase, { label: string; tone: string }> = {
  static: { label: "静态", tone: "text-faint" },
  preparing: { label: "准备中", tone: "text-warn" },
  playing: { label: "播放中", tone: "text-ok" },
  paused: { label: "已暂停", tone: "text-dim" },
  error: { label: "错误", tone: "text-danger" },
};

/// 安全的阶段元信息取值：后端返回未知阶段时回退为「静态」
export function phaseMetaOf(phase: string): { label: string; tone: string } {
  return phaseMeta[phase as PlaybackPhase] ?? phaseMeta.static;
}

/// 用于预览层的 object-fit 映射
export function fitObject(mode: FitMode): "cover" | "contain" {
  return mode === "fill" ? "cover" : "contain";
}

export { localSrc };
