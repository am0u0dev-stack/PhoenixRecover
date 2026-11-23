//! Shared types for the scan/recovery pipeline.

use serde::{self, Deserialize, Serialize};

/// Kind of recovery source.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Drive,
    Folder,
    Image,
}

/// Detected device class, used for UI warnings (SSD TRIM).
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Ssd,
    Hdd,
    Usb,
    Sd,
    Unknown,
}

/// A scannable source presented to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct SourceInfo {
    pub name: String,
    pub path: String,
    pub kind: SourceKind,
    pub size_bytes: u64,
    pub device_type: DeviceType,
    pub read_only: bool,
}

/// Recovery strategy that produced a candidate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScanStrategy {
    Undelete,
    Carve,
}

/// One recoverable file shown in the results table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverableFile {
    pub id: String,
    pub name: String,
    pub ext: String,
    pub size: u64,
    pub strategy: ScanStrategy,
    pub offset: Option<u64>,
    /// Where the validated copy lives while awaiting user confirmation.
    pub staging_path: Option<String>,
}

/// Scan phase reported in progress events.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProgressPhase {
    Metadata,
    Carve,
    Office,
    Done,
}

/// Live scan progress pushed to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub phase: ProgressPhase,
    pub percent: f64,
    pub bytes_scanned: u64,
    pub total_bytes: u64,
    pub note: String,
}

/// Result of `scan_drive`: the recoverable rows plus engine notes (why a
/// scan found nothing, permission problems, carve statistics) shown in the UI.
#[derive(Debug, Clone, Serialize)]
pub struct ScanOutcome {
    pub files: Vec<RecoverableFile>,
    pub notes: Vec<String>,
}

/// Summary returned after writing selected files to the output directory.
#[derive(Debug, Clone, Serialize)]
pub struct RecoveryReport {
    pub recovered: usize,
    pub failed: usize,
    pub bytes_recovered: u64,
    pub by_type: std::collections::BTreeMap<String, u64>,
    pub output_dir: String,
    pub notes: Vec<String>,
}

/// One 16-byte row of a hex preview pane.
#[derive(Debug, Clone, Serialize)]
pub struct HexRow {
    pub offset: u64,
    pub hex: Vec<u8>,
    pub ascii: String,
}

/// Hex/preview payload for a staged file (read-only, first bytes only).
#[derive(Debug, Clone, Serialize)]
pub struct FilePreview {
    pub path: String,
    pub size: u64,
    pub truncated: bool,
    pub rows: Vec<HexRow>,
}

/// State of the deep-scan checkpoint for a source path.
#[derive(Debug, Clone, Serialize)]
pub struct CheckpointStatus {
    pub exists: bool,
    pub checkpoint_path: String,
    pub output_dir: String,
}
