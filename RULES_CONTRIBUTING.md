# Contributing Cleanup Rules

All of CClean's cleaning ability is driven by declarative YAML rules — no deletion code needs to be written. Adding cache cleanup for a new application is usually just a short YAML snippet.

- Built-in rules: `src-tauri/resources/rules/*.yaml` (compiled into the binary via `rust-embed`).
- User custom rules:
  - Windows: `%APPDATA%\com.cclean.desktop\rules\`
  - macOS: `~/Library/Application Support/com.cclean.desktop/rules/`
  - The in-app **Rules** page can create, validate, edit and delete rules; "Open folder" lets you drop in YAML files manually.

## 1. Rule file structure

A YAML file contains an array of rules. Full set of fields for one rule:

```yaml
- id: apps.example_cache        # required, globally unique, dotted; category.app_content
  name: Example App Cache       # required, UI label
  category: apps                # required: system | browser | dev | ai | apps | gpu
  risk: L1                      # required: L1 (regenerable) | L2 (elevation/services) | L0 (report-only)
  os: [windows, macos]          # optional; empty = all platforms; windows | macos | linux
  default_selected: true        # optional, default true; set false for risky items
  delete: permanent             # optional: permanent (default) | recycle (Recycle Bin/Trash)
  action: ""                    # optional: "report" = count/report only, never delete
  description: One sentence on what is cleaned and the impact
  services:                     # optional: services to stop/restart around cleaning (L2 only)
    os: [windows]
    stop: [wuauserv, bits]
    restart: true               # true (default) = restart only services that were originally running
  targets:
    - os: [windows]             # optional: target applies only on these platforms
      paths:                    # required; variables and globs supported
        - "${LOCALAPPDATA}/Example/Cache"
      include: []               # optional glob allow-list (relative to paths); empty = everything
      exclude:                  # optional glob denylist (relative to paths), wins over include
        - "**/keep/**"
```

### Path variables

| Variable | Windows | macOS |
| --- | --- | --- |
| `${USERPROFILE}` / `${HOME}` | `C:\Users\<you>` | `/Users/<you>` |
| `${LOCALAPPDATA}` | `%LOCALAPPDATA%` | falls back to HOME |
| `${APPDATA}` | `%APPDATA%\Roaming` | `~/Library/Application Support` |
| `${TEMP}` / `${TMPDIR}` | Temp folder | `$TMPDIR` |
| `${WINDIR}` | `C:\Windows` | — |
| `${PROGRAMDATA}` | `C:\ProgramData` | — |

### Globs and wildcarded directories

- Path segments support `*` `?` `[]`; e.g. `${LOCALAPPDATA}/Google/Chrome/User Data/*/Cache` enumerates every profile.
- `include`/`exclude` use relative globs; `**` matches any depth.
- `..` in paths is **forbidden** and rejected at load time.

## 2. Security model (hard constraints)

Rules are force-validated at load time; any rule that fails is rejected wholesale (the reason is printed to the console and the app keeps running):

1. **Allow-list roots**: after expansion a target must live inside one of these roots:
   - the system temp dir, `%LOCALAPPDATA%`, `%APPDATA%`;
   - user-directory `.cache/.npm/.cargo/.ollama/.lmstudio/.zcode/.zai/.claude/.codex/.copilot/.bito/.lingma`, conda, the WeChat files folder, etc.;
   - Windows: `%WINDIR%/Temp|Logs|Minidump|SoftwareDistribution|Prefetch`, `MEMORY.DMP`, `%PROGRAMDATA%/Microsoft/Windows`, `Windows.old`, drive-root `hiberfil.sys/pagefile.sys/swapfile.sys` (report-only);
   - macOS: `~/Library/Caches|Logs|Application Support|Developer`, AI app containers, `/Library/Caches`.
2. **Never allowed**: `C:\Windows\System32`, `WinSxS`, `Installer`, Program Files, the whole Documents/Desktop/Downloads trees, browser Cookies/History/Login Data and similar personal data.
3. **Personal data and large files are report-only**: AI model stores, received chat files, etc. use `action: report` + `risk: L0`; the UI shows official removal guidance and never deletes them.
4. **Dry-run by default**: every cleanup previews size and file counts and runs only after confirmation; locked/unauthorized files are skipped and processes are never killed.
5. Custom-rule ids must not collide with built-in ids (built-in rules cannot be overridden).

> If your rule genuinely needs a new allow-list root (for example, an app that writes caches to a non-standard location), justify it in the PR and extend `allowed_roots()` in `src-tauri/src/rules/mod.rs` — never route around the check with `..`.

## 3. Authoring workflow

1. In the app's **Rules** page click **New rule**, paste the YAML and click **Validate** — parsing, required fields, id collisions and the full allow-list check run before saving.
2. Save, then scan on the **Clean** page and confirm the size, file counts and target paths match expectations.
3. Keep `default_selected: false` while testing the dry-run and a real cleanup; verify the app still works and its cache is rebuilt.
4. To contribute a built-in rule, place the YAML in the right category file under `src-tauri/resources/rules/` and open a PR.

## 4. Pre-submit checklist

- Run `cargo test` in `src-tauri` (covers built-in rule loading and allow-list rejection).
- `cargo run --example smoke` probes rule hits on your machine read-only.
- Rule ids are globally unique and consistent with their category.
- Only **regenerable** content (cache/temp/logs) is declared; anything uncertain is downgraded to `action: report`.
- The description states the user-visible impact (e.g. "first game launch after clearing shaders may stutter while caches rebuild").

## 5. Category conventions for built-in rules

| File | category | Typical content |
| --- | --- | --- |
| `system.yaml` | system | Temp folders, Windows Update cache, thumbnails, WER, Prefetch |
| `browser.yaml` | browser | Chrome/Edge/Firefox Cache/Code Cache/GPUCache (never Cookies) |
| `dev.yaml` | dev | npm/pnpm/yarn, cargo, Gradle, Maven, pip, build directories |
| `ai.yaml` | ai | Cursor, Doubao, WorkBuddy, CodeBuddy, ZCode, Claude/Codex/Copilot CLI, ollama models (report), … |
| `apps.yaml` | apps | WeChat/QQ caches, desktop-app caches (received files report-only) |
| `gpu.yaml` | gpu | NVIDIA DXCache/GLCache, D3DSCache, GPU shader caches |
| `report.yaml` | various | AI model stores, system large files and other report-only items |
