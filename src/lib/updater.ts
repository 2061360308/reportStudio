import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const LAST_CHECK_KEY = "report-studio:last-update-check";
const SKIP_VERSION_KEY = "report-studio:skip-version";
const CHECK_INTERVAL = 24 * 60 * 60 * 1000;

export interface AppUpdateProgress {
  downloaded: number;
  total: number;
  percent: number;
}

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function isWindows(): boolean {
  return typeof navigator !== "undefined" && /windows/i.test(navigator.userAgent);
}

export function skippedVersion(): string | null {
  return localStorage.getItem(SKIP_VERSION_KEY);
}

export function skipVersion(version: string): void {
  localStorage.setItem(SKIP_VERSION_KEY, version);
}

export async function checkAppUpdate(manual = false): Promise<Update | null> {
  if (!import.meta.env.PROD || !isTauriRuntime()) return null;

  const last = Number(localStorage.getItem(LAST_CHECK_KEY) ?? "0");
  if (!manual && Date.now() - last < CHECK_INTERVAL) return null;

  try {
    const update = await check({ timeout: 15000 });
    localStorage.setItem(LAST_CHECK_KEY, String(Date.now()));
    if (update && !manual && update.version === skippedVersion()) return null;
    return update;
  } catch (error) {
    if (manual) throw error;
    return null;
  }
}

export async function installAppUpdate(
  update: Update,
  onProgress: (progress: AppUpdateProgress) => void,
): Promise<void> {
  let downloaded = 0;
  let total = 0;

  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      total = event.data.contentLength ?? 0;
      onProgress({ downloaded: 0, total, percent: 0 });
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      const percent = total ? Math.min(100, Math.round((downloaded / total) * 100)) : 0;
      onProgress({ downloaded, total, percent });
    } else if (event.event === "Finished") {
      onProgress({ downloaded, total, percent: 100 });
    }
  });

  // Windows 安装阶段应用会被安装器结束，通常不会执行到这里；
  // 其他平台需要手动重启。
  if (!isWindows()) await relaunch();
}
