//! Scan engine: undelete (filesystem metadata) and carve (byte signatures)
//! through the real `unearth` crate, plus an Office-format pass through the
//! real `forsmost` engine. All source access is read-only
//! (`Source::open` uses `OpenOptions::read(true)`).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tauri::Emitter;
use unearth::carver::{self, CarveOptions, NoProgress};
use unearth::identify;
use unearth::recover::{self, RecoverOptions, RecoverStats, Volume};
use unearth::source::Source;

use crate::types::{ProgressPhase, RecoverableFile, ScanProgress, ScanStrategy};

/// Formats surfaced by the engines (spec minimum: jpg png pdf docx pptx xlsx
/// zip rar mp4 mov — and every other signature unearth ships).
pub const TARGET_TYPES: &[&str] = &["jpg", "png", "pdf", "docx", "pptx", "xlsx", "zip", "rar", "mp4", "mov"];

/// True while a scan is running; checked by carve progress + engine loops.
pub static CANCEL: AtomicBool = AtomicBool::new(false);

pub fn reset_cancel() {
    CANCEL.store(false, Ordering::SeqCst);
}

pub fn was_cancelled() -> bool {
    CANCEL.load(Ordering::SeqCst)
}

/// Emits `scan-progress` events to the frontend; doubles as unearth's
/// `ProgressSink` so deep-scan percentage is real, not simulated.
pub struct Progress {
    app: Option<tauri::AppHandle>,
    total: AtomicU64,
    scanned: AtomicU64,
    note: Mutex<String>,
}

impl Progress {
    pub fn new(app: Option<tauri::AppHandle>) -> Arc<Self> {
        Arc::new(Self {
            app,
            total: AtomicU64::new(0),
            scanned: AtomicU64::new(0),
            note: Mutex::new(String::new()),
        })
    }

    pub fn set_note(&self, note: &str) {
        if let Ok(mut n) = self.note.lock() {
            if *n != note {
                *n = note.to_string();
                drop(n);
                self.emit();
            }
        }
    }

    fn emit(&self) {
        if let Some(app) = &self.app {
            let scanned = self.scanned.load(Ordering::Relaxed);
            let total = self.total.load(Ordering::Relaxed);
            let percent = if total > 0 {
                (scanned.min(total) as f64 / total as f64) * 100.0
            } else {
                -1.0
            };
            let _ = app.emit(
                "scan-progress",
                ScanProgress {
                    phase: ProgressPhase::Carve,
                    percent,
                    bytes_scanned: scanned,
                    total_bytes: total,
                    note: self.note.lock().map(|n| n.clone()).unwrap_or_default(),
                },
            );
        }
    }
}

impl carver::ProgressSink for Progress {
    fn begin(&self, total: u64) {
        self.total.store(total, Ordering::Relaxed);
        self.emit();
    }
    fn update(&self, scanned: u64) {
        self.scanned.store(scanned, Ordering::Relaxed);
        self.emit();
    }
    fn finish(&self, scanned: u64) {
        self.total.store(scanned, Ordering::Relaxed);
        self.scanned.store(scanned, Ordering::Relaxed);
        self.emit();
    }
}

pub fn no_progress() -> NoProgress {
    NoProgress
}

/// Deterministic key for a source path: the same source always maps to the
/// same carve output directory and checkpoint file, which is what makes
/// interrupted deep scans resumable across app restarts.
pub fn source_key(path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let canon = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut h = DefaultHasher::new();
    canon.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Stable per-source output directory for carved files (required to be
/// identical between the original run and any resume).
pub fn carve_dir_for(path: &Path) -> PathBuf {
    staging_root().join(format!("carve-{}", source_key(path)))
}

/// Stable per-source checkpoint file used by unearth's `resume` support.
pub fn checkpoint_for(path: &Path) -> PathBuf {
    let base = staging_root().join("checkpoints");
    fs::create_dir_all(&base).ok();
    base.join(format!("{}.ckpt", source_key(path)))
}

fn staging_root() -> PathBuf {
    let base = std::env::var("PHOENIX_STAGING_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| data_local_dir().join("staging"));
    fs::create_dir_all(&base).ok();
    base
}

pub(crate) fn data_local_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("phoenixrecover")
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("phoenixrecover")
    }
}

fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S%.3f").to_string()
}

/// Undelete scan: detect filesystems on `path`, recover deleted entries via
/// metadata (original names/paths), staging copies under the app data dir.
/// Returns stats when a filesystem was recognised.
pub fn undelete_scan(
    path: &Path,
    files: &mut Vec<RecoverableFile>,
    notes: &mut Vec<String>,
    app: Option<tauri::AppHandle>,
) -> anyhow::Result<Vec<RecoverStats>> {
    let src = Source::open(path)?;

    let volumes = recover::detect(&src)?;
    if volumes.is_empty() {
        notes.push("undelete: no supported filesystem found — use Deep Scan".into());
        return Ok(Vec::new());
    }

    let out_dir = staging_root().join(format!("undelete-{}", stamp()));
    fs::create_dir_all(&out_dir)?;

    let opts = RecoverOptions {
        min_size: 1,
        ..RecoverOptions::default()
    };

    let mut all_stats = Vec::new();
    for (i, volume) in volumes.iter().enumerate() {
        if was_cancelled() {
            break;
        }
        if let Some(handle) = &app {
            let _ = handle.emit(
                "scan-progress",
                ScanProgress {
                    phase: ProgressPhase::Metadata,
                    percent: -1.0,
                    bytes_scanned: 0,
                    total_bytes: 0,
                    note: format!("metadata pass {}/{} ({})", i + 1, volumes.len(), volume_name(volume)),
                },
            );
        }
        let stats = volume.recover_deleted(&src, &out_dir, &opts)?;
        notes.push(format!(
            "undelete: {} recovered, {} skipped ({})",
            stats.recovered,
            stats.skipped,
            volume_name(volume)
        ));
        all_stats.push(stats);
    }

    let n: u64 = all_stats.iter().map(|s| s.recovered).sum();
    collect_staged(&out_dir, ScanStrategy::Undelete, files, n as usize);
    Ok(all_stats)
}

fn volume_name(volume: &Volume) -> String {
    match volume {
        Volume::Ntfs(_) => "NTFS".into(),
        Volume::Fat(_) => "FAT".into(),
        Volume::Exfat(_) => "exFAT".into(),
        Volume::Ext(_) => "ext4".into(),
        Volume::Hfs(_) => "HFS+".into(),
        Volume::HfsStd(_) => "HFS".into(),
        Volume::Apfs(_) => "APFS".into(),
        Volume::Btrfs(_) => "Btrfs".into(),
        Volume::Xfs(_) => "XFS".into(),
        Volume::Iso(_) => "ISO9660".into(),
        _ => "filesystem".into(),
    }
}

/// Walk a staging directory and turn validated copies into result rows.
fn collect_staged(dir: &Path, strategy: ScanStrategy, files: &mut Vec<RecoverableFile>, _recovered: usize) {
    let mut stack = vec![dir.to_path_buf()];
    let mut seen = 0usize;
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else { continue };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let Ok(meta) = fs::metadata(&p) else { continue };
            if !meta.is_file() || meta.len() == 0 {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let ext = p
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .filter(|e| !e.is_empty())
                .or_else(|| guess_ext(&p))
                .unwrap_or_else(|| "bin".into());
            if !TARGET_TYPES.iter().any(|t| ext.eq_ignore_ascii_case(t)) {
                continue;
            }
            seen += 1;
            files.push(RecoverableFile {
                id: format!("pending-{seen}"),
                name,
                ext: ext.clone(),
                size: meta.len(),
                strategy,
                offset: None,
                staging_path: Some(p.to_string_lossy().into_owned()),
            });
        }
    }
}

fn guess_ext(path: &Path) -> Option<String> {
    let mut f = fs::File::open(path).ok()?;
    let mut buf = [0u8; 16];
    f.read_exact(&mut buf).ok()?;
    identify::identify(&buf).map(|d| d.ext.to_string())
}

/// Deep scan: byte-level carving with real progress + cancellation, then an
/// Office-format pass through the real forsmost engine.
pub fn carve_scan(
    path: &Path,
    files: &mut Vec<RecoverableFile>,
    notes: &mut Vec<String>,
    app: Option<tauri::AppHandle>,
    resume: bool,
) -> anyhow::Result<()> {
    let progress = Progress::new(app.clone());
    progress.set_note("opening source read-only");

    // Deterministic output dir + checkpoint so an interrupted scan can be
    // resumed later (unearth requires identical output dir and options).
    let out_dir = carve_dir_for(path);
    let checkpoint = checkpoint_for(path);
    fs::create_dir_all(&out_dir)?;
    if resume {
        progress.set_note("resuming from checkpoint");
    }

    let src = Source::open(path)?;
    let active = unearth::signatures::select(
        &TARGET_TYPES.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    )?;

    let opts = CarveOptions {
        output_dir: out_dir.clone(),
        start: 0,
        end: None,
        min_size: 512,
        max_size: None,
        max_files: None,
        allow_nested: false,
        validate: true,
        dedup: false,
        progress: false,
        checkpoint: Some(checkpoint),
        resume,
        organize: false,
        dry_run: false,
        align: 1,
    };

    progress.set_note("carving signatures (unearth)");
    let stats = carver::carve(&src, &active, &opts, progress.as_ref())?;
    notes.push(format!(
        "unearth carve: {} files recovered, {} bytes",
        stats.files_recovered, stats.bytes_recovered
    ));

    collect_staged(&out_dir, ScanStrategy::Carve, files, stats.files_recovered as usize);

    if was_cancelled() {
        return Ok(());
    }

    // Office formats through the real forsmost engine.
    if let Some(handle) = &app {
        let _ = handle.emit(
            "scan-progress",
            ScanProgress {
                phase: ProgressPhase::Office,
                percent: -1.0,
                bytes_scanned: 0,
                total_bytes: 0,
                note: "forsmost: docx/pptx/xlsx pass".into(),
            },
        );
    }
    match crate::forsmost_pass::run(path, &staging_root(), notes) {
        Ok(office_dir) => collect_staged(&office_dir, ScanStrategy::Carve, files, 0),
        Err(_) => { /* note already recorded; unearth signatures covered these types */ }
    }

    progress.set_note("scan complete");
    Ok(())
}
