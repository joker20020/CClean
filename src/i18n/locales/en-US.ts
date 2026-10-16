/**
 * English (US) messages.
 * Add a new locale by creating another file implementing MessageSchema
 * (derived from the zh-CN source) and registering it in i18n/index.ts.
 */
import type zhCN from "./zh-CN";

/**
 * 结构与 zh-CN 保持一致，但叶子节点放宽为 string（避免中文 as const 的
 * 字面量类型要求英文值逐字相同）。新增语种时同样实现该结构即可。
 */
type DeepString<T> = {
  [K in keyof T]: T[K] extends string ? string : DeepString<T[K]>;
};
type MessageSchema = DeepString<typeof zhCN>;

const enUS: MessageSchema = {
  app: {
    brand: {
      subtitle: "Disk Cleaner",
      windowTitle: "CClean Disk Cleaner",
    },
    nav: {
      dashboard: "Dashboard",
      clean: "Scan & Clean",
      ai: "AI Cleanup",
      bigfiles: "Large Files",
      trend: "Growth Trends",
      rules: "Rules",
      settings: "Settings",
    },
    themeSwitch: {
      title: "Switch theme",
      light: "Light",
      dark: "Dark",
      system: "System",
    },
    titlebar: {
      minimize: "Minimize",
      maximize: "Maximize",
      restore: "Restore",
      close: "Close",
    },
    lowSpace: {
      title: "Low free space on the system drive",
      goClean: "Clean now",
      snooze: "Mute for 12 hours",
      confirm:
        "Only {gb} GB of free space remains on the system drive (below {pct}%).\n\nClick \"Clean now\" to scan and clean, or \"Mute for 12 hours\" to snooze this alert.",
    },
  },
  common: {
    ok: "OK",
    cancel: "Cancel",
    save: "Save",
    saving: "Saving…",
    saved: "Saved",
    delete: "Delete",
    edit: "Edit",
    refresh: "Refresh",
    open: "Open",
    openDir: "Open folder",
    locate: "Locate",
    collapse: "Collapse",
    loading: "Loading…",
    scanning: "Scanning…",
    notScanned: "Not scanned",
    notDetected: "Not detected",
    free: "free",
    used: "used",
    available: "Free space",
    total: "Total",
    items: "{n} items",
    files: "{n} files",
    skippedN: "{n} skipped",
    showMore: "Show more ({n} remaining)",
    showMoreRows: "Show more ({n} remaining)",
    expandMore: "Show {rest} more ({total} in total)",
    selectAll: "Select all",
    reportOnly: "Report only",
    comingSoon: "This module will be delivered in a later milestone",
    m1Ready: "M1 currently supports: rule scanning (dry-run), cache cleaning, and disk overview.",
  },
  category: {
    system: "System",
    browser: "Browser",
    dev: "Developer Tools",
    ai: "AI Tools",
    apps: "App Caches",
    gpu: "GPU / Games",
  },
  risk: {
    L0: "Report only",
    L1: "Safe",
    L2: "Admin required",
  },
  /** Localized overrides for report-only rules (AI model stores); key is the rule id without the report. prefix */
  reportRules: {
    ollama_models: {
      name: "Ollama model library",
      description:
        "Local LLM model files. Remove them with `ollama rm <model>`; accidental deletion means re-downloading several to tens of GB",
      guidance:
        "Run `ollama list` in a terminal to see installed models and `ollama rm <model>` to delete one; do not delete the blobs folder directly.",
    },
    hf_models: {
      name: "HuggingFace model cache",
      description:
        "Fully downloaded models and datasets; remove them with huggingface-cli or manually in the file manager",
      guidance:
        "Run `huggingface-cli delete-cache` to pick and delete models interactively, or open the folder and manually delete the model folders after confirming.",
    },
    modelscope_models: {
      name: "ModelScope model cache",
      description:
        "Full models downloaded from ModelScope; review them in the file manager and delete manually",
      guidance:
        "ModelScope provides no official cleanup command. Open the folder and manually delete the model folders after confirming.",
    },
    lmstudio_models: {
      name: "LM Studio model library",
      description:
        "GGUF and other models downloaded by LM Studio; removing them in My Models is recommended",
      guidance:
        "Delete models on the LM Studio My Models page, or open the folder and handle them manually.",
    },
    torch_hub: {
      name: "Torch Hub models",
      description: "Pretrained weights downloaded by torch.hub",
      guidance:
        "torch.hub pretrained weights are re-downloaded automatically when your code runs again.",
    },
  },
  dashboard: {
    errorRead: "Failed to read system information: {msg}",
    banner: {
      title: "Low space on the system drive: ",
      text: "Only {bytes} ({pct}%) free on {mount}. Consider cleaning caches soon.",
      goClean: "Clean now",
      snooze: "Mute for 12 hours",
    },
    gauge: {
      title: "System drive free space",
      usedInfo: "{mount} · {used} used of {total}",
    },
    disks: {
      title: "Disks overview",
      available: "{bytes} free",
      usage: "{used} / {total} · {fs}",
      elevated: " · Elevated",
      startScan: "Start scan & clean",
    },
    promise: {
      title: "Safety guarantees",
      p1: "Only regenerable caches in the allow-list are cleaned; dry-run always comes first.",
      p2: "AI models, personal files and received chat files are report-only and never auto-deleted.",
      p3: "Locked files are skipped automatically; processes are never force-killed, and every run is logged.",
      p4: "Irreversible targets such as the Recycle Bin, Windows.old and restore points only get official guidance.",
    },
    loading: "Reading system information…",
  },
  clean: {
    scanSelected: "Scan selected",
    scanning: "Scanning…",
    cleanSelected: "Clean selected caches",
    cleaning: "Cleaning…",
    chip: {
      scannable: "Cleanable {bytes}",
      report: "Report only {bytes}",
      freed: "Freed this run {bytes}",
    },
    brew: {
      title: "Homebrew cache maintenance",
      run: "Run brew cleanup -s",
      running: "Cleaning…",
      confirm: "This runs brew cleanup -s to remove old Homebrew versions and download caches. Continue?",
    },
    groupCount: "{n} items",
    outcome: {
      freed: "Freed {bytes} · {files} deleted",
      skipped: " · {n} items skipped (locked or no permission)",
    },
    noDir: "No matching directory found",
    loadingRules: "Loading rules…",
    confirmTitle: "Confirm cleaning",
    confirmOk: "Start cleaning",
    selectedTotal: "{count} selected · {bytes}",
    progress: {
      scanning: "Scanning",
      cleaning: "Cleaning",
    },
    confirmClean:
      "This permanently deletes {count} cache items and is expected to free {bytes}.\nCaches are regenerable; locked files are skipped automatically. Continue?",
  },
  ai: {
    title: "AI model storage usage (report only, never auto-deleted)",
    subtitle: "Models often range from a few GB to tens of GB and must be re-downloaded if deleted. Use the official commands or manage the folders manually.",
    total: "Total {bytes}",
    stats: "Analyzing…",
    loading: "Scanning model folders…",
    empty: "No local AI model stores detected (Ollama / HuggingFace / ModelScope / LM Studio / Torch Hub).",
  },
  bigfiles: {
    tab: {
      files: "Large files",
      dirs: "Folder usage",
    },
    rescan: "Rescan",
    scanning: "Scanning…",
    scanRange: "Scan scope: {roots}",
    scanRangeDefault: "Default user folders",
    table: {
      file: "File",
      size: "Size",
      modified: "Modified",
      name: "Name",
      fileCount: "Files",
      typeDir: "Folder",
      typeFile: "File",
    },
    locate: "Locate",
    empty: "No files above the threshold were found",
    preset: {
      userHome: "Home folder",
      cRootSlow: "C: drive root (slow)",
      rootSlow: "Filesystem root (slow)",
      appdataLocal: "AppData\\Local",
      library: "~/Library",
    },
    scanningDirs: "Recursively calculating sizes (the system drive root can take several minutes)…",
  },
  trend: {
    title: "Storage growth trends",
    subtitle: "Snapshots are recorded after scans on the clean page and by scheduled tasks; at least two snapshots are needed to show a trend.",
    record: "Take a snapshot now",
    recording: "Recording…",
    loading: "Loading…",
    diskChartTitle: "System drive capacity (GB)",
    diskFree: "System drive free",
    diskUsed: "System drive used",
    catChartTitle: "Cache usage by category (GB)",
    hint: "Only {n} snapshot(s) so far; the curve appears automatically after scheduled snapshots (daily by default).",
    logsTitle: "Cleanup history",
    eventsTitle: "Live cache-directory events",
    eventsNote: "Latest 100",
    col: {
      time: "Time",
      rule: "Rule",
      freed: "Freed",
      status: "Status",
      detail: "Detail",
      kind: "Type",
      path: "Path",
    },
    status: {
      ok: "Success",
      partial: "Partial success",
      failed: "Failed",
    },
    kind: {
      create: "Created",
      write: "Written",
      delete: "Deleted",
      rename: "Renamed",
    },
    noLogs: "No cleanup records yet",
    noEvents: "No events (only create/write/delete actions inside cache folders are monitored, aggregated over 2 seconds)",
    manage: {
      live: "Live",
      clear: "Clear",
      deleteRow: "Delete this record",
      clearLogsTitle: "Clear cleanup history",
      clearLogsConfirm:
        "This deletes all {n} cleanup records.\nIt only clears local history and never touches any files on disk. Continue?",
      clearEventsTitle: "Clear real-time events",
      clearEventsConfirm:
        "This deletes all {n} cache-folder event records.\nIt only clears local history and never touches any files on disk. Continue?",
    },
  },
  rules: {
    title: "Rules",
    newRule: "＋ New rule",
    myRules: "My rules ({n})",
    noUserRules: "No custom rules yet. Click the button above to create one.",
    builtinRules: "Built-in rules ({n}, read-only)",
    reportOnly: "Report only",
    editor: {
      newTitle: "New rule",
      editTitle: "Edit rule",
      sub: "New files are auto-named by the first rule id; scanning and cleaning take effect immediately after saving, while folder watching applies after a restart.",
      validate: "Validate",
      saving: "Saving…",
      cancel: "Cancel",
      placeholder: "Paste rule YAML…",
      validateOk: "Validation passed: {n} rule(s) ({ids})",
      tips: "Safety constraints: paths must live under whitelisted roots such as temp and application cache folders; .. traversal, system folders (System32/WinSxS/Installer) and personal document folders are rejected. Every rule is force-validated before saving. See RULES_CONTRIBUTING.md in the project root for the full format.",
      deleteTitle: "Delete rule",
      deleteConfirm: "Delete the custom rule file {name}?",
    },
    placeholder: {
      title: "Select a rule on the left to edit, or create a new custom cleanup rule.",
      dir: "Rules folder: {path}",
    },
    template: `# Custom cleanup rules (one item or a list)
# Path variables: \${USERPROFILE}/\${HOME} \${LOCALAPPDATA} \${APPDATA} \${TEMP} \${WINDIR} \${PROGRAMDATA}
# Paths must stay inside whitelisted roots (cache/temp folders); system and personal document folders are rejected
- id: apps.myapp_cache          # globally unique; must not collide with built-in rules
  name: My App Cache
  category: apps                # system | browser | dev | ai | apps | gpu
  risk: L1                      # L1 is cleanable; for report-only use L0 + action: report
  os: [windows]                 # windows | macos; leave empty for all platforms
  default_selected: false
  delete: permanent             # permanent | recycle
  description: Example rule that cleans an application's cache folder
  targets:
    - paths:
        - "\${LOCALAPPDATA}/MyApp/Cache"
`,
  },
  settings: {
    loading: "Loading…",
    privilege: {
      title: "Privileges",
      current: "Current privileges",
      admin: "Running as administrator",
      user: "Running with standard privileges",
      desc: "Standard privileges automatically skip protected locations such as C:\\Windows\\Temp, Prefetch and Windows Update caches; restart elevated to clean them. Elevation applies only to the current run and is reset after a restart.",
      elevate: "Restart as administrator",
      waiting: "Waiting for authorization…",
      requesting: "Requesting administrator privileges — please click Yes in the system prompt…",
      failed: "Elevation not completed: {msg}",
      fullPerms: "Full cleanup privileges are enabled",
    },
    appearance: {
      title: "Appearance",
      theme: "Theme",
      themeDesc: "Light, dark, or follow the system appearance",
      light: "Light",
      dark: "Dark",
      system: "System",
      language: "Language",
      languageDesc: "Interface language; the tray menu and system notifications switch as well",
      langZh: "中文",
      langEn: "English",
      langSystem: "System",
    },
    background: {
      title: "Background & monitoring",
      resident: "Resident in the background (system tray)",
      residentDesc: "Minimize to the tray when the main window is closed, continuing scheduled snapshots and low-space monitoring",
      autostart: "Launch at login",
      autostartDesc: "Start CClean automatically after login (recommended together with tray residency)",
      interval: "Automatic snapshot interval",
      intervalDesc: "Quietly scans all rules in the background and records usage for trend comparison",
      everyHours: "Every {h} hours",
      threshold: "Low-space alert threshold",
      thresholdDesc: "Send a system notification when the system drive free percentage drops below this value (at most once per hour by default; mute for 12 hours from the dashboard banner)",
      thresholdOption: "Under {p}% free",
    },
    float: {
      title: "Desktop float widget",
      enabled: "Show the float widget",
      enabledDesc: "A small always-on-top widget showing live system drive free space; it is draggable and snaps to the left/right screen edges",
      through: "Click-through",
      throughDesc: "When enabled, mouse clicks pass through the widget (view-only); use the tray menu or this switch to restore it",
      deleteMode: "Default deletion mode",
      deleteModeDesc: "Permanent deletion cannot be undone; the Recycle Bin can be restored (per-item switching remains available on the clean page)",
      permanent: "Delete permanently",
      recycle: "Move to Recycle Bin",
    },
    about: {
      title: "About & updates",
      version: "App updates",
      versionDesc: "Current version v{version}. The update source is configured via plugins.updater.endpoints in tauri.conf.json; after a new release is published, download and install it here with one click.",
      check: "Check for updates",
      processing: "Working…",
      checking: "Checking for updates…",
      upToDate: "You are already on the latest version",
      found: "New version {version} found; downloading and installing…",
      downloading: "Downloading version {version}…",
      downloadingProgress: "Downloading version {version}… {mb} MB downloaded",
      installing: "Download complete; installing…",
      installed: "The update is installed. Please quit and restart CClean.",
      failed: "Update check failed: {msg} (shown when the release endpoint is unconfigured or offline)",
    },
  },
  float: {
    free: "free",
    confirmClean: "Clean recommended caches?",
    cleaning: "Cleaning…",
    noItems: "Nothing to clean",
    alreadyClean: "All clean",
    freed: "Freed {bytes}",
    done: "Cleaning complete",
    failed: "Cleaning failed — see the main window",
    titleConfirm: "Confirm cleaning",
    titleCancel: "Cancel",
    titleQuickClean: "One-click clean of recommended caches",
    titleOpen: "Open the main window",
    titleHide: "Hide the widget (re-enable it from Settings or the tray menu)",
  },
};

export default enUS;
