# Packaging, Signing and Auto-Update

The auto-update channel is hosted on **GitHub Releases** of
[`joker20020/CClean`](https://github.com/joker20020/CClean). The app fetches a
`latest.json` manifest from the **latest** release and verifies every download
with a minisign signature.

## 1. Local build

Prerequisites: stable Rust, Node 22+, pnpm. On Windows the NSIS toolchain is
downloaded automatically by Tauri on first build (network required).

```powershell
# Windows: build only the NSIS installer (recommended)
$env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.tauri\cclean.key"
pnpm tauri build --bundles nsis
# Output: src-tauri/target/release/bundle/nsis/CClean_0.1.0_x64-setup.exe
# Updater signature (Tauri 2 signs the installer itself):
#   CClean_0.1.0_x64-setup.exe.sig
```

```bash
# macOS: dmg (must be built on macOS)
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/cclean.key"
pnpm tauri build --bundles dmg
```

- The installer uses **per-user install** (`installMode: currentUser`); no administrator rights are needed.
- The WebView2 bootstrapper is downloaded on demand; Windows 11 machines with WebView2 preinstalled need no extra step.
- The Start Menu shortcut carries an AUMID so system toast notifications (low-space alerts, etc.) work. Toasts from unpackaged development builds may be silently dropped by Windows.

## 2. Code signing (required before broad release)

Current builds are **unsigned**, so Windows SmartScreen shows a first-run warning. This is expected and does not affect functionality.

Before public release:

- **Windows**: obtain an OV/EV Authenticode code-signing certificate and rebuild with the certificate environment configured. Note that `TAURI_SIGNING_PRIVATE_KEY` signs the **update bundle** (minisign) — it is separate from, and in addition to, the Authenticode certificate. See the official Tauri NSIS signing docs for `TAURI_CERTIFICATE_SHA256` / `TAURI_CERTIFICATE_PASSWORD`.
- **macOS**: an Apple Developer ID is required; codesign and notarize both the `.app` and the dmg, otherwise users cannot open the app directly.

## 3. Updater configuration (already wired)

`src-tauri/tauri.conf.json` contains:

```json
"plugins": {
  "updater": {
    "pubkey": "<minisign public key>",
    "endpoints": [
      "https://github.com/joker20020/CClean/releases/latest/download/latest.json"
    ],
    "dialog": false
  }
}
```

- The key pair lives at `~/.tauri/cclean.key` (private — **never commit it**) and `cclean.key.pub` (public; its content is embedded as `pubkey` above).
- `releases/latest/download/latest.json` always resolves to the asset of the newest non-draft, non-prerelease release.

## 4. Release procedure

1. Bump the version in **both** `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` (and `package.json` if desired).
2. Build with `TAURI_SIGNING_PRIVATE_KEY` set (see section 1).
3. Read the signature:

   ```powershell
   $sig = Get-Content "src-tauri/target/release/bundle/nsis/CClean_<ver>_x64-setup.exe.sig" -Raw
   ```

4. Create `latest.json` (on Tauri 2 the updater downloads the signed NSIS installer `.exe` directly):

   ```json
   {
     "version": "0.2.0",
     "notes": "What changed in this release",
     "pub_date": "2026-11-01T00:00:00Z",
     "platforms": {
       "windows-x86_64": {
         "signature": "<full contents of the .sig file>",
         "url": "https://github.com/joker20020/CClean/releases/download/v0.2.0/CClean_0.2.0_x64-setup.exe"
       }
     }
   }
   ```

5. Publish the GitHub Release `v<ver>` and upload three assets:
   - `CClean_<ver>_x64-setup.exe` (installer and updater payload)
   - `CClean_<ver>_x64-setup.exe.sig` (signature, for reference)
   - `latest.json` (manifest consumed by the endpoint above)

   With the GitHub CLI:

   ```powershell
   gh release create v0.2.0 `
     CClean_0.2.0_x64-setup.exe `
     CClean_0.2.0_x64-setup.exe.sig `
     latest.json `
     --title "v0.2.0" --notes "Release notes"
   ```

6. Users click **Settings → Check for updates**; the update downloads and installs (the app restarts afterwards).

> Key hygiene: if the private key leaks, rotate the keypair immediately (installed clients trust only the old public key, so one manual-update release is required). If it is lost, automatic updates can no longer be signed for existing clients — back it up securely.
