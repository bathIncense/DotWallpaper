// 媒体类型定义和平台工具

/// 媒体类型
export type MediaKind = 'image' | 'video' | 'gif' | 'dynamic_heic';

/// 填充模式
export type FitMode = 'fill' | 'fit' | 'stretch' | 'center' | 'tile';

/// 平台能力
export interface PlatformCapabilities {
    platform: 'macos';
    media_kinds: string[];
    supported_fit_modes: string[];
    multi_display: boolean;
    native_dynamic_heic: boolean;
}

/// 显示器信息
export interface DisplayInfo {
    id: string;
    name: string;
    logical_bounds: [number, number, number, number];
    scale_factor: number;
    primary: boolean;
    mirrored: boolean;
}

/// 壁纸分配请求
export interface WallpaperAssignment {
    display_id: string;
    media_id: string;
    fit_mode: FitMode;
    muted: boolean;
}

/// 显示器壁纸状态
export interface DisplayWallpaperState {
    display_id: string;
    request_id: number;
    revision: number;
    phase: string;
    desired_assignment: WallpaperAssignment | null;
    actual_assignment: WallpaperAssignment | null;
    system_wallpaper: string | null;
    error: string | null;
}

/// 媒体条目
export interface MediaEntry {
    id: string;
    path: string;
    kind: MediaKind;
    content_version: string;
    thumbnail: string;
    thumbnail_state: string;
    read_only: boolean;
    width: number;
    height: number;
    duration_ms?: number;
    dynamic_kind?: string;
}

/// 获取媒体类型显示名称
export function mediaKindName(kind: MediaKind): string {
    switch (kind) {
        case 'image': return '图片';
        case 'video': return '视频';
        case 'gif': return 'GIF';
        case 'dynamic_heic': return '动态壁纸';
    }
}

/// 获取填充模式显示名称
export function fitModeName(mode: FitMode): string {
    switch (mode) {
        case 'fill': return '填充';
        case 'fit': return '适应';
        case 'stretch': return '拉伸';
        case 'center': return '居中';
        case 'tile': return '平铺';
    }
}

/// 检查文件是否为支持的媒体类型
export function getMediaKindFromPath(path: string): MediaKind | null {
    const ext = path.split('.').pop()?.toLowerCase();
    if (!ext) return null;

    const imageExts = ['jpg', 'jpeg', 'png', 'bmp', 'webp', 'heic', 'heif'];
    const videoExts = ['mp4', 'mov', 'm4v', 'avi', 'mkv', 'webm'];
    const gifExts = ['gif'];

    if (imageExts.includes(ext)) {
        // HEIC 需要进一步检测是否为动态
        if (ext === 'heic' || ext === 'heif') {
            // TODO: 检测动态 HEIC
            return 'image';
        }
        return 'image';
    }
    if (videoExts.includes(ext)) return 'video';
    if (gifExts.includes(ext)) return 'gif';

    return null;
}

/// 生成媒体 ID
export function makeMediaId(kind: MediaKind, path: string): string {
    return `${kind}/${path}`;
}

/// 从媒体 ID 解析类型和路径
export function parseMediaId(mediaId: string): { kind: MediaKind; path: string } | null {
    const slashIndex = mediaId.indexOf('/');
    if (slashIndex < 0) return null;
    const kind = mediaId.substring(0, slashIndex) as MediaKind;
    const path = mediaId.substring(slashIndex + 1);
    return { kind, path };
}
