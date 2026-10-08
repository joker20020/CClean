# CClean

[English](./README.md) | **简体中文**

跨平台（Windows / macOS）磁盘缓存清理与占用监控工具，基于 **Tauri 2 + Rust + Vue 3 + ECharts**。

> 状态：**M1–M4 已完成** —— 规则引擎、缓存清理、AI 报告、大文件分析、快照趋势、常驻守护、桌面浮窗、自定义规则、NSIS 安装包与自动更新通道均已就绪。
> 完整设计见 [DESIGN.zh-CN.md](./DESIGN.zh-CN.md)，规则编写见 [RULES_CONTRIBUTING.zh-CN.md](./RULES_CONTRIBUTING.zh-CN.md)，打包发布见 [RELEASE.zh-CN.md](./RELEASE.zh-CN.md)。

## 功能

- **声明式 YAML 规则**：系统 / 浏览器 / 开发工具 / AI 工具 / 应用 / 显卡六类缓存，规则位于 `src-tauri/resources/rules/`
- **默认 Dry-run**：先扫描出每项可释放大小，勾选确认后才清理
- **白名单安全模型**：规则路径必须落在平台白名单根目录内，越界规则加载时即被拒绝；个人文件、浏览器密码/历史、AI 模型只报告不删除
- **AI 专项清理**：Cursor / VS Code / Windsurf / Trae、Claude / ChatGPT 桌面端、豆包桌面端、腾讯 WorkBuddy / CodeBuddy、ZCode（含官方确认可删的 cli/exec）、Claude Code / Codex / Copilot CLI、Bito、通义灵码、HuggingFace 残留下载、torch 扩展等；会话历史、skills、配置、登录态一律不碰
- **AI 模型只报告**：Ollama（自动读取 `ollama list`）、HuggingFace / ModelScope / LM Studio / Torch Hub 模型库占用、子目录明细、打开目录与官方删除指引
- **大文件 / 大目录分析**：个人目录 ≥100MB/500MB/1GB 大文件表（大小、修改时间、定位）；"定位"在资源管理器/Finder 中打开所在文件夹并选中文件；任意目录一级占用 Treemap
- **平台专项**：Windows 更新缓存清理自动停/启 wuauserv、BITS 服务并恢复原状态；macOS 支持 `brew cleanup -s`（先预演）与 Xcode/模拟器缓存
- **快照与趋势**：SQLite 本地库（应用数据目录 `cclean.db`，无外网请求）记录系统盘容量与各规则占用；首次启动自动建立基线快照；趋势页展示容量曲线、分类增长、清理记录
- **常驻守护（可选，默认关闭）**：系统托盘（立即快照/打开/退出）、定时快照（6/12/24/48h 可选）、低空间系统通知（启动即检查，默认每小时最多一次，仪表盘提示条可暂缓 12 小时、到期自动恢复）、关闭窗口最小化到托盘、开机自启
- **缓存目录监听**：仅监听规则登记的缓存目录（最多 80 个、2 秒去抖），新建/写入/删除事件入库并实时推送到趋势页，不做全盘挂钩
- **桌面浮窗（可选）**：透明置顶迷你胶囊实时显示系统盘剩余空间，可拖动、靠近屏幕左右边缘自动吸附、可切换鼠标穿透，位置与状态持久化；浮窗内置一键清理（二次确认）、打开主窗口、隐藏三个快捷操作
- **自定义规则**：应用内「规则」页直接编写 YAML（内置规则只读浏览），保存前强制白名单安全校验，存放于应用数据目录 `rules/`，扫描/清理即时生效
- **安装包与自动更新**：Windows NSIS 当前用户安装包；内置 Updater，设置页一键检查更新（发布流程见 RELEASE.md）
- **占用跳过**：被占用、无权限文件自动跳过，不强杀进程
- **磁盘总览**：系统盘仪表与各分区占用条

## 文档

- [DESIGN.zh-CN.md](./DESIGN.zh-CN.md)：完整设计与安全模型
- [RULES_CONTRIBUTING.zh-CN.md](./RULES_CONTRIBUTING.zh-CN.md)：清理规则 YAML 写法、白名单与贡献流程
- [RELEASE.zh-CN.md](./RELEASE.zh-CN.md)：打包、代码签名与自动更新发布步骤

## 开发运行

前置：Rust（stable）、Node 18+、pnpm；Windows 还需 WebView2（Win10/11 通常自带）与 MSVC 构建工具。

```powershell
pnpm install
pnpm tauri dev      # 开发运行
```

其他命令：

```powershell
# 前端类型检查 + 构建
pnpm build

# Rust 编译检查 / 单元测试（规则安全校验、glob 匹配）
cd src-tauri
cargo check
cargo test

# 无界面冒烟测试：真实扫描若干缓存目录（只读，不删除）
cargo run --example smoke

# 打包安装程序
pnpm tauri build
```

## 安全边界

- 只删除规则白名单内的可再生缓存；回收站、Windows.old、系统还原点、WinSxS 等不可逆/系统项仅提供官方入口指引
- L1 默认永久删除（执行前有确认对话框），后续版本支持回收站与隔离区模式
- 所有代码本地运行，无网络请求、无遥测
