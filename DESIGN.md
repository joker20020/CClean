# CClean Design Document

> Cross-platform (Windows / macOS) disk cleaner and usage monitor.
> Stack: Tauri 2 (Rust backend + web frontend, ECharts charts).
> Form factor: on-demand by default, with an optional background daemon (system tray, scheduled snapshots, directory watcher, translucent float widget).

---

## 1. Product goals

1. Stop the system drive (Windows `C:`, macOS system volume) from growing endlessly by safely deleting **regenerable caches and temporary files**.
2. Ship a dedicated **AI tool category**: caches of AI IDEs, AI desktop apps and AI development frameworks; AI **model files are report-only, never auto-deleted**.
3. Provide **category-level usage monitoring**: category usage, large-file / large-folder rankings, and growth trends based on historical snapshots.
4. Optional residency: scheduled monitoring, low-space alerts, a float widget showing live disk usage, and write/delete events on watched directories.

Non-goals: registry cleaning, system "boosting", software uninstallation, driver/service-level optimization.

## 2. Security model (highest priority)

### 2.1 Risk levels

| Level | Definition | Handling |
|---|---|---|
| L0 read-only | Scan, statistics, snapshot, watch | Runs directly, never modifies the system |
| L1 safe cleanup | Regenerable cache/temp files | Dry-run list → user checks items → clean |
| L2 system action | Requires admin / changes system state (e.g. stopping Windows Update services, `brew cleanup`) | Explicitly labelled, separately authorized, services restored afterwards |
| L3 forbidden | Irreversible or potentially damaging to system/data | Hard-coded denylist; cannot be enabled even by rule files |

### 2.2 Four iron rules

1. **Allow-list**: only targets explicitly declared by rule files are cleaned. At load time paths are normalized and boundary-checked; drive roots, the user home root, `Windows\System32`, `WinSxS`, `Windows\Installer`, `Program Files`, etc. are forbidden.
2. **Dry-run by default**: every cleanup first produces a scan report (reclaimable size, file count, risk level per item) and executes only after confirmation.
3. **Personal data & models are report-only**: Desktop/Documents/Downloads, received IM files, and AI model stores (Ollama / HuggingFace / ModelScope / LM Studio / Torch Hub …) are shown with size and an "open folder" action or official removal instructions — never auto-deleted.
4. **Full audit trail**: cleanups are logged (rule, path, size, time, result, failure reason); locked files are skipped, processes are never killed; the user is told to close running browsers.

### 2.3 Deletion strategy

- L1 defaults to **permanent deletion** (caches are regenerable; dry-run confirmation + logging always apply).
- Per rule / per cleanup the mode can switch between permanent deletion, Recycle Bin/Trash, and (in a later version) a quarantine folder.
- Irreversible actions (emptying the Recycle Bin, `Windows.old`, restore points) are **never executed automatically**; the UI only points to official system tools (cleanmgr, Storage Settings, `vssadmin` instructions).

### 2.4 Hard-coded denylist (L3)

- Windows: `C:\Windows\System32`, `C:\Windows\WinSxS` (DISM guidance only), `C:\Windows\Installer`, `C:\Program Files*`, the registry, `pagefile/hiberfil/swapfile.sys` (report + powercfg guidance), critical services and processes.
- macOS: `/System`, `/usr` (except Homebrew-managed parts), `/Library/Apple`, SIP-protected directories, system snapshots (report-only).
- Both platforms: browser Cookies / History / Bookmarks / Login Data / password stores; personal-directory files; security-software folders; AI models and training outputs.

## 3. Architecture

```
┌─────────────────────────────────────────────────────────┐
│ Frontend (Vue 3 + TS + ECharts, Tauri WebView)           │
│ Main window: Dashboard / Clean / AI / Big Files /        │
│              Trends / Rules / Settings                   │
│ Float widget (separate transparent window, optional)     │
├─────────────────────────────────────────────────────────┤
│ Tauri Commands (IPC) + Events (watcher stream)           │
├─────────────────────────────────────────────────────────┤
│ Rust core: rules / scanner / cleaner / analyzer /        │
│            watcher / snapshot(db) / settings / tray      │
├─────────────────────────────────────────────────────────┤
│ Platform layer: platform-windows / platform-macos        │
│ Trash · elevation · service control (Win) · notify ·     │
│ brew (mac)                                               │
├─────────────────────────────────────────────────────────┤
│ resources/rules/*.yaml (declarative, shipped, extensible)│
│ App data dir: SQLite snapshot DB, clean_log, settings    │
└─────────────────────────────────────────────────────────┘
```

### 3.1 Rust modules (`src-tauri/src/`)

| Module | Responsibility |
|---|---|
| `main.rs / lib.rs` | Tauri builder, plugin registration, window/tray initialization, single-instance guard |
| `rules/` | Rule model, YAML loading, variable expansion, glob parsing, safety validation (denylist/boundaries) |
| `scanner/` | Target discovery, parallel (rayon) directory sizing, availability/running detection, dry-run |
| `cleaner/` | Delete executor (permanent/recycle), service stop-restore (Win L2), result aggregation, failure collection |
| `analyzer/` | Large-file scan, directory-size ranking, disk overview |
| `watcher.rs` | `notify`-based rule-directory watcher (ReadDirectoryChangesW / FSEvents), debounced persistence and emit |
| `snapshot.rs` / `storage.rs` | SQLite: snapshots, snapshot items, event stream, cleanup logs |
| `resident.rs` | Settings, scheduler, low-space notifications, autostart |
| `floatwin.rs` | Float window control (transparent, always-on-top, click-through, edge snap, hide) |
| `privilege.rs` / `appdir.rs` | UAC elevation restart; app data directory creation & legacy-id migration |

### 3.2 Key dependencies (Cargo)

`tauri` 2, `tauri-plugin-{single-instance,opener,notification,autostart,updater}`, `serde / serde_yaml / serde_json`, `walkdir`, `rayon`, `notify`, `rusqlite (bundled)`, `trash`, `dirs`, `thiserror`, `chrono`, `sysinfo`, `glob`, `rust-embed`, `windows-sys`.

## 4. Cleanup rules (YAML)

Rule files live in `src-tauri/resources/rules/`, grouped by category: `system.yaml / browser.yaml / dev.yaml / ai.yaml / apps.yaml / gpu.yaml / report.yaml`.

### 4.1 Schema

```yaml
- id: system.temp.user            # globally unique, category.name
  name: User temp files           # UI label
  category: system                # system|browser|dev|ai|apps|gpu|report
  risk: L1                        # L1|L2; report rules are L0
  os: [windows, macos]            # target platforms
  default_selected: true          # pre-checked after dry-run
  delete: permanent               # permanent|recycle
  description: Temporary files created by programs; locked files are skipped
  targets:
    - paths: ["${TEMP}"]          # variables, see 4.2
      include: ["**/*"]           # globs, default **/*
      exclude: []                 # exclusion globs
      follow_symlinks: false
    - paths: ["${TMPDIR}"]
      os: [macos]
```

Report-only rules use the same structure plus `action: open_dir | guide`; `guide` carries explanatory text and official commands and shows no delete button.

### 4.2 Path variables

- Common: `${HOME}`, `${TMPDIR}`
- Windows: `${TEMP}`, `${WINDIR}`, `${LOCALAPPDATA}`, `${APPDATA}`, `${USERPROFILE}`, `${PROGRAMDATA}`
- macOS: `${HOME}/Library/Caches`, `${HOME}/Library/Logs`, `${HOME}/Library/Application Support`

At load time: variable expansion → absolute-path normalization → must reside inside an allow-list root (user cache dirs, system cache dirs, …). Otherwise the target is marked as a load error and disabled; it never executes.

## 5. Built-in cleanup coverage

### 5.1 Windows

| Category | L1 auto-clean targets | Report / guidance only |
|---|---|---|
| System | `%TEMP%`, `C:\Windows\Temp` (L2 elevated), Windows Update cache `SoftwareDistribution\Download` (L2, stops/restores wuauserv & BITS to prior state), Delivery Optimization cache, thumbnail cache `Explorer\thumbcache_*`, WER reports, `Windows\Logs`, `Minidump`, `MEMORY.DMP`, Prefetch (unchecked by default) | Recycle Bin, `Windows.old`, restore points (vssadmin guide), WinSxS (DISM guide), hiberfil/pagefile/swapfile (powercfg guide) |
| Browsers | `Cache`, `Code Cache`, `GPUCache` for Chrome/Edge/Brave/Firefox, Firefox `cache2`; enumerates every `Default` / `Profile *` | Cookies, History, Bookmarks, Login Data, keys (hard denylist); prompts to close running browsers |
| Dev | pip (`%LOCALAPPDATA%\pip\cache`), npm (`%LOCALAPPDATA%\npm-cache`), pnpm store, yarn, uv, cargo `registry\cache`, go-build, NuGet v3 cache, conda `pkgs`, JetBrains `caches/log` | `.m2`, `.gradle\caches`, `.nuget\packages` (regenerable but slow to re-download, unchecked); Docker/WSL vhdx (report + `docker builder prune`/compact guidance) |
| AI | **AI IDEs**: Cursor / VS Code / VSCodium / Windsurf / Trae `Cache`, `CachedData`, `Code Cache`, `GPUCache`, `logs`, `CachedExtensionVSIXs`; **AI desktop apps** (Claude/ChatGPT/Doubao/Kimi/Tongyi/DeepSeek and similar Electron apps) Cache families and logs; HuggingFace/ModelScope `*.incomplete` downloads; torch_extensions; Jupyter runtime | **Model stores report-only**: `~/.ollama/models` (with `ollama list/rm` guide), `.cache\huggingface\hub`, `.cache\modelscope`, LM Studio models, `.cache\torch\hub`, ComfyUI/A1111 `models` and `outputs`, AnythingLLM document store |
| Apps | WeChat/QQ **pure Cache folders** (e.g. `FileStorage\Cache`, exact glob matches), common video/music/cloud-drive app caches | Received files/images/videos/chat history (personal data, report-only) |
| GPU | NVIDIA `DXCache/GLCache/NV_Cache`, AMD/Intel shader caches, `D3DSCache`, Steam `htmlcache` | Game installations and saves |

### 5.2 macOS

| Category | L1 auto-clean targets | Report / guidance only |
|---|---|---|
| System | `~/Library/Caches` (user app caches), `~/Library/Logs`, `${TMPDIR}`, `~/Library/Logs/DiagnosticReports` | `/Library/Caches` (elevated, unchecked), Trash, system snapshots, iOS backups (`~/Library/Application Support/MobileSync/Backup`) |
| Pkg manager / dev | `brew cleanup -s` (dry-run first, L2 authorized), pip/npm/yarn/pnpm/uv/cargo caches, go-build, Xcode `DerivedData`, `iOS DeviceSupport` (unchecked) | Xcode Archives, Docker.raw (report + in-Docker cleanup guide), Maven/Gradle stores |
| Browsers | Cache folders of Chrome/Edge/Brave/Firefox/Safari (Safari: Caches only) | Same denylist as Windows |
| AI | Cache families of AI IDEs and Electron AI apps under `~/Library/Application Support/<App>/` and `~/Library/Caches/<App>`, HF/ModelScope partial downloads | `~/.ollama/models`, `~/.cache/huggingface`, LM Studio models, torch hub, ComfyUI models/outputs |
| Apps | Per-app caches under `~/Library/Caches` (shown per bundle, selectable) | Documents/Downloads/Desktop, photo libraries, received IM files |

### 5.3 Large files / folders (L0, both platforms)

- Files larger than a threshold (default 100 MB, adjustable) under home folders (Desktop, Downloads, Documents, Videos, `$HOME`): path, size, last-modified time, "reveal in folder".
- Top- and second-level directory size rankings of the system drive (Windows: Users/Windows/Program Files/ProgramData; macOS: /Applications, ~/Library, /Library, /System report-only).
- Report-only; all deletion is left to the user in the system file manager.

## 6. Monitoring, snapshots and residency

### 6.1 SQLite tables (`cclean.db` in the app data directory)

- `snapshots(id, created_at, disk_total, disk_used, disk_free)`
- `snapshot_items(id, snapshot_id, rule_id, path, size_bytes, file_count)`
- `fs_events(id, ts, kind(write|delete|create|rename), rule_id, path, size_delta)`
- `clean_logs(id, ts, rule_id, path, size_bytes, status, detail)`
- `settings(key, value)`

### 6.2 Resident components (off by default; enable in Settings)

1. **System tray**: snapshot now, open main window, float toggle, quit; double-click shows the window.
2. **Scheduled snapshots**: default once per day (configurable interval); silently scans rule paths and writes snapshots. Low-space threshold (default 10% free) raises a system notification — checked at startup, at most once per hour by default; the dashboard banner can snooze for 12 h, after which the 1-hour cadence automatically resumes.
3. **Directory watcher**: watches only rule-registered cache directories (`notify`, 2 s debounce); writes/delete events are persisted and pushed to the UI. **No full-disk hooks.**
4. **Launch at login**: `tauri-plugin-autostart`, needed only for residency.

### 6.3 Float widget (optional, off by default)

- Separate Tauri window: transparent, borderless, always-on-top, optional click-through, draggable, edge-snapping; capsule window 200×64 (56 px capsule body with padding for rounded corners and shadow).
- Content: system-drive free space plus a mini usage bar.
- Buttons: ① one-click clean (scans then cleans all default-safe rules, with an in-widget confirmation that auto-cancels after 4 s; freed space shown afterwards); ② open/focus main window; ③ hide the widget. Click-through remains in the tray menu and Settings page.
- Transparency is guaranteed by global CSS (transparent html/body/#app, zero body margin) so no white edge appears outside the rounded corners. No network data is collected; everything comes from local scans and watchers.

## 7. Tauri Commands (frontend/backend contract)

| Command | Description |
|---|---|
| `get_system_overview()` | Disk list (mount/total/used/free), OS, version, elevation status |
| `get_rules()` | Rule metadata plus target existence / current-platform availability |
| `scan(rule_ids: Option<Vec<String>>)` | Dry-run/real scan: size, file count, skipped count, status per rule |
| `clean(items, mode)` | Executes the confirmed list; returns freed bytes and failures; writes `clean_logs` |
| `list_big_files(dirs, min_bytes, limit)` | Large-file listing |
| `list_dir_sizes(path, depth)` | Directory-size ranking |
| `get_ai_report()` | AI caches plus model-store report (ollama, HF, …) |
| `get_brew_status()` / `run_brew_cleanup()` | macOS Homebrew status and cleanup |
| `take_snapshot()` / `get_trend(...)` / `get_clean_logs()` / `get_fs_events(...)` | Snapshots, trends, logs, watcher history; live events via Tauri event `fs-event` |
| `get_settings()` / `set_settings(updates)` | Residency, schedule, thresholds, autostart, float, delete mode, theme |
| `list_custom_rules()` / `save_custom_rule()` / `delete_custom_rule()` / `validate_custom_rule()` / `custom_rules_dir()` | User rule management with mandatory validation |
| `set_float_enabled()` / `set_float_click_through()` / `show_main_window()` | Float/window control |
| `get_privilege_status()` / `relaunch_elevated_app()` | UAC detection and elevated restart |
| `open_path(path)` / `reveal_item(path)` | Open folder / select file in the system file manager (path allow-list checked) |

Every write command re-validates the path allow-list on the backend; the frontend is never trusted.

## 8. Frontend pages

1. **Dashboard**: capacity gauge, category usage, free space, quick-scan action, low-space banner; theme-aware charts (light/dark/follow-system).
2. **Clean**: rules grouped by category (risk tags, default checks), scan progress, result details (size/files/skipped), delete-mode switch, execution report. Zero-file and unavailable items render as unchecked and disabled.
3. **AI**: cleanable AI caches plus model-store report (model name, size, reveal, official removal commands).
4. **Big files**: threshold filter, large-file table (paged), folder treemap.
5. **Trends**: category growth lines, snapshots, cleanup records, live event stream (folded lists).
6. **Rules**: built-in rules grouped and read-only; custom-rule YAML editor with validation.
7. **Settings**: privileges/elevation, appearance (theme), background & monitoring, float widget, about & updates.

## 9. Packaging & distribution

- Windows: NSIS installer (Tauri bundle), per-user install; L2 system items request UAC at runtime while the main app runs unprivileged.
- macOS: `.app` + DMG (built on macOS); privileged operations such as brew use the system authorization dialog.
- Frontend assets are bundled locally; no external requests and no telemetry.
- Auto-updates are hosted on GitHub Releases; see [RELEASE.md](./RELEASE.md).

## 10. Milestones

- **M1 (MVP) ✅**: Tauri skeleton; rule loading & safety validation; dry-run scanner; system/browser/GPU rules (both platforms); basic cleaning (permanent + optional recycle) with logging; Dashboard and Clean pages. Acceptance: scan sizes within 5% of Explorer properties; dry-run deletes nothing; space is actually reclaimed; denylisted paths can never be hit.
- **M2 ✅**: dev/ai/apps rule packs; AI model report page (ollama integration, per-folder breakdown, official guides); large-file scan and folder treemap; macOS brew cleanup and Xcode/simulator caches; Windows Update service stop/restore (wuauserv/BITS).
- **M3 ✅**: five SQLite tables and baseline snapshot; Trends page (capacity curve, category growth, cleanup records, live event stream); tray; scheduled snapshots and low-space notifications (1 h default cooldown, 12 h snooze then auto-revert); close-to-tray and autostart; `notify` watcher (80 dirs, 2 s debounce); Settings page.
- **M4 ✅**: desktop float widget; custom rules (app-data `rules/*.yaml` merged with built-ins, in-app editor with validation, id-collision guard, hot reload); NSIS installer (per-user, Chinese/English) and Tauri Updater wiring (minisign key, one-click check); rule contribution guide; app icon. Code-signing certificates remain the publisher's responsibility (see [RELEASE.md](./RELEASE.md)).

## 11. Project layout

```
CClean/
├─ README.md / README.zh-CN.md
├─ DESIGN.md / RULES_CONTRIBUTING.md / RELEASE.md (+ .zh-CN.md)
├─ package.json / pnpm-lock.yaml / vite.config.ts / tsconfig.json
├─ index.html
├─ src/                       # Vue 3 frontend
│  ├─ pages/ components/ api/ utils/
│  ├─ App.vue / FloatWidget.vue / theme.css / main.ts
├─ src-tauri/
│  ├─ Cargo.toml / tauri.conf.json
│  ├─ capabilities/ icons/
│  ├─ resources/rules/*.yaml
│  └─ src/
│     ├─ main.rs / lib.rs / commands.rs
│     ├─ rules/ scanner/ cleaner/ analyzer/
│     ├─ watcher.rs snapshot.rs storage.rs resident.rs
│     ├─ floatwin.rs privilege.rs appdir.rs
│     └─ platform/{windows.rs,macos.rs,mod.rs}
└─ design/                    # icon master and design sources
```
