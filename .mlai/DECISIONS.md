# Decisions

## forsmost is driven through its CLI, not its library
The published `forsmost 0.1.1` crate has an empty library target (`src/lib.rs` only has
`#![doc]`; modules are private to its binary). PhoenixRecover therefore calls the real
`forsmost` CLI (`-i <input> -o <dir> -t docx,xlsx,pptx`) in
`src-tauri/src/forsmost_pass.rs`, and falls back to unearth's docx/xlsx/pptx signatures
with a scan note when the binary is missing. If a later forsmost release exposes a real
lib API, swap the CLI call for direct library use.

## unearth 0.4.0 API facts (verified from crate source)
- `Source::open(path)` opens read-only; `recover::detect(&Source) -> Vec<Volume>`;
  `Volume::recover_deleted(&src, &out_dir, &RecoverOptions)`; `RecoverStats.recovered/.skipped`.
- `CarveOptions` has no `Default` — all fields are set explicitly (see engine.rs).
- `carver::carve(&src, &signatures, &opts, &ProgressSink)`; progress via `begin/update/finish`.
