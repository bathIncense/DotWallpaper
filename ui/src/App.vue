<script setup lang="ts">
// App - macOS 紧凑三段式布局：顶部工具栏 / 左侧媒体网格 / 右侧预览与控制
import { onMounted, onUnmounted } from "vue";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useApp } from "./composables/useApp";
import TopBar from "./components/TopBar.vue";
import MediaGrid from "./components/MediaGrid.vue";
import PreviewPanel from "./components/PreviewPanel.vue";
import Toaster from "./components/Toaster.vue";
import SvgIcon from "./components/SvgIcon.vue";

const app = useApp();

// ---------- 拖放导入 ----------
// 优先使用 Tauri 原生拖放事件（可获得真实本地路径），
// 浏览器 / 开发环境下回退到 HTML5 drop。
let unlistenDragDrop: (() => void) | null = null;

onMounted(async () => {
  void app.refreshAll();
  try {
    unlistenDragDrop = await getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "enter" || payload.type === "over") {
        app.setDragging(true);
      } else if (payload.type === "leave") {
        app.setDragging(false);
      } else if (payload.type === "drop") {
        app.setDragging(false);
        const paths = payload.paths ?? [];
        if (paths.length) void app.importPaths(paths);
      }
    });
  } catch {
    // 非 Tauri 环境：使用 HTML5 drop（见 onHtmlDrop）
  }
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  unlistenDragDrop?.();
  window.removeEventListener("keydown", onKeydown);
});

// HTML5 回退：仅在没有 Tauri 拖放事件时生效
function onHtmlDrop(e: DragEvent) {
  if (unlistenDragDrop) return;
  const files = Array.from(e.dataTransfer?.files ?? []);
  const paths = files
    .map((f) => (f as File & { path?: string }).path || f.name)
    .filter((p) => p.startsWith("/") || /^[a-zA-Z]:[\\/]/.test(p));
  app.setDragging(false);
  if (paths.length) void app.importPaths(paths);
  else if (files.length) app.toast("浏览器环境无法取得文件路径，请在应用内拖放导入", "warning");
}

function onHtmlDragOver(e: DragEvent) {
  if (unlistenDragDrop) return;
  e.preventDefault();
  app.setDragging(true);
}

// ---------- 快捷键 ----------
function onKeydown(e: KeyboardEvent) {
  if (!e.metaKey) return;
  const key = e.key.toLowerCase();
  if (key === "r") {
    e.preventDefault();
    void app.refreshAll();
  } else if (key === "s") {
    e.preventDefault();
    void app.applySelected();
  }
}
</script>

<template>
  <div
    class="flex h-screen w-screen flex-col overflow-hidden bg-bg text-tx"
    @dragover="onHtmlDragOver"
    @dragleave.self="app.setDragging(false)"
    @drop.prevent="onHtmlDrop"
  >
    <TopBar />

    <main class="flex min-h-0 flex-1">
      <MediaGrid />
      <PreviewPanel />
    </main>

    <!-- 拖放遮罩 -->
    <Teleport to="body">
      <Transition name="drag">
        <div
          v-if="app.dragging.value"
          class="pointer-events-none fixed inset-0 z-[100] flex items-center justify-center bg-black/40 backdrop-blur-[2px]"
        >
          <div class="flex items-center gap-2 rounded-2xl border border-accent/45 bg-accent-soft px-5 py-3.5 text-[13px] font-medium text-accent shadow-[0_18px_44px_rgba(0,0,0,0.5)]">
            <SvgIcon name="import" :size="18" />
            松开即导入壁纸目录
          </div>
        </div>
      </Transition>
    </Teleport>

    <Toaster />
  </div>
</template>

<style scoped>
.drag-enter-active,
.drag-leave-active {
  transition: opacity 0.15s ease;
}
.drag-enter-from,
.drag-leave-to {
  opacity: 0;
}
</style>
