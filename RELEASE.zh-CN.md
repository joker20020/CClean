# 打包、签名与自动更新说明

## 1. 本地打包

前置：Rust 稳定版、Node 22+、pnpm；Windows 打包 NSIS 由 Tauri 自动下载工具链（首次需要网络）。

```powershell
# Windows：只出 NSIS 安装包（推荐）
$env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.tauri\cclean.key"
pnpm tauri build --bundles nsis
# 产物：src-tauri/target/release/bundle/nsis/CClean_0.1.0_x64-setup.exe
# 同时生成自动更新用的签名文件：CClean_0.1.0_x64-setup.nsis.zip + .sig
```

```bash
# macOS：dmg（需在 macOS 上构建）
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/cclean.key"
pnpm tauri build --bundles dmg
```

- 安装包为**当前用户安装**（`installMode: currentUser`），不需要管理员权限；
- WebView2 引导程序采用按需下载模式，已装 WebView2 的 Windows 11 无额外步骤；
- 安装后开始菜单快捷方式带 AUMID，系统 toast 通知（低空间告警等）才能正常弹出——开发态未打包程序的 toast 可能被 Windows 静默丢弃。

## 2. 代码签名（发布前必做）

当前构建产物**未签名**，Windows SmartScreen 首次运行会弹警告，这是正常现象，不影响功能。

正式发布建议：

- **Windows**：购买 OV/EV 代码签名证书（Authenticode），配置环境变量后重新打包：
  `TAURI_SIGNING_PRIVATE_KEY` 是**更新包**签名密钥（minisign），与代码签名证书是两回事，两者都需要；
  在 CI 中配置 `TAURI_CERTIFICATE_SHA256`/`TAURI_CERTIFICATE_PASSWORD` 等（见 Tauri 官方 NSIS 签名文档）。
- **macOS**：需要 Apple Developer ID，对 `.app` 与 dmg 做 codesign + 公证（notarization），否则用户无法直接打开。

## 3. 自动更新发布流程

更新通道使用 Tauri Updater（minisign 签名校验）：

1. 密钥已生成在 `~/.tauri/cclean.key`（私钥，保密，勿提交）与 `cclean.key.pub`（公钥已写入 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`）；
2. 更新端点已配置为 GitHub Releases：`https://github.com/joker20020/CClean/releases/latest/download/latest.json`（最新正式 Release 的同名资产）；
3. 发布新版本：修改 `tauri.conf.json` 与 `Cargo.toml` 的版本号 → 带 `TAURI_SIGNING_PRIVATE_KEY` 环境变量执行 `pnpm tauri build`；
4. 在 GitHub Releases 上传：
   - `CClean_<ver>_x64-setup.nsis.zip`（注意是 zip，不是裸 exe）及其 `.sig`；
   - `CClean_<ver>_x64-setup.exe`（供手动下载）；
5. 发布说明里附带 `latest.json`（Tauri 模板）：

```json
{
  "version": "0.2.0",
  "notes": "更新内容",
  "pub_date": "2026-11-01T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<.sig 文件内容>",
      "url": "https://github.com/OWNER/CClean/releases/download/v0.2.0/CClean_0.2.0_x64-setup.nsis.zip"
    }
  }
}
```

用户在「设置 → 检查更新」即可一键下载安装（安装完成后需重启应用）。

> 私钥一旦泄露或丢失：泄露需立即更换密钥（已安装版本只信任旧公钥，需要发版引导手动更新）；丢失则无法再向存量用户推送自动更新，请妥善备份。
