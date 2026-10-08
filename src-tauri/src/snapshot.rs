//! 快照采集：扫描全部规则 + 系统盘容量，写入 SQLite。

use std::sync::Arc;

use crate::platform;
use crate::rules::Rule;
use crate::scanner;
use crate::storage::{Snapshot, SnapshotInputItem, Storage};

/// 选择系统盘：Windows 为 C:\，macOS/Linux 为 /，否则取第一块盘。
fn system_disk(disks: &[platform::DiskInfo]) -> Option<platform::DiskInfo> {
    disks
        .iter()
        .find(|d| {
            let m = d.mount_name.replace('\\', "/").to_lowercase();
            m == "c:/" || m == "/"
        })
        .or_else(|| disks.first())
        .cloned()
}

/// 执行一次完整快照（扫描可能耗时，应在阻塞线程中调用）。
pub fn take(storage: &Storage, rules: &[Rule]) -> Result<Snapshot, String> {
    let overview = platform::overview();
    let disk = system_disk(&overview.disks)
        .ok_or_else(|| "未检测到任何磁盘".to_string())?;

    let stats = scanner::scan_rules(rules, None);
    let items: Vec<SnapshotInputItem> = stats
        .iter()
        .filter(|s| s.size_bytes > 0 || s.file_count > 0)
        .map(|s| SnapshotInputItem {
            rule_id: s.rule.id.clone(),
            size_bytes: s.size_bytes as i64,
            file_count: s.file_count as i64,
        })
        .collect();

    let id = storage
        .insert_snapshot(
            &disk.mount_name,
            disk.total_bytes,
            disk.used_bytes,
            disk.available_bytes,
            &items,
        )
        .map_err(|e| e.to_string())?;

    Ok(Snapshot {
        id,
        created_at: chrono::Utc::now().timestamp(),
        disk_name: disk.mount_name,
        disk_total: disk.total_bytes as i64,
        disk_used: disk.used_bytes as i64,
        disk_free: disk.available_bytes as i64,
    })
}

/// 在阻塞线程中采集快照的便捷封装（供命令与定时任务共用）。
pub fn take_shared(storage: Arc<Storage>, rules: Vec<Rule>) -> Result<Snapshot, String> {
    take(&storage, &rules)
}
