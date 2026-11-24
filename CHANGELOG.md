# Changelog

All notable changes to PhoenixRecover are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.9] — Unreleased

### Added

- Read-only enforcement layer (`src-tauri/src/read_only.rs`) — every source is
  opened with `O_RDONLY`
- Undelete (quick scan) via `unearth` filesystem metadata parsing
  (NTFS, FAT, exFAT, ext4, HFS+, APFS, btrfs, …)
- Deep scan / byte-level carving across 150+ file formats
- Office-format carving (`docx`, `xlsx`, `pptx`) through the `forsmost` CLI,
  bundled as an installer sidecar with a graceful unearth-signature fallback
- Live scan progress and cancellation via `scan-progress` / `scan-done` events
- Per-file selection with a recovery report (recovered / failed / bytes, per type)
- SSD TRIM warning banner
- Folder and disk-image sources
- Installers for Windows (`.msi`, `.exe` NSIS) and Linux (`.deb`, `.rpm`,
  `.AppImage`) via the `release` GitHub Actions workflow
- udev rule and polkit policy shipped with the `.deb` for unprivileged
  raw-device reads

### Security

- Read-only guarantee documented and enforced across all source paths
- `photorec.log` and `photorec.se2` are no longer committed to the repository
- Added `SECURITY.md` with a responsible-disclosure policy
