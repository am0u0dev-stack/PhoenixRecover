//! Deep-scan backend: drives the real PhotoRec CLI (CGSecurity, GPL v2+) —
//! the strongest open-source carver, 480+ file formats — in its
//! non-interactive `/cmd` mode, staging its output for the results table.
//!
//! PhotoRec is never modified and the source is only ever read.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};

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
            // Tauri `externalBin` installs `bin/photorec-<triple>.exe` with the
            // triple stripped, i.e. `photorec.exe` next to the app executable;
            // the DLL resources land under `<exe dir>/bin/`. Try every layout.
            let dirs = [dir.to_path_buf(), dir.join("bin")];
            let names = [
                "photorec",
                "photorec.exe",
                "photorec_win.exe",
                "photorec-x86_64-pc-windows-msvc.exe",
            ];
            for name in names {
                for base in &dirs {
                    let sidecar = base.join(name);
                    if sidecar.is_file() {
                        return Some(sidecar);
                    }
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

    // PhotoRec's /cmd mode checks for a terminal and silently exits without
    // carving when stdin/stdout are pipes (verified on 7.2: exit 0, empty
    // report). A pseudo-terminal makes it run identically to the interactive
    // build on every platform (Unix pty / Windows ConPTY).
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 50,
            cols: 200,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| anyhow::anyhow!("pty open failed: {e}"))?;
    let mut command = CommandBuilder::new(&bin);
    // The bundled PhotoRec is a cygwin build whose DLLs ship as Tauri
    // resources under `<app>/bin/`. Windows DLL search consults PATH, so
    // prepend the app dir, that resource dir and the binary's own dir.
    let mut search_dirs: Vec<PathBuf> = Vec::new();
    if let Some(parent) = bin.parent() {
        search_dirs.push(parent.to_path_buf());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            search_dirs.push(dir.to_path_buf());
            search_dirs.push(dir.join("bin"));
        }
    }
    if let Ok(existing) = std::env::join_paths(
        search_dirs
            .iter()
            .cloned()
            .chain({
                let raw = std::env::var_os("PATH").unwrap_or_default();
                std::env::split_paths(&raw).collect::<Vec<PathBuf>>()
            }),
    ) {
        command.env("PATH", existing);
    }
    command.arg("/d");
    command.arg(&base);
    command.arg("/cmd");
    command.arg(input);
    command.arg(&script);
    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|e| anyhow::anyhow!("failed to launch PhotoRec: {e}"))?;
    drop(pair.slave);

    // Drain the pty on a thread so PhotoRec never blocks on a full buffer;
    // poll the child so the app's cancel flag can stop the scan.
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| anyhow::anyhow!("pty reader failed: {e}"))?;
    let pump = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        buf
    });

    let mut cancelled = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(e) => {
                let _ = pump.join();
                return Err(anyhow::anyhow!("PhotoRec failed: {e}"));
            }
        }
        if crate::engine::was_cancelled() {
            cancelled = true;
            let _ = child.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    drop(pair.master);
    let _ = pump.join();
    if cancelled {
        notes.push("PhotoRec: scan cancelled by user".into());
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
