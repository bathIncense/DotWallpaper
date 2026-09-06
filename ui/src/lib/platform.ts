// 平台工具函数

import { invoke } from '@tauri-apps/api/core';
import type {
    PlatformCapabilities,
    DisplayInfo,
    WallpaperAssignment,
    DisplayWallpaperState,
    FitMode,
} from '../types/media';

/// 获取平台能力
export async function getPlatformCapabilities(): Promise<PlatformCapabilities> {
    return await invoke('get_platform_capabilities');
}

/// 枚举所有显示器
export async function listDisplays(): Promise<DisplayInfo[]> {
    return await invoke('list_displays');
}

/// 应用壁纸到指定显示器
export async function applyWallpaper(assignment: WallpaperAssignment): Promise<DisplayWallpaperState> {
    return await invoke('apply_wallpaper', { assignment });
}

/// 获取指定显示器的壁纸状态
export async function getWallpaperState(displayId: string): Promise<DisplayWallpaperState> {
    return await invoke('get_wallpaper_state', { displayId });
}

/// 暂停指定显示器的动态壁纸
export async function pauseWallpaper(displayId: string): Promise<void> {
    return await invoke('pause_wallpaper', { displayId });
}

/// 恢复指定显示器的动态壁纸
export async function resumeWallpaper(displayId: string): Promise<void> {
    return await invoke('resume_wallpaper', { displayId });
}

/// 停止指定显示器的动态壁纸
export async function stopWallpaper(displayId: string): Promise<void> {
    return await invoke('stop_wallpaper', { displayId });
}

/// 检查是否为 macOS 平台
export function isMacOS(): boolean {
    return navigator.platform?.toLowerCase().includes('mac') ?? false;
}

/// 获取跨平台快捷键前缀
export function shortcutPrefix(): string {
    return '⌘';
}

/// 获取平台特定的快捷键键名
export function shortcutKey(key: string): string {
    return `${shortcutPrefix()}+${key}`;
}
