import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DownloadProgress, HistoryEntry, RecordingState } from "../types";

export const onRecordingStateChanged = (
  cb: (state: RecordingState) => void,
): Promise<UnlistenFn> =>
  listen<RecordingState>("recording-state-changed", (e) => cb(e.payload));

export const onTranscriptionComplete = (
  cb: (entry: HistoryEntry) => void,
): Promise<UnlistenFn> =>
  listen<HistoryEntry>("transcription-complete", (e) => cb(e.payload));

export const onDownloadProgress = (
  cb: (progress: DownloadProgress) => void,
): Promise<UnlistenFn> =>
  listen<DownloadProgress>("download-progress", (e) => cb(e.payload));

export const onSettingsUpdated = (cb: () => void): Promise<UnlistenFn> =>
  listen("settings-updated", () => cb());

export const onError = (cb: (message: string) => void): Promise<UnlistenFn> =>
  listen<string>("app-error", (e) => cb(e.payload));
