<script setup lang="ts">
// 关于 - 应用信息
import { NIcon } from "naive-ui";
import { Info } from "lucide-vue-next";
import dotCode from "@/assets/dotCode.png";

defineProps<{ appName: string; appVersion: string }>();
</script>

<template>
  <div class="panel-body">
    <div class="panel-header">
      <NIcon :component="Info" :size="18" class="panel-icon text-accent" />
      <div class="panel-title">关于</div>
    </div>

    <div class="about-layout mt-4 flex flex-col gap-3">
      <!-- 应用信息 -->
      <div class="app-card flex items-center gap-3 rounded-lg bg-panel-2 px-4 py-4">
        <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-accent">
          <img :src="dotCode" alt="DotWallpaper Logo" class="h-6 w-6" />
        </div>
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <span class="text-[14px] font-semibold text-tx">{{ appName }}</span>
            <span class="rounded-md bg-white/5 px-1.5 py-0.5 text-[10.5px] text-faint">v{{ appVersion }}</span>
          </div>
          <p class="mt-0.5 text-[11px] text-faint">桌面壁纸管理工具</p>
        </div>
      </div>

      <div class="mt-1 text-center text-[10.5px] text-faint/70">
        © 2026 DotWallpaper. Made with ♥
      </div>
    </div>
  </div>
</template>

<style scoped>
.panel-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.panel-icon {
  flex-shrink: 0;
}
.panel-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-tx);
}
</style>
