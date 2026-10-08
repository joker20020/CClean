# 清理规则贡献指南

CClean 的全部清理能力都由声明式 YAML 规则驱动，不写一行删除代码。新增一个软件的缓存清理，通常只需要在规则文件里加一段 YAML。

- 内置规则目录：`src-tauri/resources/rules/*.yaml`（编译时通过 rust-embed 打包进程序）
- 用户自定义规则目录：
  - Windows：`%APPDATA%\com.cclean.desktop\rules\`
  - macOS：`~/Library/Application Support/com.cclean.desktop/rules/`
  - 应用内「规则」页可直接新建、校验、编辑、删除，也可以点「打开目录」手工放 YAML 文件

## 1. 规则文件结构

一个 YAML 文件包含若干条规则（数组）。单条规则的完整字段：

```yaml
- id: apps.example_cache        # 必填，全局唯一，点分命名，建议 类别.软件_内容
  name: 示例应用缓存             # 必填，界面显示名（中文）
  category: apps                # 必填：system | browser | dev | ai | apps | gpu
  risk: L1                      # 必填：L1（可再生缓存）| L2（需提权/停服务）| L0（只报告）
  os: [windows, macos]          # 可选，留空 = 全平台；windows | macos | linux
  default_selected: true        # 可选，扫描页是否默认勾选，默认 true；高风险建议 false
  delete: permanent             # 可选：permanent（默认，永久删除）| recycle（移入回收站）
  action: ""                    # 可选："report" 表示只统计/报告，永远不删除
  description: 一句话说明清理内容与影响
  services:                     # 可选：清理前后需要停/启的系统服务（仅 L2）
    os: [windows]
    stop: [wuauserv, bits]
    restart: true               # true（默认）= 只重启原本就在运行的服务
  targets:
    - os: [windows]             # 可选，该目标仅在指定平台生效
      paths:                    # 必填，支持路径变量与 glob 通配
        - "${LOCALAPPDATA}/Example/Cache"
      include: []               # 可选 glob 白名单（相对 paths），为空 = 目录下全部
      exclude:                  # 可选 glob 黑名单（相对 paths），优先于 include
        - "**/keep/**"
```

### 路径变量

| 变量 | Windows | macOS |
| --- | --- | --- |
| `${USERPROFILE}` / `${HOME}` | `C:\Users\<你>` | `/Users/<你>` |
| `${LOCALAPPDATA}` | `%LOCALAPPDATA%` | 同 HOME（兜底） |
| `${APPDATA}` | `%APPDATA%\Roaming` | `~/Library/Application Support` |
| `${TEMP}` / `${TMPDIR}` | 临时目录 | `$TMPDIR` |
| `${WINDIR}` | `C:\Windows` | — |
| `${PROGRAMDATA}` | `C:\ProgramData` | — |

### glob 与通配目录

- 路径段支持 `*` `?` `[]`，例如 `${LOCALAPPDATA}/Google/Chrome/User Data/*/Cache` 会枚举所有 Profile。
- `include`/`exclude` 使用相对 glob，`**` 匹配任意层级。
- 路径中**禁止 `..`**，加载期直接拒绝。

## 2. 安全模型（硬性约束）

规则在加载时强制安全校验，不通过的规则会被整条拒绝（控制台打印原因，程序继续运行）：

1. **白名单根目录**：目标路径展开后必须位于下列根目录之内：
   - 系统临时目录、`%LOCALAPPDATA%`、`%APPDATA%`；
   - 用户目录下的 `.cache/.npm/.cargo/.ollama/.lmstudio/.zcode/.zai/.claude/.codex/.copilot/.bito/.lingma`、conda、微信文件目录等；
   - Windows：`%WINDIR%/Temp|Logs|Minidump|SoftwareDistribution|Prefetch`、`MEMORY.DMP`、`%PROGRAMDATA%/Microsoft/Windows`、`Windows.old`、根目录的 `hiberfil.sys/pagefile.sys/swapfile.sys`（仅报告）；
   - macOS：`~/Library/Caches|Logs|Application Support|Developer`、各 AI 应用容器、`/Library/Caches`。
2. **永远不允许**：`C:\Windows\System32`、`WinSxS`、`Installer`、Program Files、用户的文档/桌面/下载整体、浏览器 Cookies/History/Login Data 等个人数据。
3. **个人数据与大文件只报告**：AI 模型库、聊天接收文件等用 `action: report` + `risk: L0`，界面给出官方删除指引，绝不自动删。
4. **默认 Dry-run**：所有清理先预演展示大小与文件数，用户确认后才执行；被占用/无权限文件自动跳过，从不强杀进程。
5. 用户自定义规则的 id 不允许与内置规则重名（防止覆盖内置规则）。

> 如果你的规则确实需要新的白名单根（例如一个把缓存写在非标准位置的软件），请在 PR 中说明理由，修改 `src-tauri/src/rules/mod.rs` 的 `allowed_roots()`，而不是在规则里用 `..` 绕路。

## 3. 编写步骤

1. 在应用「规则」页点「新建规则」，粘贴 YAML，点「校验」——校验会做解析、必填字段、id 冲突、白名单全套检查；
2. 保存后到「扫描与清理」页扫描，确认大小、文件数、目标路径符合预期；
3. 先保持 `default_selected: false` 自测 Dry-run + 实际清理，确认软件仍能正常运行、缓存会自动重建；
4. 贡献内置规则：把 YAML 按类别放入 `src-tauri/resources/rules/`，提交 PR。

## 4. 提交前检查

- `src-tauri` 目录运行 `cargo test`（含内置规则加载与白名单拒绝用例）；
- `cargo run --example smoke` 可在本机只读探测规则命中情况；
- 规则 id 全局唯一、命名与类别一致；
- 只声明缓存/临时/日志等**可再生**内容；不确定的内容一律降级为 `action: report`；
- 描述里写清楚「删了会怎样」（例如着色器缓存重建期间游戏首次加载会卡顿）。

## 5. 内置规则分类约定

| 文件 | category | 典型内容 |
| --- | --- | --- |
| `system.yaml` | system | 临时目录、Windows 更新缓存、缩略图、WER、Prefetch |
| `browser.yaml` | browser | Chrome/Edge/Firefox 等的 Cache/Code Cache/GPUCache（绝不碰 Cookies） |
| `dev.yaml` | dev | npm/pnpm/yarn、cargo、Gradle、Maven、pip、构建目录 |
| `ai.yaml` | ai | Cursor、豆包、WorkBuddy、CodeBuddy、ZCode、Claude/Codex/Copilot CLI、ollama 模型（报告）等 |
| `apps.yaml` | apps | 微信/QQ 缓存、各桌面应用 cache（聊天接收文件只报告） |
| `gpu.yaml` | gpu | NVIDIA DXCache/GLCache、D3DSCache、各 GPU 着色器缓存 |
| `report.yaml` | 各分类 | AI 模型库、系统大文件等只报告项 |
