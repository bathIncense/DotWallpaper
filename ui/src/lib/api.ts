// Native WebKit bridge: the Xcode macOS app hosts the Vue bundle directly.
import type {
  AppSnapshot,
  DisplayInfo,
  DisplayWallpaperState,
  ImportResult,
  MediaItem,
  PlaybackAction,
  Settings,
  WallpaperAssignment,
} from "../types";

type NativePayload = Record<string, unknown>;
type NativeResponseEvent = CustomEvent<{ id: string; payload: unknown }>;
type NativeEvent = CustomEvent<{ name: string; payload: unknown }>;

let requestSeq = 0;
const pending = new Map<
  string,
  { resolve: (value: unknown) => void; reject: (reason: unknown) => void }
>();

if (typeof window !== "undefined") {
  window.addEventListener("dotwallpaper-response", (event) => {
    const { id, payload } = (event as NativeResponseEvent).detail;
    const request = pending.get(id);
    if (!request) return;
    pending.delete(id);
    const error = payload && typeof payload === "object" && "error" in payload
      ? (payload as NativePayload).error
      : undefined;
    if (typeof error === "string") request.reject(new Error(error));
    else request.resolve(payload ?? {});
  });
}

function invokeNative<T>(
  action: string,
  params: NativePayload = {},
): Promise<T> {
  const native = typeof window === "undefined" ? undefined : window.DotWallpaperNative;
  if (!native) {
    return Promise.reject(new Error("必须在 WallpaperEngine.app 内运行"));
  }

  const id = `request-${Date.now()}-${++requestSeq}`;
  return new Promise<T>((resolve, reject) => {
    pending.set(id, { resolve: resolve as (value: unknown) => void, reject });
    native.invoke(action, { ...params, requestId: id });
  });
}

export function onNativeEvent<T>(name: string, handler: (payload: T) => void): () => void {
  const listener = (event: Event) => {
    const detail = (event as NativeEvent).detail;
    if (detail?.name === name) handler(detail.payload as T);
  };
  window.addEventListener("dotwallpaper-event", listener);
  return () => window.removeEventListener("dotwallpaper-event", listener);
}

export const api = {
  getAppSnapshot: () => invokeNative<AppSnapshot>("getAppSnapshot"),
  listMedia: () => invokeNative<MediaItem[]>("listMedia").then((p) => p as unknown as MediaItem[]),
  listDisplays: () => invokeNative<DisplayInfo[]>("listDisplays").then((p) => p as unknown as DisplayInfo[]),
  applyWallpaper: (assignment: WallpaperAssignment) =>
    invokeNative<DisplayWallpaperState>("applyWallpaper", { assignment: JSON.stringify(assignment) })
      .then((p) => p as unknown as DisplayWallpaperState),
  controlPlayback: (displayId: string, action: PlaybackAction) =>
    invokeNative<DisplayWallpaperState>("controlPlayback", {
      displayId,
      controlAction: action,
    }).then((p) => p as unknown as DisplayWallpaperState),
  pickLibraryDirectory: () =>
    invokeNative<{ pickedDirectory: string | null }>("pickLibraryDirectory").then(
      (p) => p.pickedDirectory ?? null,
    ),
  pickMediaFiles: () =>
    invokeNative<{ pickedFiles: string[] | null }>("pickMediaFiles").then(
      (p) => p.pickedFiles ?? [],
    ),
  importMedia: (paths: string[]) =>
    invokeNative<ImportResult>("importMedia", { paths: JSON.stringify(paths) }).then(
      (p) => p as unknown as ImportResult,
    ),
  deleteMedia: (path: string) =>
    invokeNative<{ ok: boolean }>("deleteMedia", { path }).then(() => undefined),
  updateSettings: (settings: Settings) =>
    invokeNative<{ ok: boolean }>("updateSettings", {
      settings: JSON.stringify(settings),
    }).then(() => undefined),
};

/// 本地绝对路径 → WKWebView 可加载的 file URL。
export function localSrc(path?: string | null): string {
  if (!path) return "";
  return `file://${path.split("/").map(encodeURIComponent).join("/")}`;
}

/// 统一提取错误文案
export function errMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}
