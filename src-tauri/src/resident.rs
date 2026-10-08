//! 常驻后台：定时快照 + 低空间通知。线程每 10 分钟醒一次，按设置决定动作。

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::platform;
use crate::snapshot;
use crate::storage::Storage;
use crate::AppState;

pub const SETTING_INTERVAL_HOURS: &str = "snapshot_interval_hours";
pub const SETTING_LOW_SPACE_PCT: &str = "low_space_threshold_pct";
pub const SETTING_RESIDENT: &str = "resident_enabled";
pub const SETTING_LAST_SNAPSHOT: &str = "last_snapshot_ts";
pub const SETTING_LAST_ALERT: &str = "last_low_space_alert_ts";
/// 用户主动暂缓到的时间戳（0 表示未暂缓）
pub const SETTING_SNOOZE_UNTIL: &str = "low_space_snooze_until";

const TICK_SECS: u64 = 600;
/// 默认告警冷却：1 小时
const ALERT_COOLDOWN_SECS: i64 = 3600;

fn setting_or(storage: &Storage, key: &str, default: i64) -> i64 {
    storage
        .get_setting(key)
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// 启动后台调度线程。
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(TICK_SECS));
        if let Some(state) = app.try_state::<AppState>() {
            let storage = state.storage.clone();
            let rules = state.rules.read().map(|g| g.clone()).unwrap_or_default();
            run_tick(&app, &storage, &rules);
        }
    });
}

fn run_tick(app: &AppHandle, storage: &Storage, rules: &[crate::rules::Rule]) {
    let now = chrono::Utc::now().timestamp();
    let interval_hours = setting_or(storage, SETTING_INTERVAL_HOURS, 24).clamp(1, 168);
    let last_snapshot = setting_or(storage, SETTING_LAST_SNAPSHOT, 0);

    if now - last_snapshot >= interval_hours * 3600 {
        match snapshot::take(storage, rules) {
            Ok(snap) => {
                let _ = storage.set_setting(SETTING_LAST_SNAPSHOT, &snap.created_at.to_string());
                check_low_space(
                    app,
                    storage,
                    &snap.disk_name,
                    snap.disk_free,
                    snap.disk_total,
                    now,
                );
                let _ = app.emit("snapshot-taken", snap);
            }
            Err(e) => {
                eprintln!("[resident] 定时快照失败: {e}");
            }
        }
        return;
    }

    // 未到快照时间也检查一次剩余空间（用实时容量，成本很低）
    let overview = platform::overview();
    if let Some(disk) = overview
        .disks
        .iter()
        .find(|d| {
            let m = d.mount_name.replace('\\', "/").to_lowercase();
            m == "c:/" || m == "/"
        })
    {
        check_low_space(app, storage, &disk.mount_name, disk.available_bytes as i64, disk.total_bytes as i64, now);
    }
}

fn check_low_space(
    app: &AppHandle,
    storage: &Storage,
    disk_name: &str,
    free: i64,
    total: i64,
    now: i64,
) {
    if total <= 0 {
        return;
    }

    // 用户主动点了“N 小时内不再提醒”：到期前静默；到期后清除暂缓，恢复默认 1h 冷却
    let snooze_until = setting_or(storage, SETTING_SNOOZE_UNTIL, 0);
    if snooze_until > 0 {
        if now < snooze_until {
            return;
        }
        let _ = storage.set_setting(SETTING_SNOOZE_UNTIL, "0");
    }

    let pct = free * 100 / total;
    let threshold = setting_or(storage, SETTING_LOW_SPACE_PCT, 10).clamp(1, 50);
    if pct >= threshold {
        return;
    }
    let last_alert = setting_or(storage, SETTING_LAST_ALERT, 0);
    if now - last_alert < ALERT_COOLDOWN_SECS {
        return;
    }
    let _ = storage.set_setting(SETTING_LAST_ALERT, &now.to_string());

    let free_gb = free as f64 / (1024.0 * 1024.0 * 1024.0);
    let title = "系统盘剩余空间不足";
    let body = format!(
        "{disk_name} 仅剩 {free_gb:.1} GB（{pct}%），建议打开 CClean 清理缓存；\
         可在仪表盘提示条中选择 12 小时内不再提醒"
    );
    show_notification(app, title, &body);
}

/// 立即执行一次低空间检查（启动时调用，不等定时轮询）。
pub fn check_now(app: &AppHandle) {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return;
    };
    let now = chrono::Utc::now().timestamp();
    let overview = platform::overview();
    if let Some(disk) = overview.disks.iter().find(|d| {
        let m = d.mount_name.replace('\\', "/").to_lowercase();
        m == "c:/" || m == "/"
    }) {
        check_low_space(
            app,
            &state.storage,
            &disk.mount_name,
            disk.available_bytes as i64,
            disk.total_bytes as i64,
            now,
        );
    }
}

fn show_notification(app: &AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        eprintln!("[resident] 通知发送失败: {e}");
    }
}
