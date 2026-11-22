//! Office-format carving through the real `forsmost` engine.
//!
//! The published `forsmost 0.1.1` crate has an empty library target (its
//! modules are private to its own binary), so it is driven through its real
//! CLI (`forsmost -i <input> -o <dir> -t docx,xlsx,pptx`). If the binary is
//! not installed, the caller falls back to unearth's Office signatures and a
//! note is recorded — no recovery logic is invented here.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Locate the `forsmost` CLI. Resolution order:
/// 1. `FORSMOST_BIN` env override (tests / power users)
/// 2. the Tauri sidecar shipped next to the app executable
/// 3. `forsmost` on PATH (dev checkouts)
fn forsmost_binary() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FORSMOST_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sidecar = dir.join("forsmost");
            if sidecar.is_file() {
                return Some(sidecar);
            }
        }
    }
    // Fall back to PATH via `Command::new("forsmost")` semantics.
    None
}

/// Run forsmost on `input` (device or image), staging docx/pptx/xlsx matches.
/// Returns the output directory on success.
pub fn run(input: &Path, staging_root: &Path, notes: &mut Vec<String>) -> anyhow::Result<PathBuf> {
    let out_dir = staging_root.join(format!("forsmost-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
    std::fs::create_dir_all(&out_dir)?;

    let mut cmd = match forsmost_binary() {
        Some(bin) => {
            notes.push(format!("forsmost: using bundled sidecar at {}", bin.display()));
            let mut c = Command::new(&bin);
            c.arg("-i").arg(input);
            c.arg("-o").arg(&out_dir);
            c.arg("-t").arg("docx,pptx,xlsx");
            c
        }
        None => {
            // Dev fallback: rely on PATH.
            let mut c = Command::new("forsmost");
            c.arg("-i").arg(input);
            c.arg("-o").arg(&out_dir);
            c.arg("-t").arg("docx,pptx,xlsx");
            c
        }
    };

    notes.push("forsmost: carving Office formats (docx, pptx, xlsx)".into());
    match cmd.output() {
        Ok(out) if out.status.success() => {
            notes.push("forsmost: Office pass completed".into());
            Ok(out_dir)
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            notes.push(format!(
                "forsmost: Office pass skipped ({})",
                stderr.lines().next().unwrap_or("forsmost unavailable")
            ));
            Err(anyhow::anyhow!("forsmost failed: {stderr}"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            notes.push(
                "forsmost CLI not found (sidecar or PATH) — Office carving fell back to unearth signatures"
                    .into(),
            );
            Err(anyhow::anyhow!("forsmost binary not installed"))
        }
        Err(e) => {
            notes.push(format!("forsmost: {e}"));
            Err(anyhow::anyhow!(e))
        }
    }
}
