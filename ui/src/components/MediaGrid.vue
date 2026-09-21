<script setup lang="ts">
// MediaGrid - 左栏：图片 / 视频缩略图网格
import { computed, ref } from "vue";
import { localSrc, useApp } from "../composables/useApp";
import type { MediaItem } from "../types";
import SvgIcon from "./SvgIcon.vue";

const app = useApp();

const items = computed<MediaItem[]>(() => app.media.value);
const selectedPath = computed(() => app.selectedPath.value);

// 已应用到某台显示器的壁纸路径集合（用于角标圆点）
const appliedPaths = computed(() => {
  const set = new Set<string>();
  for (const s of app.states.value) {
    if (s.assignment?.path) set.add(s.assignment.path);
  }
  return set;
});

// 缩略图加载失败 → 显示纯 CSS 渐变占位
const failed = ref<Set<string>>(new Set());
function onThumbError(item: MediaItem) {
  if (failed.value.has(item.path)) return;
  const next = new Set(failed.value);
  next.add(item.path);
  failed.value = next;
}

function thumbFor(item: MediaItem): string {
  return failed.value.has(item.path) ? "" : localSrc(item.thumb) || localSrc(item.path);
}

function onClick(item: MediaItem) {
  app.selectItem(item.path);
}

function onKeydown(e: KeyboardEvent, item: MediaItem) {
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    app.selectItem(item.path);
  }
}
</script>

<template>
  <section class="flex min-h-0 min-w-0 flex-1 flex-col">
    <div class="flex h-8 shrink-0 items-center justify-between px-3.5">
      <span class="text-[11px] font-medium tracking-wide text-faint">媒体库</span>
      <span v-if="items.length" class="text-[11px] text-faint">{{ items.length }} 项</span>
    </div>

    <!-- 网格 -->
    <div
      v-if="items.length"
      class="min-h-0 flex-1 overflow-y-auto px-3.5 pb-3.5"
    >
      <ul class="grid grid-cols-[repeat(auto-fill,minmax(132px,1fr))] gap-2.5">
        <li
          v-for="item in items"
          :key="item.path"
          class="group relative cursor-pointer overflow-hidden rounded-[10px] border bg-panel transition-all duration-150 hover:-translate-y-[1px] hover:border-line-2"
          :class="
            item.path === selectedPath
              ? 'border-accent shadow-[0_0_0_2px_var(--color-accent-soft)]'
              : 'border-line'
          "
          tabindex="0"
          role="button"
          :aria-label="item.name"
          @click="onClick(item)"
          @keydown="onKeydown($event, item)"
        >
          <div class="media-fallback relative aspect-[16/10] w-full overflow-hidden">
            <img
              v-if="thumbFor(item)"
              :src="thumbFor(item)"
              :alt="item.name"
              class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-[1.03]"
              loading="lazy"
              decoding="async"
              draggable="false"
              @error="onThumbError(item)"
            />
            <!-- 视频角标 -->
            <span
              v-if="item.kind === 'video'"
              class="absolute bottom-1.5 right-1.5 flex h-[18px] w-[18px] items-center justify-center rounded-full bg-black/60 text-white backdrop-blur-sm"
              title="视频"
            >
              <SvgIcon name="play" :size="9" fill />
            </span>
            <!-- 已在桌面上显示 -->
            <span
              v-if="appliedPaths.has(item.path)"
              class="absolute left-1.5 top-1.5 h-2 w-2 rounded-full bg-ok shadow-[0_0_0_2px_rgba(0,0,0,0.35)]"
              title="已作为壁纸"
            ></span>
          </div>
          <div class="flex min-w-0 items-center gap-1 px-2 py-1.5">
            <SvgIcon
              :name="item.kind === 'video' ? 'film' : 'image'"
              :size="11"
              class="shrink-0 text-faint"
            />
            <span class="min-w-0 truncate text-[11px]" :class="item.path === selectedPath ? 'text-tx' : 'text-dim'" :title="item.name">
              {{ item.name }}
            </span>
          </div>
        </li>
      </ul>
    </div>

    <!-- 空状态引导 -->
    <div v-else class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-6 pb-8 text-center">
      <div class="media-fallback flex h-20 w-20 items-center justify-center rounded-2xl border border-dashed border-line-2">
        <SvgIcon name="image" :size="30" class="text-faint" />
      </div>
      <div>
        <p class="text-[13px] font-medium text-tx">
          {{ app.loadingMedia.value ? "正在加载媒体库…" : "媒体库还是空的" }}
        </p>
        <p class="mt-1 text-[11.5px] leading-relaxed text-faint">
          选择一个包含图片 / 视频的文件夹，或直接把文件拖入窗口导入
        </p>
      </div>
      <div class="flex items-center gap-2">
        <button class="mac-btn mac-btn-primary" @click="app.pickFolder()">
          <SvgIcon name="folder" :size="12" />
          选择文件夹
        </button>
        <span class="text-[11px] text-faint">或拖入图片 / 视频</span>
      </div>
    </div>
  </section>
</template>
