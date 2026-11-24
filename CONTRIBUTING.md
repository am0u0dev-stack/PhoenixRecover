## Contributing to PhoenixRecover

Thanks for your interest in improving PhoenixRecover. This project is a
**read-only** data recovery engine — every change must preserve that guarantee.

### The one rule that matters

> PhoenixRecover never writes to a source device.

If a change touches anything that opens a file, a block device, or an image,
it must open it `O_RDONLY`. Recovery output goes to a separate directory the
user explicitly chooses. Reviewers will reject any patch that can mutate a
source.

### Reporting bugs

Open an issue and include:

- OS, and whether the source was an HDD, SSD, USB drive, SD card or image
- The filesystem or format involved (NTFS, ext4, APFS, …)
- What you expected vs. what happened
- The relevant `photorec.log` excerpt, with the `photorec.se2` metadata if
  it is safe to share (it contains the scan report, not your files)

Never attach personal files. A redacted log is far more useful than a
private document.

### Development

```sh
npm install
npm run tauri dev    # desktop app with hot reload
npm run tauri build  # release installers
```

Rust engine tests — real carve + byte-identical recovery assertions:

```sh
cd src-tauri && cargo test
```

Please run `cargo test` and `cargo fmt` before opening a pull request.

### Pull requests

- One logical change per PR
- Describe *why*, not just *what*
- Note any behaviour change in the UI or the recovered-file layout
- Keep the read-only guarantee intact — mention it explicitly in the PR body

### Adding a file signature

New formats are added as signatures in the recovery engine. Prefer the
existing `unearth` signature set; only add a bespoke signature when the
format genuinely is not covered, and include a test that carves and verifies
a byte-identical recovery.

## Code of conduct

Be respectful. Assume good intent. Discussions happen in issues and pull
requests, not in personal attacks.
