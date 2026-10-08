//! 缓存目录监听：只监听规则登记且当前存在的缓存目录（不做全盘挂钩）。
//! 事件 2 秒去抖聚合后写入 fs_events，并向前端 emit `fs-event`。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{Event, EventKind, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager};

use crate::rules::Rule;
use crate::scanner;
use crate::storage::Storage;
use crate::AppState;

const MAX_WATCHES: usize = 80;
const DEBOUNCE_SECS: u64 = 2;

fn normalize(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    s.strip_prefix("//?/").unwrap_or(&s).to_lowercase()
}

/// 收集规则登记的、当前存在的目录（去重，上限 MAX_WATCHES）。
fn watch_roots(rules: &[Rule]) -> Vec<(PathBuf, String)> {
    let mut roots: Vec<(PathBuf, String)> = Vec::new();
    for rule in rules {
        for target in scanner::resolve_rule(rule) {
            if !target.path.is_dir() {
                continue;
            }
            let norm = normalize(&target.path);
            if roots.iter().any(|(p, _)| normalize(p) == norm) {
                continue;
            }
            roots.push((target.path.clone(), rule.id.clone()));
            if roots.len() >= MAX_WATCHES {
                return roots;
            }
        }
    }
    roots
}

fn match_rule(roots: &[(PathBuf, String)], path: &Path) -> Option<String> {
    let p = normalize(path);
    roots
        .iter()
        .filter(|(root, _)| {
            let r = normalize(root);
            p == r || p.starts_with(&format!("{r}/"))
        })
        .max_by_key(|(root, _)| normalize(root).len())
        .map(|(_, id)| id.clone())
}

fn kind_name(kind: EventKind) -> Option<&'static str> {
    match kind {
        EventKind::Create(_) => Some("create"),
        EventKind::Remove(_) => Some("delete"),
        EventKind::Modify(notify::event::ModifyKind::Name(_)) => Some("rename"),
        EventKind::Modify(_) => Some("write"),
        _ => None,
    }
}

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let (rules, storage): (Vec<Rule>, Arc<Storage>) = {
            let Some(state) = app.try_state::<AppState>() else {
                return;
            };
            let rules = match state.rules.read() {
                Ok(g) => g.clone(),
                Err(_) => return,
            };
            (rules, state.storage.clone())
        };
        let roots = watch_roots(&rules);
        if roots.is_empty() {
            return;
        }

        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[watcher] 创建监听器失败: {e}");
                return;
            }
        };
        let mut watched = 0usize;
        for dir in roots.iter().map(|(d, _)| d) {
            // 二次确认目录仍存在（如 Prefetch 被组策略关闭会在启动后消失），
            // 不存在的目录静默跳过，避免 PathNotFound 噪声报错
            if !dir.is_dir() {
                continue;
            }
            match watcher.watch(dir, RecursiveMode::Recursive) {
                Ok(()) => watched += 1,
                // 目录在监听建立瞬间不存在/消失，忽略
                Err(e) if matches!(e.kind, notify::ErrorKind::PathNotFound) => {}
                Err(e)
                    if matches!(
                        &e.kind,
                        notify::ErrorKind::Io(ioe)
                            if ioe.kind() == std::io::ErrorKind::NotFound
                    ) => {}
                Err(e) => eprintln!("[watcher] 监听 {} 失败: {e}", dir.display()),
            }
        }
        println!("[watcher] 已监听 {watched} 个缓存目录");

        // key: (rule_id, path) -> kind（同一路径多种事件时保留最后一次）
        let mut pending: HashMap<(String, String), String> = HashMap::new();

        loop {
            match rx.recv_timeout(Duration::from_secs(DEBOUNCE_SECS)) {
                Ok(Ok(event)) => {
                    let Some(kind) = kind_name(event.kind) else {
                        continue;
                    };
                    for path in event.paths {
                        let Some(rule_id) = match_rule(&roots, &path) else {
                            continue;
                        };
                        let p = path.to_string_lossy();
                        let p = if p.len() > 500 {
                            p.chars().take(500).collect::<String>()
                        } else {
                            p.to_string()
                        };
                        pending.insert((rule_id, p), kind.to_string());
                    }
                }
                Ok(Err(e)) => eprintln!("[watcher] 事件错误: {e}"),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if pending.is_empty() {
                        continue;
                    }
                    let batch = std::mem::take(&mut pending);
                    for ((rule_id, path), kind) in batch {
                        let ts = chrono::Utc::now().timestamp();
                        match storage.insert_fs_event(&kind, &rule_id, &path, None) {
                            Ok(id) => {
                                let payload = serde_json::json!({
                                    "id": id, "ts": ts, "kind": kind,
                                    "ruleId": rule_id, "path": path, "sizeDelta": null,
                                });
                                let _ = app.emit("fs-event", payload);
                            }
                            Err(e) => eprintln!("[watcher] 事件入库失败: {e}"),
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}
