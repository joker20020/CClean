# CClean

**English** | [简体中文](./README.zh-CN.md)

A cross-platform (Windows / macOS) disk cache cleaner and usage monitor built with **Tauri 2 + Rust + Vue 3 + ECharts**. It safely removes regenerable cache and temporary files — with special coverage for AI coding tools and AI desktop apps — and helps you understand what is consuming your system drive.

> Status: **M1–M4 complete** — rule engine, cache cleaning, AI model reports, large-file analysis, snapshot trends, tray daemon, desktop float widget, custom rules, NSIS installer and the auto-update channel are all in place.
>
> Full design: [doc/DESIGN.md](./doc/DESIGN.md) · Rule authoring: [RULES_CONTRIBUTING.md](./RULES_CONTRIBUTING.md) · Release & updates: [doc/RELEASE.md](./doc/RELEASE.md)

## Highlights

- **Declarative YAML rules** for six categories — System, Browsers, Dev tools, AI tools, Apps, GPU. Built-in rules live in `src-tauri/resources/rules/`.
- **Dry-run first**: every target is scanned and its reclaimable size shown before anything is deleted. Nothing is cleaned without an explicit, per-item confirmation.
- **Allow-list safety model**: a rule target must resolve inside a platform allow-list root; out-of-bounds rules are rejected at load time. Personal files, browser passwords/history and AI models are **report-only**.
- **AI tool cleanup**: Cursor, VS Code, Windsurf, Trae; Claude / ChatGPT desktop; Doubao desktop; Tencent WorkBuddy / CodeBuddy; ZCode (including the vendor-confirmed removable `cli/exec`); Claude Code, Codex, Copilot CLI; Bito; Tongyi Lingma; HuggingFace partial downloads; torch extensions — while conversation history, skills, configuration and login state are never touched.
- **AI models are report-only**: Ollama (auto-runs `ollama list`), HuggingFace / ModelScope / LM Studio / Torch Hub model stores — with per-folder breakdowns, "reveal in folder" and official removal instructions.
- **Large files & folders**: files ≥ 100 MB / 500 MB / 1 GB across your home folders (size, modified time, reveal); "Locate" opens the containing folder **with the file selected**; any folder can be visualised as a first-level treemap.
- **Platform features**: Windows Update cache cleanup stops/starts `wuauserv` and BITS and restores their prior state; macOS supports `brew cleanup -s` (dry-run first) and Xcode/simulator caches.
- **Snapshots & trends**: a local SQLite database (`cclean.db` in the app data directory, no network requests) records system-drive capacity and per-rule usage; a baseline snapshot is taken on first launch; the Trends page shows capacity curves, category growth and cleanup history.
- **Optional tray daemon (off by default)**: tray menu (snapshot now / open / quit), scheduled snapshots (6/12/24/48 h), low-space system notifications (checked at startup; at most once per hour by default; the dashboard banner can snooze for 12 h and then automatically reverts to the 1-hour cadence), close-to-tray and launch-at-login.
- **Cache directory watcher**: watches only the directories registered by rules (up to 80, 2-second debounce). Create/write/delete events are stored and streamed live to the Trends page — no full-disk hooks.
- **Desktop float widget (optional)**: a transparent always-on-top capsule showing live free space; draggable, snaps to screen edges, supports click-through, and persists position/state. Built-in actions: one-click clean (with confirmation), open main window, hide.
- **Custom rules**: write YAML directly in the Rules page (built-in rules are read-only). Mandatory allow-list validation runs before saving; files live in the app data directory `rules/` and take effect immediately.
- **Installer & auto-update**: per-user Windows NSIS installer with a built-in updater; check for updates with one click in Settings (release process in [doc/RELEASE.md](./doc/RELEASE.md)).
- **Graceful skipping**: locked or permission-denied files are skipped automatically — processes are never force-killed.
- **Disk overview**: system-drive gauge plus per-partition usage bars.

## Documentation

- [doc/DESIGN.md](./doc/DESIGN.md) — full architecture and security model
- [RULES_CONTRIBUTING.md](./RULES_CONTRIBUTING.md) — cleanup-rule YAML format, allow-list and contribution workflow
- [doc/RELEASE.md](./doc/RELEASE.md) — packaging, signing and the GitHub Releases auto-update flow

## Developing

Prerequisites: stable Rust, Node 18+ and pnpm. On Windows you also need WebView2 (preinstalled on Windows 10/11) and the MSVC build tools.

```powershell
pnpm install
pnpm tauri dev      # run in development
```

Other commands:

```powershell
# Frontend type-check + build
pnpm build

# Rust checks / unit tests (rule validation, glob matching)
cd src-tauri
cargo check
cargo test

# Headless smoke test: real (read-only) scan of selected cache directories
cargo run --example smoke

# Build the installer
pnpm tauri build
```

## Safety boundaries

- Only regenerable caches inside the rule allow-list are ever deleted. Irreversible/system items such as the Recycle Bin, `Windows.old`, restore points and WinSxS are report-only, with links to the official system tools.
- L1 rules delete permanently by default (a confirmation dialog is always shown); recycle-bin mode is available per cleanup.
- Everything runs locally. There are no network requests and no telemetry.

## License

This project is currently distributed without an explicit open-source license. Add an `LICENSE` file before publishing derivatives.
