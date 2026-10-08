//! 权限提升（一键获取管理员权限）：
//! - 查询当前进程是否已提权（Windows 令牌 Elevation；macOS/Linux 有效 UID 为 0）
//! - 通过系统授权弹窗以提升后的权限重新启动自身（Windows UAC / macOS administrator privileges）
//!
//! 设计说明：应用本身不常驻管理员权限。用户在设置页点击"以管理员身份重启"后，
//! 系统弹出 UAC/密码框确认；确认后新实例拥有管理员权限，可清理
//! `C:\Windows\Temp`、`Prefetch`、`SoftwareDistribution\Download` 等标准权限下
//! 会被跳过的位置。用户取消授权时返回错误，原实例继续运行，不受影响。

/// 当前进程是否以提升后的权限运行。
pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows_is_elevated()
    }
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(any(target_os = "windows", unix)))]
    {
        false
    }
}

#[cfg(target_os = "windows")]
fn windows_is_elevated() -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // TOKEN_INFORMATION_CLASS::TokenElevation 的整数值为 20
    const TOKEN_ELEVATION_CLASS: i32 = 20;

    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut ret_len = 0u32;
        let status = GetTokenInformation(
            token,
            std::mem::transmute::<i32, _>(TOKEN_ELEVATION_CLASS),
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        let _ = CloseHandle(token);
        status != 0 && elevation.TokenIsElevated != 0
    }
}

/// 以提升权限重新启动当前可执行文件。成功返回后调用方应退出当前（未提权）实例。
pub fn relaunch_elevated() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("获取程序路径失败: {e}"))?;
    let exe_str = exe.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        // 通过 PowerShell 的 Start-Process -Verb RunAs 触发 UAC 弹窗；
        // 用户点"否"时命令返回非零，此处如实报错。
        let ps = format!(
            "Start-Process -FilePath '{exe_str}' -ArgumentList '--elevated' -Verb RunAs"
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps])
            .output()
            .map_err(|e| format!("启动提权进程失败: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = if stderr.contains("取消") || stderr.to_lowercase().contains("cancel") {
                "用户取消了授权".to_string()
            } else {
                stderr.trim().to_string()
            };
            return Err(format!("无法获取管理员权限: {detail}"));
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        // osascript 弹出系统授权框，以 root 启动新实例
        let script = format!(
            "do shell script \"'{exe_str}' --elevated >/dev/null 2>&1 &\" with administrator privileges"
        );
        let output = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("启动提权进程失败: {e}"))?;
        if !output.status.success() {
            return Err("无法获取管理员权限（用户取消或授权失败）".to_string());
        }
        Ok(())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = exe_str;
        Err("当前平台不支持图形化提权，请用 sudo 手动启动".to_string())
    }
}
