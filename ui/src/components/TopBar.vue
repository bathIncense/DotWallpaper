<script setup lang="ts">
// TopBar - 顶部工具栏：壁纸目录 + 选择文件夹 / 导入文件 + 显示器选择
import { computed, ref } from "vue";
import { boundsText, useApp } from "../composables/useApp";
import { api, errMessage } from "../lib/api";
import SvgIcon from "./SvgIcon.vue";

const app = useApp();
const fileInput = ref<HTMLInputElement | null>(null);

const isTauri = "__TAURI_INTERNALS__" in window;

const displayLabel = computed(() => {
  const d = app.selectedDisplay.value;
  if (!d) return "未检测到显示器";
  const res = boundsText(d);
  const tags = [d.primary ? "主显示器" : null, d.mirrored ? "镜像" : null]
    .filter(Boolean)
    .join(" · ");
  return [d.name, res, tags].filter(Boolean).join("  ");
});

const displayCount = computed(() => app.displays.value.length);

// 导入文件：优先取原生 File.path（Tauri/WKWebView 可用时），否则提示拖放
function onFileChange(e: Event) {
  const input = e.target as HTMLInputElement;
  const files = Array.from(input.files ?? []);
  input.value = "";
  if (!files.length) return;
  const paths = files
    .map((f) => (f as File & { path?: string }).path || f.name)
    .filter((p) => p.startsWith("/") || /^[a-zA-Z]:[\\/]/.test(p));
  if (!paths.length) {
    app.toast("当前 WebView 未提供文件绝对路径，请把图片或视频直接拖入窗口", "warning");
    return;
  }
  void app.importPaths(paths);
}

async function clickImport() {
  if (!isTauri) {
    fileInput.value?.click();
    return;
  }
  try {
    const paths = await api.pickMediaFiles();
    if (paths.length) await app.importPaths(paths);
  } catch (err) {
    app.toast(`导入失败：${errMessage(err)}`, "error");
  }
}
</script>

<template>
  <header
    class="flex h-11 shrink-0 items-center gap-2 border-b border-line bg-bg-2/80 px-3.5 backdrop-blur"
    data-tauri-drag-region
  >
    <!-- 目录 -->
    <div class="flex min-w-0 flex-1 items-center gap-2">
      <SvgIcon name="folder" :size="14" class="text-accent" />
      <span class="shrink-0 text-[12px] text-faint">壁纸目录</span>
      <span
        class="min-w-0 truncate text-[12px] text-dim"
        :title="app.libraryDir.value || '未选择'"
        >{{ app.dirName.value }}</span
      >
    </div>

    <!-- 操作 -->
    <button class="mac-btn" title="选择壁纸目录" @click="app.pickFolder()">
      <SvgIcon name="folder" :size="12" />
      选择文件夹
    </button>
    <button class="mac-btn" title="导入图片或视频" @click="clickImport">
      <SvgIcon name="import" :size="12" />
      导入文件
    </button>
    <button
      class="mac-btn !px-1.5"
      title="重新加载列表（⌘R）"
      :disabled="app.loadingMedia.value"
      @click="app.refreshAll()"
    >
      <SvgIcon
        name="refresh"
        :size="13"
        :class="app.loadingMedia.value ? 'animate-spin' : ''"
      />
    </button>

    <span v-if="displayCount" class="mx-1 h-5 w-px shrink-0 bg-line"></span>

    <!-- 显示器选择 -->
    <label class="flex min-w-0 items-center gap-1.5">
      <SvgIcon name="monitor" :size="13" class="text-faint" />
      <select
        class="mac-select"
        :value="app.selectedDisplayId.value"
        :title="displayLabel"
        :disabled="!displayCount"
        @change="app.selectDisplay(($event.target as HTMLSelectElement).value)"
      >
        <option v-for="d in app.displays.value" :key="d.id" :value="d.id">
          {{ d.name }}{{ d.primary ? "（主）" : "" }} {{ boundsText(d) }}
        </option>
      </select>
    </label>

    <input
      ref="fileInput"
      type="file"
      multiple
      accept="image/*,video/*"
      class="hidden"
      @change="onFileChange"
    />
  </header>
</template>
