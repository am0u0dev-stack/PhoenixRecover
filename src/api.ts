import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CheckpointStatus,
  FilePreview,
  HistoryEntry,
  RecoverableFile,
  RecoveryReport,
  ScanProgress,
  SourceInfo,
} from "./types";

export function listSources(): Promise<SourceInfo[]> {
  return invoke("list_sources");
}

export function scanDrive(
  path: string,
  deep: boolean,
  resume = false,
  kinds: string[] = [],
): Promise<RecoverableFile[]> {
  return invoke("scan_drive", { path, deep, resume, kinds });
}

export function recoverFiles(ids: string[], outputDir: string): Promise<RecoveryReport> {
  return invoke("recover_files", { files: ids, outputDir });
}

export function cancelScan(): Promise<void> {
  return invoke("cancel_scan");
}

export function onScanProgress(cb: (p: ScanProgress) => void): Promise<() => void> {
  return listen<ScanProgress>("scan-progress", (e) => cb(e.payload));
}

export function scanHistory(): Promise<HistoryEntry[]> {
  return invoke("scan_history");
}

export function checkpointStatus(path: string, kinds: string[] = []): Promise<CheckpointStatus> {
  return invoke("checkpoint_status", { path, kinds });
}

export function fileHexPreview(path: string): Promise<FilePreview> {
  return invoke("file_hex_preview", { path });
}
