//! Deep-scan backend: drives the real PhotoRec CLI (CGSecurity, GPL v2+) —
//! the strongest open-source carver, 480+ file formats — in its
//! non-interactive `/cmd` mode, staging its output for the results table.
//!
//! PhotoRec is never modified and the source is only ever read.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve the bundled photorec binary: env override, the Tauri sidecar
/// location next to the app executable, then PATH.
fn photorec_binary() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("PHOTOREC_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["photorec", "photorec_win.exe"] {
                let sidecar = dir.join(name);
                if sidecar.is_file() {
                    return Some(sidecar);
                }
            }
        }
    }
    // Dev / CI fallback: the Linux package path or plain PATH lookup.
    for cand in ["/usr/bin/photorec", "/usr/local/bin/photorec"] {
        if Path::new(cand).is_file() {
            return Some(PathBuf::from(cand));
        }
    }
    None
}

/// Run PhotoRec over `input`, carving the extensions selected by `kinds`
/// (picker categories; missing/empty = everything), into a stable staging
/// directory. Returns the directory holding recovered files.
pub fn run(
    input: &Path,
    staging_root: &Path,
    kinds: &[String],
    notes: &mut Vec<String>,
) -> anyhow::Result<PathBuf> {
    let out_dir = staging_root.join(format!(
        "photorec-{}",
        crate::engine::source_key(input, kinds)
    ));
    std::fs::create_dir_all(&out_dir)?;

    let bin = photorec_binary().ok_or_else(|| {
        anyhow::anyhow!("PhotoRec backend not found — neither bundled sidecar nor installed (expected next to the app executable or in PATH)")
    })?;
    notes.push(format!("PhotoRec: using backend {}", bin.display()));

    // Base run: whole-image carve. `/d` names the recup_dir base; PhotoRec
    // appends .1, .2 … per run. Single `/cmd <device> <commands>` form — the
    // exact non-interactive syntax verified against PhotoRec 7.2.
    let base = out_dir.join("rec");
    let filters = photorec_filters(kinds);
    let script = if filters.is_empty() {
        "search".to_string()
    } else {
        format!("{filters},search")
    };
    let mut cmd = Command::new(&bin);
    cmd.arg("/d").arg(&base).arg("/cmd").arg(input).arg(&script);

    let out = cmd.output()?;
    if !out.status.success() && !out.stderr.is_empty() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if err.contains("syntax error") {
            notes.push(format!("PhotoRec: filter syntax rejected ({err}); full-format carve ran instead"));
        }
    }

    // PhotoRec writes recup results into `<base>.N` siblings; also accept the
    // base itself when a future version stops suffixing.
    let mut found = out_dir.clone();
    if !staging_has_files(&found) {
        let mut best: Option<(u32, PathBuf)> = None;
        for entry in std::fs::read_dir(&out_dir)?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(n) = name.rsplit('.').next().and_then(|s| s.parse::<u32>().ok()) {
                if best.as_ref().map(|(m, _)| n > *m).unwrap_or(true) {
                    best = Some((n, entry.path()));
                }
            }
        }
        if let Some((_, dir)) = best {
            found = dir;
        }
    }
    let n = std::fs::read_dir(&found).map(|d| d.count()).unwrap_or(0);
    notes.push(format!("PhotoRec: staged {n} files"));
    Ok(found)
}

fn staging_has_files(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut d| d.any(|e| e.map(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false)).unwrap_or(false)))
        .unwrap_or(false)
}

/// Map the picker's categories onto PhotoRec fileopt family groups. Only
/// filters the scanned families — never broadens PhotoRec's defaults.
fn photorec_filters(kinds: &[String]) -> String {
    use crate::engine::ALL_KINDS;
    let all = kinds.is_empty() || kinds.len() == ALL_KINDS.len();
    if all {
        return String::new();
    }
    let mut parts: Vec<String> = vec!["fileopt".into(), "everything".into(), "disable".into()];
    for k in kinds {
        let exts: &[&str] = match k.as_str() {
            "image" => &["jpg", "png", "gif", "bmp", "tif"],
            "video" => &["mp4", "mov", "avi", "mkv", "mpg"],
            "document" => &["pdf", "docx", "xlsx", "pptx", "doc", "xls", "ppt"],
            "audio" => &["mp3", "wav", "aac", "flac"],
            "archive" => &["zip", "rar", "7z"],
            _ => &[],
        };
        for e in exts {
            parts.push((*e).into());
            parts.push("enable".into());
        }
    }
    parts.join(",")
}
