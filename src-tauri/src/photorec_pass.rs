//! Deep-scan backend: drives the real PhotoRec CLI (CGSecurity, GPL v2+) —
//! the strongest open-source carver, 480+ file formats — in its
//! non-interactive `/cmd` mode, staging its output for the results table.
//!
//! PhotoRec is never modified and the source is only ever read.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};

use crate::engine::Progress;

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

/// Prepend the app dir, its `bin\` resource dir and the binary's own dir to
/// PATH so PhotoRec's cygwin DLLs resolve no matter which layout installed.
fn dll_search_path(bin: &Path) -> Option<std::ffi::OsString> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(parent) = bin.parent() {
        dirs.push(parent.to_path_buf());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
            dirs.push(dir.join("bin"));
        }
    }
    let raw = std::env::var_os("PATH").unwrap_or_default();
    dirs.extend(std::env::split_paths(&raw));
    std::env::join_paths(dirs).ok()
}

/// Run PhotoRec over `input`, carving the extensions selected by `kinds`
/// (picker categories; missing/empty = everything), into a stable staging
/// directory. Returns the directory holding recovered files.
pub fn run(
    input: &Path,
    staging_root: &Path,
    kinds: &[String],
    progress: Option<Arc<Progress>>,
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

    // Live progress: a monitor thread counts staged files/bytes while the
    // backend carves, so the UI's progress bar moves in real time.
    let stop = Arc::new(AtomicBool::new(false));
    let monitor = progress.clone().map(|p| {
        let dir = out_dir.clone();
        let stop = stop.clone();
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let (n, bytes) = count_staged(&dir);
                if n > 0 {
                    p.backend_progress(
                        None,
                        &format!("PhotoRec: {n} files ({bytes}) recovered so far"),
                    );
                }
                std::thread::sleep(Duration::from_millis(1200));
            }
        })
    });

    let mut cancelled = false;
    // PhotoRec's /cmd mode checks for a terminal and silently exits without
    // carving when stdin/stdout are pipes (verified on 7.2: exit 0, empty
    // report). A pseudo-terminal makes it run identically to the interactive
    // build on every platform (Unix pty / Windows ConPTY).
    let pty_result = (|| -> anyhow::Result<()> {
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
        if let Some(path) = dll_search_path(&bin) {
            command.env("PATH", path);
        }
        command.arg("/d");
        command.arg(&base);
        command.arg("/cmd");
        command.arg(input);
        command.arg(&script);
        let spawn = pair.slave.spawn_command(command);
        let mut child = match spawn {
            Ok(c) => c,
            Err(e) => {
                drop(pair.slave);
                drop(pair.master);
                // Windows: the bundled backend requires elevation (os error
                // 740). Relaunch it hidden through the shell's UAC path —
                // no visible terminal, no manual admin step needed.
                #[cfg(windows)]
                if e.to_string().contains("740") {
                    return run_elevated_windows(
                        &bin,
                        &base,
                        input,
                        &script,
                        &dll_dir(&bin),
                        &mut cancelled,
                        notes,
                    );
                }
                return Err(anyhow::anyhow!("failed to launch PhotoRec: {e}"));
            }
        };
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
        Ok(())
    })();
    stop.store(true, Ordering::Relaxed);
    if let Some(m) = monitor {
        let _ = m.join();
    }
    pty_result?;
    if cancelled {
        notes.push("PhotoRec: scan cancelled by user".into());
    }

    // PhotoRec writes recup results into `<base>.N` siblings; also accept the
    // base itself when a future version stops suffixing.
    let mut found = out_dir.clone();
    if staging_has_files(&found) {
        // keep base
    } else {
        let mut best: Option<(u32, PathBuf)> = None;
        for entry in std::fs::read_dir(&out_dir)?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(n) = name.rsplit('.').next_back().and_then(|s| s.parse::<u32>().ok()) {
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

/// Windows-only fallback: the bundled PhotoRec requires an elevated token
/// (CreateProcess fails with os error 740). Launch it through
/// ShellExecuteExW with the "runas" verb — the normal UAC path — hidden
/// (SW_HIDE, so no console window pops over the app) and wait on the
/// returned handle so cancellation still works.
#[cfg(windows)]
fn run_elevated_windows(
    bin: &Path,
    base: &Path,
    input: &Path,
    script: &str,
    workdir: &Path,
    cancelled: &mut bool,
    notes: &mut Vec<String>,
) -> anyhow::Result<()> {
    let args = format!(
        "\"/d\" \"{}\" /cmd \"{}\" {script}",
        base.display(),
        input.display()
    );
    let handle = launch_elevated_windows(bin, &args, workdir)?;
    notes.push("PhotoRec: running elevated (UAC approved)".into());
    loop {
        let done = unsafe {
            windows_sys::Win32::System::Threading::WaitForSingleObject(handle, 400)
        };
        if done == 0 {
            break; // WAIT_OBJECT_0
        }
        if crate::engine::was_cancelled() {
            *cancelled = true;
            unsafe {
                windows_sys::Win32::System::Threading::TerminateProcess(handle, 1);
            }
            break;
        }
    }
    unsafe {
        windows_sys::Win32::Foundation::CloseHandle(handle);
    }
    Ok(())
}

/// Directory holding PhotoRec's cygwin DLLs (the Tauri resource `bin\` folder
/// when installed, else the exe's own folder). Used as the elevated child's
/// working directory: across the UAC boundary our modified PATH is lost, and
/// the loader's cwd search is the only remaining way to find them.
fn dll_dir(bin: &Path) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            let b = d.join("bin");
            if b.is_dir() {
                return b;
            }
        }
    }
    bin.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
}

/// ShellExecuteExW("runas") returning the child's process handle. The
/// elevated child inherits the parent's environment? Not across UAC — so
/// PhotoRec's DLLs must sit in the exe's own directory (the loader's first
/// search location); the working directory is set there as a second net.
#[cfg(windows)]
fn launch_elevated_windows(
    bin: &Path,
    args: &str,
    workdir: &Path,
) -> anyhow::Result<windows_sys::Win32::Foundation::HANDLE> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let verb: Vec<u16> = "runas\0".encode_utf16().collect();
    let file: Vec<u16> = bin.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let params: Vec<u16> = args.encode_utf16().chain(std::iter::once(0)).collect();
    let dir: Vec<u16> = workdir.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    sei.lpVerb = verb.as_ptr();
    sei.lpFile = file.as_ptr();
    sei.lpParameters = params.as_ptr();
    sei.lpDirectory = dir.as_ptr();
    sei.nShow = SW_HIDE;
    if unsafe { ShellExecuteExW(&mut sei) } == 0 {
        return Err(anyhow::anyhow!(
            "elevated PhotoRec launch was cancelled or blocked — approve the UAC prompt to deep-scan"
        ));
    }
    Ok(sei.hProcess)
}

fn staging_has_files(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut d| d.any(|e| e.map(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false)).unwrap_or(false)))
        .unwrap_or(false)
}

/// Count staged recup output: (file count, human-readable byte total).
fn count_staged(out_dir: &Path) -> (u64, String) {
    let mut n = 0u64;
    let mut bytes = 0u64;
    if let Ok(dirs) = std::fs::read_dir(out_dir) {
        for dir in dirs.flatten() {
            let Ok(files) = std::fs::read_dir(dir.path()) else { continue };
            for f in files.flatten() {
                if let Ok(m) = f.metadata() {
                    if m.is_file() {
                        n += 1;
                        bytes += m.len();
                    }
                }
            }
        }
    }
    let pretty = if bytes >= 1 << 30 {
        format!("{:.1} GB", bytes as f64 / (1 << 30) as f64)
    } else if bytes >= 1 << 20 {
        format!("{:.1} MB", bytes as f64 / (1 << 20) as f64)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    };
    (n, pretty)
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
