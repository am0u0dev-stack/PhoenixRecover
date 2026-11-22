# PhoenixRecover — Plan

## Goal and users
A professional desktop data recovery tool (Tauri v2 + React + TS + Tailwind, Rust engine) for
people who lost files on HDDs, USB drives, SD cards, SSDs, disk images or a specific folder.
"Working" = select a source, run a real read-only scan, review a file list with checkboxes,
recover only the selected files to a chosen output directory. Monochrome premium UI.

## Scope
In scope: source selection (drives + folder), undelete scan (filesystem metadata via `unearth`),
deep scan / carving (via `unearth` + `forsmost` for Office formats), per-file selection,
recovery with report, read-only enforcement, progress events, SSD TRIM warning,
Liquid Gooey FAB, ThinkingOrb scan modal, dark/light monochrome themes,
Windows (.msi/.exe NSIS) and Linux (.deb/.AppImage/.rpm) build config and docs.

Out of scope for now: RAID/LVM assembly, cloud sources, hex preview of file contents,
scheduler, non-English UI.

## Architecture
- **Frontend**: React 19 + TypeScript + Vite + Tailwind v4. Components: `LiquidGooeyFab`
  (real `liquid-gooey` package), `ThinkingOrb` (real `thinking-orbs` package), scan progress
  modal, results table, SSD warning banner, settings dialog. Talks to backend only through
  Tauri commands (`invoke`) and events (`scan-progress`, `scan-done`).
- **Backend** (Rust, `src-tauri`): commands `list_sources`, `scan_drive(path, deep)`,
  `recover_files(files, output_dir)`, `cancel_scan`.
  - Undelete (quick scan): `unearth::recover::detect` + `Volume::recover_deleted` — real
    filesystem metadata parsing (NTFS, FAT, exFAT, ext4, HFS+, APFS, btrfs, …). Recovered
    candidates are written to an app-managed **staging** directory; the source is opened
    read-only (`unearth::source::Source` opens with `O_RDWR` off / read-only) and never written.
  - Carving (deep scan): `unearth::carver::carve` with `ProgressSink` → progress events and
    cancellation. Office formats (docx/xlsx/pptx) additionally go through the real `forsmost`
    engine; because the published `forsmost 0.1.1` crate has an empty library target
    (its modules are private to its binary), it is driven through its real CLI
    (`-t docx,xlsx,pptx -o <staging> <image>`); when the CLI is not installed, Office carving
    falls back to unearth's docx/xlsx/pptx/CFBF signatures and the scan notes this.
  - Folder sources: quick scan walks the folder and identifies files by signature
    (`unearth::identify`); deep scan carves inside each file to surface embedded/residual data.
- **Data model**: `SourceInfo`, `RecoverableFile` (id, name, ext, size, strategy, offset/path,
  staging path), `RecoveryReport` (recovered/failed/bytes, per-type). Staging lives under the
  app data dir; recovery copies selected staging files to the user's output dir.
- **Persistence**: no database — recovery state is file-based (staging dirs + report JSON);
  this is a desktop utility, not a web service.
- **Hosting**: desktop app. Bundles: msi/nsis (Windows), deb/rpm/appimage (Linux), configured
  in `tauri.conf.json`.

## Risks and assumptions
- `forsmost` library surface is empty (verified from crate source) → CLI integration +
  graceful fallback, documented in README.
- Raw block device scans need privileges: sudo/udev on Linux, Administrator on Windows
  (documented). Folder and disk-image scans work unprivileged.
- SSD TRIM makes recovery near-impossible; UI shows a prominent warning (spec requirement).
- In-browser preview cannot `invoke` Tauri commands; the UI degrades to a clear
  "backend unavailable" state. Real end-to-end is proven by Rust integration tests +
  `cargo check`/`tsc`/`vite build`.

## Phase 1: Vertical slice — real scan → select → recover
- [x] Tauri v2 project scaffold (Vite React TS + Tailwind, src-tauri workspace)
- [x] Read-only engines wired to real crates: `unearth` undelete + carve, `forsmost` CLI pass
- [x] Commands `list_sources` / `scan_drive` / `recover_files` / `cancel_scan` with progress events
- [x] Monochrome UI: source picker, scan modal with ThinkingOrb + %, results table with
      checkboxes, Recover Selected, SSD warning, Liquid FAB, theme toggle
- [x] Rust integration test: carve a synthetic image through the real engine and recover
      selected files end-to-end; `cargo check`, `tsc`, `vite build` pass
Acceptance: build the app, pick a folder or disk image, run Quick or Deep scan, see real
progress, tick files, recover them to an output folder and see the report.

## Phase 2: Instrument polish
- [x] File hex/preview pane, type/size filters, sort, virtualized results for 100k+ rows
- [x] Scan checkpoint/resume surfacing (unearth supports it), per-scan history
- [x] Light-theme fine tuning, icon set refresh
Acceptance: deep scan on a 1 GB image can be cancelled, resumed and filtered smoothly.

## Phase 3: Packaging and distribution
- [x] CI builds for msi/nsis/deb/rpm/AppImage; code signing notes
- [x] Bundled `forsmost` sidecar binary so Office carving needs no manual install
- [x] udev/polkit recipe shipped with the .deb for unprivileged raw-device reads
Acceptance: release artifacts attached to a GitHub release for both OSes.
Status: Linux bundles built locally (deb 3.71 MiB, rpm 3.72 MiB, AppImage
83.29 MiB) and verified to contain the sidecar + udev/polkit files; the
`release` workflow (.github/workflows/release.yml) builds the Windows
msi/nsis pairs and attaches everything to a draft GitHub release on `v*` tags
(run it by pushing a tag; signing needs the repo secrets noted in README).
