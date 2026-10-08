# CClean 设计文档

> 跨平台（Windows / macOS）磁盘清理与占用监控工具。
> 技术栈：Tauri 2（Rust 后端 + Web 前端，ECharts 图表）。
> 形态：按需启动为主，可选常驻后台（系统托盘 + 定时快照 + 目录监听 + 半透明浮窗）。

---

## 1. 产品目标

1. 解决系统盘（Windows 的 C 盘 / macOS 的系统盘）持续增长问题，安全清理**可再生的缓存与临时文件**。
2. 内置 **AI 工具专项**：覆盖 AI IDE、AI 桌面端、AI 开发框架的缓存；AI **模型文件只报告、不自动删**。
3. 提供**分类占用监控**：分类占用、大文件/大目录排行、基于历史快照的增长趋势。
4. 可选常驻：定时监控、低空间告警、浮窗实时展示磁盘占用与监控目录的写入/删除事件。

非目标：不做注册表清理、不做系统"加速"、不做软件卸载、不做驱动/服务级优化。

## 2. 安全模型（最高优先级）

### 2.1 风险分级

| 级别 | 定义 | 处理方式 |
|---|---|---|
| L0 只读 | 扫描、统计、快照、监听 | 直接执行，不修改系统 |
| L1 安全清理 | 可再生的缓存/临时文件 | Dry-run 清单 → 用户勾选确认 → 清理 |
| L2 系统操作 | 需要管理员/影响系统状态（如停 Windows Update 服务、`brew cleanup`） | 单独标注、单独授权、操作后恢复服务原状态 |
| L3 禁止 | 不可恢复或可能导致系统/数据损坏 | 硬编码黑名单，规则文件也无法开启 |

### 2.2 四条铁律

1. **白名单机制**：只清理规则文件显式声明的目标。规则加载时做路径规范化与越界校验，禁止指向盘符根、用户主目录、`Windows\System32`、`WinSxS`、`Windows\Installer`、`Program Files` 等。
2. **默认 Dry-run**：所有清理先出扫描报告（每项可释放大小、文件数、风险级别），确认后才执行。
3. **个人数据与模型只报告**：桌面/文档/下载等个人目录、微信接收文件、AI 模型库（Ollama / HuggingFace / ModelScope / LM Studio / torch hub 等）只展示大小并提供"打开目录"或官方删除命令指引，不自动删。
4. **全程留痕**：清理写日志（规则、路径、大小、时间、结果、失败原因）；被占用文件跳过，不强杀进程；浏览器运行时提示先关闭。

### 2.3 删除策略

- L1 默认**永久删除**（缓存可再生；执行前必经 Dry-run 确认 + 日志）。
- 每条规则/每次清理可切换：永久删除 / 移入回收站（Win 回收站、mac 废纸篓）/ 隔离区（移动到指定目录，后续版本）。
- 不可逆项（回收站清空、Windows.old、系统还原点）**永远不自动执行**，仅在界面提供系统官方入口指引（cleanmgr、存储设置、vssadmin 命令说明）。

### 2.4 硬编码黑名单（L3）

- Windows：`C:\Windows\System32`、`C:\Windows\WinSxS`（只能引导 DISM）、`C:\Windows\Installer`、`C:\Program Files*`、注册表、`pagefile/hiberfil/swapfile.sys`（只报告 + 引导 powercfg）、关键服务与进程。
- macOS：`/System`、`/usr`（除 brew 管理部分）、`/Library/Apple`、SIP 保护目录、系统快照（只报告）。
- 两平台：浏览器的 Cookies / History / Bookmarks / Login Data / 密码库；个人目录文件；安全软件目录；AI 模型与训练输出。

## 3. 总体架构

```
┌─────────────────────────────────────────────────────────┐
│ 前端（Vue 3 + TS + ECharts，Tauri WebView）               │
│ 主窗口：仪表盘 / 清理 / AI专项 / 大文件 / 趋势 / 设置      │
│ 浮窗（独立透明窗口，常驻时可选，默认关闭）                 │
├─────────────────────────────────────────────────────────┤
│ Tauri Commands（IPC）+ Events（监听事件流推送）            │
├─────────────────────────────────────────────────────────┤
│ Rust 核心：rules / scanner / cleaner / analyzer /         │
│            watcher / snapshot(db) / settings / tray       │
├─────────────────────────────────────────────────────────┤
│ 平台适配层 platform-windows / platform-macos              │
│ 回收站 · 提权 · 服务控制(Win) · 文件监听 · brew(mac)       │
├─────────────────────────────────────────────────────────┤
│ resources/rules/*.yaml（声明式规则，随版本发布、可扩展）    │
│ 应用数据目录：SQLite 快照库、clean_log、配置                │
└─────────────────────────────────────────────────────────┘
```

### 3.1 Rust 模块划分（src-tauri/src/）

| 模块 | 职责 |
|---|---|
| `main.rs / lib.rs` | Tauri Builder、插件注册、窗口/托盘初始化 |
| `rules/` | 规则模型、YAML 加载、路径变量展开、glob 解析、安全校验（黑名单/越界） |
| `scanner/` | 规则目标发现、并行（rayon）目录大小统计、占用/运行状态检测、Dry-run |
| `cleaner/` | 删除执行器（永久/回收站）、服务暂停与恢复（Win L2）、结果汇总、失败收集 |
| `analyzer/` | 大文件扫描、目录大小排行、磁盘总览 |
| `watcher/` | 基于 `notify` 的规则目录监听（Win ReadDirectoryChangesW / macOS FSEvents），事件去抖入库并推送 |
| `snapshot/` | SQLite：快照、快照明细、事件流、清理日志 |
| `settings/` | 配置（常驻、计划、浮窗开关、阈值、自启、删除策略） |
| `tray/` | 托盘菜单 |
| `widget/` | 浮窗窗口控制（透明、置顶、点击穿透、吸附、隐藏） |
| `platform/windows.rs`、`platform/macos.rs` | 回收站（trash crate 或平台 API）、提权重启、服务控制、brew、系统目录 |

### 3.2 主要依赖（Cargo）

`tauri` 2、`serde / serde_yaml / serde_json`、`walkdir`、`rayon`、`notify`、`rusqlite(bundled)`、`trash`、`dirs`、`anyhow / thiserror`、`chrono`、`sysinfo`、`glob`、`tauri-plugin-{shell,dialog,notification,autostart,opener}`。

## 4. 清理规则设计（YAML）

规则文件位于 `src-tauri/resources/rules/`，按类别分文件：`system.yaml / browser.yaml / dev.yaml / ai.yaml / apps.yaml / gpu.yaml / report.yaml`。

### 4.1 Schema

```yaml
- id: system.temp.user            # 全局唯一，分类.名称
  name: 用户临时文件               # 界面显示名
  category: system               # system|browser|dev|ai|apps|gpu|report
  risk: L1                       # L1|L2；report 类为 L0
  os: [windows, macos]           # 适用平台
  default_selected: true         # Dry-run 后默认是否勾选
  delete: permanent              # permanent|recycle
  description: 程序运行产生的临时文件，正在占用的会自动跳过
  targets:
    - paths: ["${TEMP}"]         # 支持变量，见 4.2
      include: ["**/*"]          # glob，默认 **/*
      exclude: []                # 排除 glob
      follow_symlinks: false
    - paths: ["${TMPDIR}"]
      os: [macos]
```

`report` 类规则（只报告）用同样结构，额外字段 `action: open_dir | guide`，`guide` 给出说明文字与官方命令，不提供删除按钮。

### 4.2 路径变量

- 通用：`${HOME}`、`${TMPDIR}`
- Windows：`${TEMP}`、`${WINDIR}`、`${LOCALAPPDATA}`、`${APPDATA}`、`${USERPROFILE}`、`${PROGRAMDATA}`
- macOS：`${HOME}/Library/Caches`、`${HOME}/Library/Logs`、`${HOME}/Library/Application Support`

规则加载即做：变量展开 → 绝对路径规范化 → 必须落在白名单根内（用户缓存目录、系统缓存目录等），否则该条目标记加载错误并禁用，绝不执行。

## 5. 清理项清单（M1–M2 内置）

### 5.1 Windows

| 分类 | L1 自动清理目标 | 只报告/引导 |
|---|---|---|
| 系统 | `%TEMP%`、`C:\Windows\Temp`(L2提权)、Windows 更新缓存 `SoftwareDistribution\Download`(L2，停/启 wuauserv 并恢复原状态)、Delivery Optimization 缓存、缩略图缓存 `Explorer\thumbcache_*`、WER 错误报告、`Windows\Logs`、`Minidump`、`MEMORY.DMP`、Prefetch（默认不勾） | 回收站、`Windows.old`、还原点（vssadmin 指引）、WinSxS（DISM 指引）、hiberfil/pagefile/swapfile（powercfg 指引） |
| 浏览器 | Chrome/Edge/Brave/Firefox 的 `Cache`、`Code Cache`、`GPUCache`、Firefox `cache2`；枚举所有 `Default`/`Profile *` | Cookies、History、Bookmarks、Login Data、密钥（硬黑名单）；浏览器运行时提示关闭 |
| 开发 | pip(`%LOCALAPPDATA%\pip\cache`)、npm(`%LOCALAPPDATA%\npm-cache`)、pnpm store、yarn、uv、cargo `registry\cache`、go-build、NuGet v3-cache、conda `pkgs`、JetBrains `caches/log` | `.m2`、`.gradle\caches`、`.nuget\packages`（可再生但重下慢，默认不勾，归入可选项）；Docker/WSL 的 vhdx 只报告 + `docker builder prune`/压缩指引 |
| AI | **AI IDE**：Cursor / VS Code / VSCodium / Windsurf / Trae 的 `Cache`、`CachedData`、`Code Cache`、`GPUCache`、`logs`、`CachedExtensionVSIXs`；**AI 桌面端**（Claude/ChatGPT/豆包/Kimi/通义/DeepSeek 等 Electron 应用）的 Cache 族目录与 logs；HuggingFace/ModelScope 的 `*.incomplete` 残留下载；torch_extensions；Jupyter runtime | **模型库只报告**：`~/.ollama/models`（附 `ollama list/rm` 指引）、`.cache\huggingface\hub`、`.cache\modelscope`、LM Studio models、`.cache\torch\hub`、ComfyUI/A1111 的 `models` 与 `outputs`、AnythingLLM 文档库 |
| 应用 | 微信/QQ 的**纯 Cache 目录**（`FileStorage\Cache` 等，按 glob 精确匹配）、常见视频/音乐/网盘 App 缓存 | 接收的文件/图片/视频/聊天记录（个人资料，只报告） |
| GPU | NVIDIA `DXCache/GLCache/NV_Cache`、AMD/Intel shader cache、`D3DSCache`、Steam `htmlcache` | 游戏本体、存档 |

### 5.2 macOS

| 分类 | L1 自动清理目标 | 只报告/引导 |
|---|---|---|
| 系统 | `~/Library/Caches`（用户级应用缓存）、`~/Library/Logs`、`${TMPDIR}`、用户崩溃报告 `~/Library/Logs/DiagnosticReports` | `/Library/Caches`（提权项，默认不勾）、废纸篓、系统快照、iOS 备份（`~/Library/Application Support/MobileSync/Backup`） |
| 包管理/开发 | `brew cleanup -s`（先 `--dry-run`，L2 授权）、pip(`~/Library/Caches/pip`)、npm(`~/.npm/_cacache`)、yarn、pnpm、uv、cargo、go-build、Xcode `DerivedData`、`iOS DeviceSupport`（默认不勾） | Xcode Archives、Docker.raw（报告 + Docker 内清理指引）、Maven/Gradle 仓库 |
| 浏览器 | Chrome/Edge/Brave/Firefox/Safari 的缓存目录（Safari 仅 Caches，不碰书签/历史/密码） | 同 Windows 黑名单 |
| AI | AI IDE 与 Electron AI 桌面端在 `~/Library/Application Support/<App>/` 下的 Cache 族目录、`~/Library/Caches/<App>`、HF/ModelScope 残留下载 | `~/.ollama/models`、`~/.cache/huggingface`、`~/Library/Application Support/LM Studio` 模型、torch hub、ComfyUI models/outputs |
| 应用 | `~/Library/Caches` 下各应用缓存（按 bundle 粒度展示，可勾选） | 文档/下载/桌面、照片库、聊天软件接收文件 |

### 5.3 大文件/大目录分析（L0，两平台）

- 用户目录（桌面、下载、文档、视频、`$HOME`）下大于阈值（默认 100MB，可调）的文件：路径、大小、最后修改时间、"打开所在目录"。
- 系统盘顶层与二级目录大小排行（Windows：Users/Windows/Program Files/ProgramData；macOS：/Applications、~/Library、/Library、/System 仅报告）。
- 只报告，所有删除动作由用户在系统中自行完成。

## 6. 监控、快照与常驻模式

### 6.1 SQLite 表（应用数据目录 `cclean.db`）

- `snapshots(id, created_at, disk_total, disk_used, disk_free)`
- `snapshot_items(id, snapshot_id, rule_id, path, size_bytes, file_count)`
- `fs_events(id, ts, kind(write|delete|create|rename), rule_id, path, size_delta)`
- `clean_logs(id, ts, rule_id, path, size_bytes, status, detail)`
- `settings(key, value)`

### 6.2 常驻组件（默认关闭，设置中开启）

1. **系统托盘**：立即扫描、打开主窗口、浮窗开关、退出。
2. **定时快照**：默认每日 1 次（可配间隔），静默扫描规则路径并写快照；低空间阈值（默认剩余 10%）系统通知告警。
3. **目录监听**：仅监听规则登记的缓存目录（`notify`，事件去抖 2s 聚合），写入/删除事件入库并推送给浮窗与主界面；**不做全盘挂钩**。
4. **开机自启**：`tauri-plugin-autostart`，仅常驻模式需要。

### 6.3 浮窗（widget，常驻时可选，默认关闭）

- 独立 Tauri 窗口：透明、无边框、置顶、可切换点击穿透、可拖动、边缘吸附；胶囊窗口 200×64（胶囊本体 56px 高，四周留白保证圆角与投影完整显示）。
- 内容：系统盘剩余空间 + 占用进度条（迷你仪表）。
- 功能按钮：① 一键清理（对全部默认可安全清理规则先扫描后清理，点击后浮窗内二次确认，4 秒不确认自动取消，完成后显示释放空间）；② 打开主窗口（还原并聚焦）；③ 隐藏浮窗。鼠标穿透开关保留在托盘菜单与设置页。
- 透明背景由全局 CSS（html/body/#app 透明 + body 边距清零）保证，避免圆角外出现白边；不采集任何网络数据，全部数据来自本机扫描与监听。

## 7. Tauri Command API（前后端契约）

| Command | 说明 |
|---|---|
| `get_system_overview()` | 磁盘列表（挂载点/总量/已用/剩余）、OS、版本、是否提权 |
| `get_rules()` | 规则元数据 + 目标是否存在、当前平台可用性 |
| `scan(rule_ids: Option<Vec<String>>)` | Dry-run/实扫：返回每条规则的大小、文件数、占用跳过数、状态 |
| `clean(items: Vec<CleanItem>, mode: DeleteMode)` | 按确认清单执行，返回释放量、失败/跳过明细，写 clean_logs |
| `list_big_files(dirs, min_bytes, limit)` | 大文件列表 |
| `list_dir_sizes(path, depth)` | 目录大小排行 |
| `get_snapshots(range)` / `get_trend(rule_ids, range)` | 快照与趋势 |
| `get_events(limit)` | 监听事件流（历史）；实时事件走 Tauri event `fs-event` |
| `set_resident(enabled)` / `set_widget(enabled)` / `get_settings()` / `set_settings()` | 常驻、浮窗、阈值、计划、自启、删除策略 |
| `open_path(path)` | 在系统文件管理器中定位（opener 插件，路径白名单校验） |

所有写操作 command 在后端再次校验路径白名单（不信任前端）。

## 8. 前端页面

1. **仪表盘**：容量仪表、分类占用旭日图、剩余空间、快捷扫描按钮、低空间提示。
2. **清理**：按分类分组的规则列表（风险标签、默认勾选）、扫描进度、结果明细（大小/文件数/占用跳过）、删除方式切换、执行与结果报告。
3. **AI 专项**：AI 缓存可清项 + 模型库占用报告（模型名、大小、打开目录、官方删除命令说明）。
4. **大文件**：阈值筛选、大文件表、目录排行 Treemap。
5. **趋势**：分类增长折线、增长最快 Top N、快照对比、清理记录。
6. **设置**：常驻/自启/扫描计划/低空间阈值/浮窗开关与透明度/删除默认策略/规则目录（高级，支持自定义规则）。

## 9. 打包与分发

- Windows：NSMS/NSIS 安装包（Tauri bundle），系统清理项运行时按需 UAC 提权（主程序默认普通权限）。
- macOS：`.app` + universal binary（aarch64/x86_64）、DMG；brew 等提权操作走系统授权弹窗。
- 前端资源全部本地打包，无外网请求；不收集任何遥测数据。

## 10. 里程碑与验收

- **M1（MVP）✅ 已完成**：Tauri 工程骨架；规则加载与安全校验；扫描引擎（Dry-run）；系统/浏览器/GPU 规则（两平台）；基础清理（永久删除 + 回收站可选）+ 日志；仪表盘 + 清理页。
  验收：在本机扫描结果与资源管理器属性抽查误差 <5%；Dry-run 不产生任何删除；清理后空间确实释放；黑名单路径无法被规则命中。
- **M2 ✅ 已完成**：dev/ai/apps 规则包；AI 模型报告页（ollama list 集成、子目录明细、官方删除指引）；大文件扫描与目录占用 Treemap；macOS brew cleanup（dry-run + 执行）、Xcode/模拟器缓存；Windows 更新缓存清理的服务停启（wuauserv/bits，恢复原状态）。
- **M3 ✅ 已完成**：SQLite（rusqlite bundled）五张表与首次启动基线快照；趋势页（系统盘容量曲线、分类缓存增长曲线、清理记录、实时事件流）；系统托盘（立即快照/打开/退出，双击显示）；定时快照（间隔可配，默认 24h）与低空间系统通知（阈值可配，默认 10%；启动后立即检查一次；默认冷却 1 小时，仪表盘提示条可暂缓 12 小时，到期自动恢复 1 小时节奏）；常驻模式关闭窗口隐藏到托盘 + 开机自启；`notify` 监听规则登记的缓存目录（最多 80 个，2s 去抖，写 fs_events 并 emit `fs-event`）；清理执行写 clean_logs；设置页。
- **M4 ✅ 已完成**：桌面浮窗（独立透明置顶窗口、迷你剩余空间仪表、拖动边缘吸附、可切鼠标穿透、位置/状态持久化，托盘菜单与设置页双入口）；自定义规则（应用数据目录 `rules/*.yaml` 与内置规则合并加载、应用内 YAML 编辑器+保存前强制安全校验、id 冲突防护、增删后即时生效）；NSIS 安装包配置（当前用户安装、中英文）与 Tauri Updater 自动更新接线（minisign 密钥、设置页一键检查更新，发布流程见 RELEASE.md）；规则贡献指南 RULES_CONTRIBUTING.md；应用图标。代码签名证书需发布者自行配置（见 RELEASE.md）。

## 11. 工程目录

```
CClean/
├─ DESIGN.md
├─ README.md
├─ package.json / pnpm-lock.yaml / vite.config.ts / tsconfig.json
├─ index.html
├─ src/                    # 前端 Vue3
│  ├─ pages/ views/ components/ api/ stores/
├─ src-tauri/
│  ├─ Cargo.toml
│  ├─ tauri.conf.json
│  ├─ capabilities/
│  ├─ icons/
│  ├─ resources/rules/*.yaml
│  └─ src/
│     ├─ main.rs
│     ├─ commands.rs
│     ├─ rules/ scanner/ cleaner/ analyzer/
│     ├─ watcher/ snapshot/ settings/ tray/ widget/
│     └─ platform/{windows.rs,macos.rs,mod.rs}
└─ tests/                  # 规则校验、路径安全、扫描逻辑单测
```
