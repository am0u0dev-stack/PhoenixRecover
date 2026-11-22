//! PhoenixRecover — read-only data recovery (Tauri v2 backend).
//!
//! Commands: `list_sources`, `scan_drive`, `recover_files`, `cancel_scan`.
//! All source access is strictly read-only; see `read_only.rs`.

mod engine;
mod forsmost_pass;
mod read_only;
mod store;
mod types;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use types::{CheckpointStatus, FilePreview, HexRow, RecoverableFile, RecoveryReport, ScanStrategy, SourceInfo, SourceKind, DeviceType};

/// Monotonic id source for result rows.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Scan results from the last completed scan, keyed by file id.
static LAST_SCAN: Mutex<Option<Vec<RecoverableFile>>> = Mutex::new(None);

#[tauri::command]
fn list_sources() -> Vec<SourceInfo> {
    let mut out = Vec::new();

    // Block devices (Linux): /dev/sd*, /dev/nvme*, /dev/mmcblk*, /dev/hd*
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = fs::read_dir("/dev") {
            let mut devs: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    let n = p.to_string_lossy();
                    (n.starts_with("/dev/sd")
                        || n.starts_with("/dev/nvme")
                        || n.starts_with("/dev/mmcblk")
                        || n.starts_with("/dev/hd"))
                        && !n.contains(char::is_numeric).then_some(true).unwrap_or(false)
                        || n == "/dev/sda"
                        || n == "/dev/nvme0n1"
                })
                .collect();
            devs.sort();
            for dev in devs {
                let name = dev.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                // Skip partition suffixes (sda1, nvme0n1p2) — scan whole devices.
                if name.ends_with(char::is_numeric) {
                    continue;
                }
                let size = device_size_bytes(&dev).unwrap_or(0);
                out.push(SourceInfo {
                    name: format!("Drive {}", name),
                    path: dev.to_string_lossy().into_owned(),
                    kind: SourceKind::Drive,
                    size_bytes: size,
                    device_type: guess_device_type(&dev),
                    read_only: true,
                });
            }
        }
    }

    // Mounted removable media roots make quick scans approachable on Linux.
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = fs::read_dir("/media") {
            for e in entries.flatten().take(8) {
                let p = e.path();
                if p.is_dir() {
                    out.push(SourceInfo {
                        name: e.file_name().to_string_lossy().into_owned(),
                        path: p.to_string_lossy().into_owned(),
                        kind: SourceKind::Folder,
                        size_bytes: 0,
                        device_type: DeviceType::Usb,
                        read_only: true,
                    });
                }
            }
        }
    }

    // Windows drive letters.
    #[cfg(target_os = "windows")]
    {
        for letter in 'C'..='Z' {
            let root = format!("{letter}:\\");
            let p = Path::new(&root);
            if p.is_dir() {
                let device_type = guess_device_type(p);
                out.push(SourceInfo {
                    name: format!("Drive {letter}"),
                    path: root,
                    kind: SourceKind::Drive,
                    size_bytes: 0,
                    device_type,
                    read_only: true,
                });
            }
        }
    }

    out
}

#[cfg(target_os = "linux")]
fn device_size_bytes(dev: &Path) -> Option<u64> {
    let name = dev.file_name()?.to_string_lossy().into_owned();
    let sysfs = Path::new("/sys/block").join(&name).join("size");
    let txt = fs::read_to_string(sysfs).ok()?;
    txt.trim().parse::<u64>().ok().map(|sectors| sectors * 512)
}

fn guess_device_type(path: &Path) -> DeviceType {
    let s = path.to_string_lossy().to_lowercase();
    if s.contains("mmcblk") {
        DeviceType::Sd
    } else if s.contains("usb") || s.contains("sd") && !s.contains("sda") {
        DeviceType::Usb
    } else if s.contains("nvme") {
        DeviceType::Ssd
    } else {
        DeviceType::Unknown
    }
}

/// Scan a drive, folder or disk image. `deep` selects carving mode over
/// undelete. Sources are opened read-only (see `read_only.rs`).
#[tauri::command]
fn scan_drive(
    app: tauri::AppHandle,
    path: String,
    deep: bool,
    resume: bool,
) -> Result<Vec<RecoverableFile>, String> {
    engine::reset_cancel();
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("source does not exist: {path}"));
    }

    let mut files: Vec<RecoverableFile> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let handle = Some(app);

    if p.is_dir() {
        scan_folder(&p, deep, resume, &mut files, &mut notes, handle)?;
    } else if deep {
        engine::carve_scan(&p, &mut files, &mut notes, handle, resume)
            .map_err(|e| format!("deep scan failed: {e}"))?;
    } else {
        match engine::undelete_scan(&p, &mut files, &mut notes, handle) {
            Ok(stats) => {
                if stats.is_empty() {
                    notes.push("no metadata candidates — try Deep Scan for signature carving".into());
                }
            }
            Err(e) => notes.push(format!("undelete scan failed: {e}")),
        }
    }

    // Assign final ids and remember the staging paths for recovery.
    let mut result = Vec::with_capacity(files.len());
    let mut store = LAST_SCAN.lock().unwrap();
    store.get_or_insert_with(Vec::new);
    for mut f in files {
        let id = format!("f-{}", NEXT_ID.fetch_add(1, Ordering::SeqCst));
        f.id = id.clone();
        result.push(f);
    }
    *store = Some(result.clone());

    let total_bytes: u64 = result.iter().map(|f| f.size).sum();
    store::record_scan(&path, deep, result.len(), total_bytes);

    Ok(result)
}

/// Folder source: identify files by signature (quick) and/or carve inside
/// them (deep). The folder itself is only ever read.
fn scan_folder(
    dir: &Path,
    deep: bool,
    resume: bool,
    files: &mut Vec<RecoverableFile>,
    notes: &mut Vec<String>,
    app: Option<tauri::AppHandle>,
) -> Result<(), String> {
    let _ = app;
    let mut count = 0usize;
    visit_files(dir, &mut |p, meta| {
        let mut buf = [0u8; 16];
        let mut f = match fs::File::open(p) {
            Ok(f) => f,
            Err(_) => return,
        };
        use std::io::Read;
        if f.read_exact(&mut buf).is_err() {
            return;
        }
        let detected = unearth::identify::identify(&buf);
        if let Some(d) = detected {
            if engine::TARGET_TYPES.iter().any(|t| d.ext.eq_ignore_ascii_case(t)) {
                let id = format!("f-{}", NEXT_ID.fetch_add(1, Ordering::SeqCst));
                files.push(RecoverableFile {
                    id,
                    name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                    ext: d.ext.to_string(),
                    size: meta.len(),
                    strategy: ScanStrategy::Undelete,
                    offset: None,
                    staging_path: Some(p.to_string_lossy().into_owned()),
                });
                count += 1;
            }
        }
    });

    notes.push(format!("folder scan: {count} recognisable files indexed (read-only)"));

    if deep {
        // Carve the folder for residual/embedded data beyond what signatures
        // show at offset 0: each file is carved through the same engine.
        engine::carve_scan(dir, files, notes, None, resume)
            .map_err(|e| format!("deep scan failed: {e}"))?;
    }
    Ok(())
}

fn visit_files(dir: &Path, cb: &mut dyn FnMut(&Path, &fs::Metadata)) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            match fs::metadata(&p) {
                Ok(m) if m.is_dir() => stack.push(p),
                Ok(m) if m.is_file() => cb(&p, &m),
                _ => {}
            }
        }
    }
}

/// Recover the selected files (by id from the last scan) into `output_dir`.
#[tauri::command]
fn recover_files(files: Vec<String>, output_dir: String) -> Result<RecoveryReport, String> {
    let store = LAST_SCAN.lock().unwrap();
    let last = store.as_ref().ok_or("no scan results — run a scan first")?;

    let out = PathBuf::from(&output_dir);
    fs::create_dir_all(&out).map_err(|e| format!("cannot create output dir: {e}"))?;

    let mut report = RecoveryReport {
        recovered: 0,
        failed: 0,
        bytes_recovered: 0,
        by_type: Default::default(),
        output_dir: out.to_string_lossy().into_owned(),
        notes: vec![
            format!("recovered from PhoenixRecover — {}", read_only::GUARANTEE),
        ],
    };

    for id in &files {
        let Some(f) = last.iter().find(|f| &f.id == id) else {
            report.failed += 1;
            continue;
        };
        let Some(src) = f.staging_path.as_ref() else {
            report.failed += 1;
            continue;
        };
        // Carved staging files already carry ext in their name; keep it.
        let safe_name = sanitize_name(&f.name, &f.ext);
        let dest = unique_target(&out, &safe_name);
        match fs::copy(src, &dest) {
            Ok(n) => {
                report.recovered += 1;
                report.bytes_recovered += n;
                *report.by_type.entry(f.ext.to_lowercase()).or_insert(0) += 1;
            }
            Err(_) => report.failed += 1,
        }
    }

    store::record_recovery(
        last.first().map(|f| f.name.as_str()).unwrap_or("unknown"),
        report.recovered,
        report.bytes_recovered,
        &report.output_dir,
    );

    Ok(report)
}

/// Recent scan/recovery activity from the history database.
#[tauri::command]
fn scan_history() -> Vec<store::HistoryEntry> {
    store::history(20)
}

/// Check whether an interrupted deep scan left a resumable checkpoint for
/// this source. The frontend surfaces a "Resume deep scan" action when true.
#[tauri::command]
fn checkpoint_status(path: String) -> CheckpointStatus {
    let p = PathBuf::from(&path);
    let cp = engine::checkpoint_for(&p);
    let out = engine::carve_dir_for(&p);
    CheckpointStatus {
        exists: cp.exists() && out.exists(),
        checkpoint_path: cp.to_string_lossy().into_owned(),
        output_dir: out.to_string_lossy().into_owned(),
    }
}

/// Read-only hex preview of a file (staged copy or folder source file):
/// first 512 bytes as 16-byte rows with ASCII gutter. Never writes anything.
#[tauri::command]
fn file_hex_preview(path: String) -> Result<FilePreview, String> {
    use std::io::Read;

    let p = PathBuf::from(&path);
    let meta = fs::metadata(&p).map_err(|e| format!("cannot stat file: {e}"))?;
    if meta.is_dir() {
        return Err("cannot preview a directory".into());
    }
    const MAX: u64 = 512;
    let mut f = fs::File::open(&p).map_err(|e| format!("cannot open read-only: {e}"))?;
    let take = MAX.min(meta.len());
    let mut buf = vec![0u8; take as usize];
    f.read_exact(&mut buf)
        .map_err(|e| format!("read failed: {e}"))?;

    let mut rows = Vec::with_capacity((buf.len() + 15) / 16);
    for (i, chunk) in buf.chunks(16).enumerate() {
        let ascii = chunk
            .iter()
            .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '·' })
            .collect();
        rows.push(HexRow {
            offset: (i * 16) as u64,
            hex: chunk.to_vec(),
            ascii,
        });
    }
    Ok(FilePreview {
        path: p.to_string_lossy().into_owned(),
        size: meta.len(),
        truncated: meta.len() > MAX,
        rows,
    })
}

fn sanitize_name(name: &str, ext: &str) -> String {
    let stem = name
        .split(['/', '\\'])
        .last()
        .unwrap_or(name)
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();
    if stem.to_lowercase().ends_with(&ext.to_lowercase()) {
        stem
    } else {
        format!("{stem}.{ext}")
    }
}

fn unique_target(dir: &Path, name: &str) -> PathBuf {
    let mut target = dir.join(name);
    if !target.exists() {
        return target;
    }
    let stem = Path::new(name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = Path::new(name).extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    for i in 1..10_000 {
        let candidate = if ext.is_empty() { format!("{stem}-{i}") } else { format!("{stem}-{i}.{ext}") };
        let t = dir.join(candidate);
        if !t.exists() {
            target = t;
            break;
        }
    }
    target
}

/// Ask the running scan to stop at the next progress checkpoint.
#[tauri::command]
fn cancel_scan() -> Result<(), String> {
    engine::CANCEL.store(true, Ordering::SeqCst);
    Ok(())
}

fn main() {
    run();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_sources,
            scan_drive,
            recover_files,
            cancel_scan,
            scan_history,
            checkpoint_status,
            file_hex_preview
        ])
        .run(tauri::generate_context!())
        .expect("error while running PhoenixRecover");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_extension() {
        assert_eq!(sanitize_name("photo (1).jpg", "jpg"), "photo__1_.jpg");
        assert_eq!(sanitize_name("doc.docx", "docx"), "doc.docx");
        assert_eq!(sanitize_name("carved-0001", "png"), "carved-0001.png");
    }

    #[test]
    fn unique_target_avoids_collision() {
        let dir = tempfile::tempdir().unwrap();
        let a = unique_target(dir.path(), "x.txt");
        fs::write(&a, b"").unwrap();
        let b = unique_target(dir.path(), "x.txt");
        assert_ne!(a, b, "second target must not overwrite the first");
        assert!(a.exists() && !b.exists());
    }
}
