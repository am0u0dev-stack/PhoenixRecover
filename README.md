# PhoenixRecover

Professional, read-only data recovery for the desktop. Undelete via filesystem
metadata (NTFS, FAT, exFAT, ext4, HFS+, APFS, …) or carve byte-level signatures
across 150+ formats — jpg, png, pdf, docx, pptx, xlsx, zip, rar, mp4, mov and
more — powered by the real [`unearth`](https://crates.io/crates/unearth) and
[`forsmost`](https://crates.io/crates/forsmost) engines.

Sources: HDDs, SSDs, USB flash drives, SD cards, disk images and specific
folders. You pick exactly which files to recover; nothing is written back to
the source — ever.

## Read-only guarantee

Every source is opened with `O_RDONLY` (see `src-tauri/src/read_only.rs`).
Scan candidates are staged under the app's data directory; only files you
explicitly confirm are copied to your chosen output folder.

## SSD warning (TRIM)

When you select an SSD, PhoenixRecover shows a prominent warning: TRIM erases
deleted blocks within seconds, making recovery nearly impossible. If the data
is critical, power the device off immediately and use a Live USB environment
with read-only mounting.

## Installers

Build artifacts for both platforms (from the repo root):

| OS | Command | Output |
|----|---------|--------|
| Windows | `npm run tauri build` | `.msi` (WiX) and `-setup.exe` (NSIS) in `src-tauri/target/release/bundle/{msi,nsis}` |
| Linux | `npm run tauri build` | `.deb`, `.rpm`, `.AppImage` in `src-tauri/target/release/bundle/{deb,rpm,appimage}` |

`tauri.conf.json` already declares both bundle target sets
(`targets: ["deb", "rpm", "appimage", "msi", "nsis"]`).

### Code signing

Releases are unsigned by default; here is how to sign each artifact type.

**Windows (recommended before distribution):**
- **Authenticode / EV certificate**: sign both the `.exe` and the `.msi` with `signtool`:
  ```sh
  signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /f cert.pfx /p <pass> PhoenixRecover-setup.exe
  ```
- For automated signing in CI, set the `WINDOWS_CERTIFICATE_THUMBPRINT` and
  `WINDOWS_CERTIFICATE_NAME` secrets and add a signtool step after `tauri build`.
- Tauri's own updater signature (optional, only needed for the update feed):
  generate with `npm run tauri signer generate`, store the private key in the
  `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets —
  the release workflow already passes them through.
- Unsigned installers trigger SmartScreen's "unknown publisher" warning; an EV
  certificate removes it. There is no way to distribute a trusted Windows
  installer without a certificate.

**Linux:**
- No kernel-level code signing is required for `.deb`/`.rpm`/`.AppImage`.
- The `.AppImage` can be signed (optional) with `appimage-sign`/GPG:
  ```sh
  gpg --detach-sign --armor PhoenixRecover_0.1.0_amd64.AppImage
  ```
  Distribute the `.sig` alongside; verify with `gpg --verify`.
- Repository distribution: sign the apt repository metadata (`apt-ftparchive`
  + `gpg --clearsign Release`); rpm repos use `rpm --addsign` with a GPG key
  users import into `rpm --import`.

**macOS:** not targeted by this project; skip.

## Prerequisites

### Windows
- Rust toolchain (`rustup`, stable-msvc), `cargo`
- Node.js 20+ and npm
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with the **C++ workload**
- WebView2 (preinstalled on Windows 10/11; the installer bootstraps it if missing)
- WiX/NSIS tooling is fetched automatically by Tauri on first build

### Linux

Debian/Ubuntu:
```sh
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev libgtk-3-dev file patchelf
```

Fedora:
```sh
sudo dnf install -y webkit2gtk4.1-devel gcc gcc-c++ openssl-devel \
  libayatana-appindicator3-devel librsvg2-devel file patchelf
```

Arch:
```sh
sudo pacman -S --needed webkit2gtk-4.1 base-devel openssl \
  libayatana-appindicator librsvg file patchelf
```

Then `npm install` and build with the table above.

## First run & permissions

- **Folders and disk images** need no special privileges — start there.
- **Raw block devices** (`/dev/sdX`, `/dev/nvme0n1`, physical disks) need
  elevated read access:
  - **Linux:** the `.deb` ships both rules automatically — a udev rule at
    `/usr/lib/udev/rules.d/99-phoenixrecover.rules` (read-only `0440` for
    `plugdev`) and a polkit policy at
    `/usr/share/polkit-1/actions/com.mlai.phoenixrecover.policy`. After
    installing the deb, add yourself to the group and reload udev:
    ```sh
    sudo usermod -aG plugdev "$USER"   # then log out/in
    sudo udevadm trigger
    ```
    Manual equivalent without the deb, or for sudo fallback:
    ```sh
    echo 'KERNEL=="sd*|nvme*|mmcblk*", GROUP="disk", MODE="0660"' | \
      sudo tee /etc/udev/rules.d/99-phoenixrecover.rules
    sudo udevadm control --reload && sudo udevadm trigger
    ```
    A polkit policy can grant read-only device access to specific users
    without full sudo.
  - **Windows:** run the app **as Administrator** for physical disk carving
    (raw `\\.\PhysicalDriveN` access is admin-only).
- The dedicated Office-format (docx/pptx/xlsx) carving pass uses the
  `forsmost` CLI, which is **bundled as a sidecar** in every installer (run
  `npm run sidecar` once before `npm run tauri build`, or let CI do it). When
  the sidecar is absent (e.g. a bare `cargo run` checkout), Office files are
  still carved through unearth's signatures and the scan notes the fallback.

## Development

```sh
npm install
npm run tauri dev    # desktop app with hot reload
npm run tauri build  # release installers
```

Engine tests (real carve + byte-identical recovery assertions):

```sh
cd src-tauri && cargo test
```
