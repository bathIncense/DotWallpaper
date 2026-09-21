<script setup lang="ts">
// PreviewPanel - 右侧：选中媒体预览 + 壁纸控制 + 各显示器状态
import { computed, onUnmounted, ref, watch } from "vue";
import { fitObject, phaseMetaOf, useApp } from "../composables/useApp";
import SvgIcon from "./SvgIcon.vue";

const app = useApp();

const item = computed(() => app.selected.value);
const src = computed(() => app.previewSrc.value);
const isVideo = computed(() => item.value?.kind === "video");
const objectFit = computed(() => fitObject(app.fitMode.value));

// 删除二次确认（避免原生 confirm）
const pendingDelete = ref(false);
let deleteTimer: number | undefined;
watch(item, () => {
  window.clearTimeout(deleteTimer);
  pendingDelete.value = false;
});
function onRequestDelete() {
  if (!item.value) return;
  if (!pendingDelete.value) {
    pendingDelete.value = true;
    deleteTimer = window.setTimeout(() => (pendingDelete.value = false), 4000);
    return;
  }
  window.clearTimeout(deleteTimer);
  pendingDelete.value = false;
  void app.removeMedia(item.value.path);
}
onUnmounted(() => window.clearTimeout(deleteTimer));

const displayStates = computed(() =>
  app.displays.value.map((d) => {
    const state = app.states.value.find((s) => s.displayId === d.id);
    return {
      id: d.id,
      name: d.name,
      primary: d.primary,
      phase: state?.phase ?? "static",
      current: state?.assignment?.path?.split(/[\\/]/).pop() ?? "",
      error: state?.error ?? "",
    };
  })
);
</script>

<template>
  <aside class="flex w-[320px] shrink-0 flex-col gap-3 overflow-y-auto border-l border-line bg-bg-2/40 p-3.5">
    <!-- 预览卡片 -->
    <div class="mac-card overflow-hidden">
      <div class="flex h-8 items-center gap-2 border-b border-line px-2.5">
        <SvgIcon :name="isVideo ? 'film' : 'image'" :size="12" class="text-faint" />
        <span class="min-w-0 flex-1 truncate text-[11.5px] font-medium text-dim" :title="item?.name">
          {{ item?.name ?? "未选择媒体" }}
        </span>
        <span v-if="item" class="mac-badge text-faint">
          {{ item.kind === "video" ? "视频" : "图片" }}
        </span>
      </div>

      <div class="media-fallback relative aspect-video w-full">
        <video
          v-if="item && isVideo && src"
          :key="src"
          class="h-full w-full"
          :style="{ objectFit }"
          :src="src"
          :muted="app.muted.value"
          autoplay
          loop
          playsinline
          preload="metadata"
          controlslist="nodownload"
        ></video>
        <img
          v-else-if="item && src"
          :key="src"
          class="h-full w-full"
          :style="{ objectFit }"
          :src="src"
          :alt="item.name"
          draggable="false"
        />
        <div v-else class="absolute inset-0 flex flex-col items-center justify-center gap-1.5 text-center">
          <SvgIcon name="image" :size="22" class="text-faint" />
          <span class="text-[11px] text-faint">在左侧选择图片或视频</span>
        </div>

        <!-- 当前显示器阶段 -->
        <span
          v-if="item"
          class="absolute left-2 top-2 flex items-center gap-1 rounded-md bg-black/55 px-1.5 py-[3px] text-[10px] backdrop-blur-sm"
          :class="phaseMetaOf(app.selectedStatePhase.value).tone"
        >
          {{ app.selectedDisplay.value?.name ?? "未选显示器" }} ·
          {{ phaseMetaOf(app.selectedStatePhase.value).label }}
        </span>
      </div>

      <!-- 名称路径 -->
      <p
        v-if="item"
        class="truncate border-t border-line px-2.5 py-1.5 text-[10.5px] text-faint"
        :title="item.path"
      >
        {{ item.path }}
      </p>
    </div>

    <!-- 控制区 -->
    <div class="mac-card flex flex-col gap-3 p-3">
      <div class="flex items-center justify-between gap-2">
        <span class="text-[11px] text-faint">显示方式</span>
        <div class="mac-segmented" role="radiogroup" aria-label="显示方式">
          <button
            class="mac-segment cursor-pointer"
            :class="app.fitMode.value === 'fill' ? 'mac-segment-active' : ''"
            role="radio"
            :aria-checked="app.fitMode.value === 'fill'"
            @click="app.setFitMode('fill')"
          >
            填充
          </button>
          <button
            class="mac-segment cursor-pointer"
            :class="app.fitMode.value === 'fit' ? 'mac-segment-active' : ''"
            role="radio"
            :aria-checked="app.fitMode.value === 'fit'"
            @click="app.setFitMode('fit')"
          >
            适应
          </button>
        </div>
      </div>

      <div class="flex items-center justify-between gap-2">
        <span class="flex items-center gap-1.5 text-[11px] text-faint">
          <SvgIcon :name="app.muted.value ? 'mute' : 'volume'" :size="13" :class="app.muted.value ? 'text-dim' : 'text-accent'" />
          静音播放
        </span>
        <button
          class="mac-switch"
          :class="app.muted.value ? 'mac-switch-on' : ''"
          role="switch"
          :aria-checked="app.muted.value"
          @click="app.toggleMuted()"
        >
          <span class="mac-switch-knob"></span>
        </button>
      </div>

      <button
        class="mac-btn mac-btn-primary w-full"
        :disabled="!item || app.applying.value || !app.selectedDisplayId.value"
        @click="app.applySelected()"
      >
        <SvgIcon name="check" :size="12" />
        {{ app.applying.value ? "应用中…" : "应用壁纸" }}
      </button>

      <div class="grid grid-cols-3 gap-1.5">
        <button
          class="mac-btn !px-0 text-[11.5px]"
          :disabled="!app.canControlVideo.value"
          title="暂停所选显示器的动态壁纸"
          @click="app.controlPlayback('pause')"
        >
          <SvgIcon name="pause" :size="12" />
          暂停
        </button>
        <button
          class="mac-btn !px-0 text-[11.5px]"
          :disabled="!app.canControlVideo.value"
          title="恢复播放"
          @click="app.controlPlayback('resume')"
        >
          <SvgIcon name="play" :size="11" fill />
          恢复
        </button>
        <button
          class="mac-btn !px-0 text-[11.5px]"
          :disabled="!app.canControlPlayback.value"
          title="停止该显示器的壁纸"
          @click="app.controlPlayback('stop')"
        >
          <SvgIcon name="stop" :size="11" />
          停止
        </button>
      </div>

      <button
        class="mac-btn mac-btn-danger w-full !bg-transparent !border-line"
        :disabled="!item"
        @click="onRequestDelete"
      >
        <SvgIcon name="trash" :size="12" />
        {{ pendingDelete ? "再次点击确认删除" : "删除文件" }}
      </button>
      <p v-if="pendingDelete" class="-mt-2 text-[10.5px] text-danger">
        将从磁盘永久删除该文件，4 秒内再次点击确认。
      </p>
    </div>

    <!-- 显示器状态 -->
    <div class="mac-card flex flex-col gap-1.5 p-3">
      <span class="text-[11px] font-medium text-faint">显示器状态</span>
      <div v-if="!displayStates.length" class="text-[11px] text-faint">未检测到显示器</div>
      <button
        v-for="d in displayStates"
        :key="d.id"
        class="flex w-full cursor-pointer items-center gap-2 rounded-lg px-2 py-1.5 text-left transition-colors hover:bg-panel-2"
        :class="d.id === app.selectedDisplayId.value ? 'bg-panel-2' : ''"
        @click="app.selectDisplay(d.id)"
      >
        <span class="h-1.5 w-1.5 shrink-0 rounded-full" :class="d.id === app.selectedDisplayId.value ? 'bg-accent' : 'bg-line-2'"></span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-[11.5px] text-tx">
            {{ d.name }}<span v-if="d.primary" class="text-faint"> · 主</span>
          </span>
          <span v-if="d.current" class="block truncate text-[10px] text-faint" :title="d.current">{{ d.current }}</span>
          <span v-if="d.error" class="block truncate text-[10px] text-danger" :title="d.error">{{ d.error }}</span>
        </span>
        <span class="mac-badge shrink-0" :class="phaseMetaOf(d.phase).tone">
          {{ phaseMetaOf(d.phase).label }}
        </span>
      </button>
    </div>
  </aside>
</template>
