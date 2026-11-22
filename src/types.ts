export type SourceKind = "drive" | "folder" | "image";
export type DeviceType = "ssd" | "hdd" | "usb" | "sd" | "unknown";
export type ScanStrategy = "undelete" | "carve";

export interface SourceInfo {
  name: string;
  path: string;
  kind: SourceKind;
  size_bytes: number;
  device_type: DeviceType;
  read_only: boolean;
}

export interface RecoverableFile {
  id: string;
  name: string;
  ext: string;
  size: number;
  strategy: ScanStrategy;
  offset: number | null;
  staging_path: string | null;
}

export interface ScanProgress {
  phase: "metadata" | "carve" | "office" | "done";
  percent: number;
  bytes_scanned: number;
  total_bytes: number;
  note: string;
}

export interface CheckpointStatus {
  exists: boolean;
  checkpoint_path: string;
  output_dir: string;
}

export interface HexRow {
  offset: number;
  hex: number[];
  ascii: string;
}

export interface FilePreview {
  path: string;
  size: number;
  truncated: boolean;
  rows: HexRow[];
}

export interface HistoryEntry {
  id: number;
  kind: string;
  source: string;
  detail: string;
  files: number;
  bytes: number;
  ts: string;
}

export interface RecoveryReport {
  recovered: number;
  failed: number;
  bytes_recovered: number;
  by_type: Record<string, number>;
  output_dir: string;
  notes: string[];
}
