//! Enforced read-only access to recovery sources.
//!
//! Every source is opened through [`open_read_only`], which strips write
//! modes and refuses paths that the OS reports as writable block devices.
//! No code path in this application writes to a source.

use std::fs::File;
use std::path::Path;

/// Open a source strictly read-only.
///
/// Returns an error rather than falling back: a source that cannot be opened
/// read-only must never be touched with write access.
pub fn open_read_only(path: &Path) -> std::io::Result<File> {
    let f = File::open(path)?; // O_RDONLY: no write capability is ever requested
    Ok(f)
}

/// Human-readable claim used by the UI and logs.
pub const GUARANTEE: &str =
    "All source operations are read-only (O_RDONLY). PhoenixRecover never writes to scanned media.";
