//! 后端静态文案的最小化 i18n：托盘菜单、系统通知。
//! 语言设置与前端共享同一个 storage 键（"language"），取值
//! zh-CN / en-US / system；新增语种时在此扩展 Lang 与 Strings。

use crate::storage::Storage;

pub const SETTING_LANGUAGE: &str = "language";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    ZhCn,
    EnUs,
}

pub struct Strings {
    pub tray_snapshot: &'static str,
    pub tray_show: &'static str,
    pub tray_float: &'static str,
    pub tray_through: &'static str,
    pub tray_quit: &'static str,
    pub tray_tooltip: &'static str,
    pub snap_title: &'static str,
    pub snap_body: &'static str,
    pub low_title: &'static str,
}

const ZH: Strings = Strings {
    tray_snapshot: "立即记录快照",
    tray_show: "打开主窗口",
    tray_float: "桌面浮窗",
    tray_through: "浮窗鼠标穿透",
    tray_quit: "退出 CClean",
    tray_tooltip: "CClean 磁盘清理",
    snap_title: "CClean 快照已记录",
    snap_body: "可在趋势页查看占用变化",
    low_title: "系统盘剩余空间不足",
};

const EN: Strings = Strings {
    tray_snapshot: "Take a snapshot now",
    tray_show: "Open main window",
    tray_float: "Desktop float widget",
    tray_through: "Widget click-through",
    tray_quit: "Quit CClean",
    tray_tooltip: "CClean Disk Cleaner",
    snap_title: "CClean snapshot recorded",
    snap_body: "Open the Trends page to see usage changes",
    low_title: "Low free space on the system drive",
};

impl Strings {
    pub fn get(lang: Lang) -> &'static Strings {
        match lang {
            Lang::ZhCn => &ZH,
            Lang::EnUs => &EN,
        }
    }
}

impl Lang {
    pub fn low_space_body(self, disk: &str, free_gb: f64, pct: i64) -> String {
        match self {
            Lang::ZhCn => format!(
                "{disk} 仅剩 {free_gb:.1} GB（{pct}%），建议打开 CClean 清理缓存；\
                 可在仪表盘提示条中选择 12 小时内不再提醒"
            ),
            Lang::EnUs => format!(
                "Only {free_gb:.1} GB ({pct}%) free on {disk}. Open CClean to clean caches;\n\
                 you can mute this for 12 hours from the dashboard banner."
            ),
        }
    }
}

/// 从持久化设置解析语言；system 时探测操作系统语言。
pub fn from_storage(storage: &Storage) -> Lang {
    match storage
        .get_setting(SETTING_LANGUAGE)
        .ok()
        .flatten()
        .as_deref()
    {
        Some("zh-CN") => Lang::ZhCn,
        Some("en-US") => Lang::EnUs,
        _ => system_lang(),
    }
}

fn system_lang() -> Lang {
    if let Ok(v) = std::env::var("LANG").or_else(|_| std::env::var("LC_ALL")) {
        return if v.to_lowercase().starts_with("zh") {
            Lang::ZhCn
        } else {
            Lang::EnUs
        };
    }
    #[cfg(windows)]
    {
        return windows_system_lang();
    }
    #[cfg(not(windows))]
    {
        Lang::ZhCn
    }
}

#[cfg(windows)]
fn windows_system_lang() -> Lang {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if len > 0 {
        let s = String::from_utf16_lossy(&buf[..len as usize - 1]);
        return if s.to_lowercase().starts_with("zh") {
            Lang::ZhCn
        } else {
            Lang::EnUs
        };
    }
    Lang::ZhCn
}
