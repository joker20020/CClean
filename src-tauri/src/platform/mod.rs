//! 平台适配：磁盘信息、系统版本、提权状态检测、brew、Windows 服务控制。

use serde::Serialize;
use sysinfo::{Disks, System};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub mount_name: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub os: String,
    pub platform: String,
    pub elevated: bool,
    pub disks: Vec<DiskInfo>,
}

#[derive(Debug, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct BrewInfo {
    pub available: bool,
    pub version: String,
    /// `brew cleanup -n` 的输出（将清理什么）
    pub dry_run: String,
}

pub fn overview() -> Overview {
    let disks = Disks::new_with_refreshed_list()
        .list()
        .iter()
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            DiskInfo {
                mount_name: d.mount_point().to_string_lossy().to_string(),
                file_system: String::from_utf8_lossy(d.file_system().as_encoded_bytes())
                    .to_string(),
                total_bytes: total,
                available_bytes: available,
                used_bytes: total.saturating_sub(available),
            }
        })
        .collect();

    let name = System::name().unwrap_or_else(|| std::env::consts::OS.to_string());
    let version = System::os_version().unwrap_or_default();
    let os = format!("{name} {version}").trim().to_string();

    Overview {
        os,
        platform: crate::rules::current_platform().to_string(),
        elevated: is_elevated(),
        disks,
    }
}

#[cfg(windows)]
fn is_elevated() -> bool {
    std::process::Command::new("net")
        .arg("session")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_elevated() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// macOS Homebrew 缓存状态（其他平台返回 unavailable）。
pub fn brew_status() -> BrewInfo {
    #[cfg(target_os = "macos")]
    {
        let version = match std::process::Command::new("brew")
            .arg("--version")
            .output()
        {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").to_string()
            }
            _ => return BrewInfo::default(),
        };
        let dry_run = match std::process::Command::new("brew")
            .args(["cleanup", "-n"])
            .output()
        {
            Ok(o) => {
                let mut s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() {
                    s = "当前没有可清理的 Homebrew 缓存".to_string();
                }
                s
            }
            Err(e) => format!("执行 brew cleanup 预演失败: {e}"),
        };
        BrewInfo {
            available: true,
            version,
            dry_run,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        BrewInfo::default()
    }
}

/// 执行 `brew cleanup -s`，返回输出。
pub fn brew_cleanup() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("brew")
            .args(["cleanup", "-s"])
            .output()
            .map_err(|e| format!("启动 brew 失败: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("当前平台不支持 brew".to_string())
    }
}

/// Windows 服务控制（清理 Windows 更新缓存前后使用）。
#[cfg(windows)]
pub mod svc {
    pub fn is_running(name: &str) -> bool {
        let Ok(out) = std::process::Command::new("sc.exe")
            .args(["query", name])
            .output()
        else {
            return false;
        };
        let text = String::from_utf8_lossy(&out.stdout);
        text.lines().any(|l| l.contains("RUNNING"))
    }

    pub fn stop(name: &str) -> Result<(), String> {
        let out = std::process::Command::new("sc.exe")
            .args(["stop", name])
            .output()
            .map_err(|e| format!("{e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }

    pub fn start(name: &str) -> Result<(), String> {
        let out = std::process::Command::new("sc.exe")
            .args(["start", name])
            .output()
            .map_err(|e| format!("{e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}
