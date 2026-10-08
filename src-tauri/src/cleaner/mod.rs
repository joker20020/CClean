//! 清理执行器：永久删除 / 回收站两种模式。
//! 只删除匹配规则的文件与清空后变空的目录，目标根目录本身保留；
//! 被占用或无权限的文件跳过并记录，绝不强杀进程。

use std::path::{Path, PathBuf};

use crate::rules::Rule;
use crate::scanner::{resolve_rule, Matcher};

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanOutcome {
    pub rule_id: String,
    pub freed_bytes: u64,
    pub deleted_files: u64,
    pub error_count: u64,
    pub errors: Vec<String>,
}

pub fn clean_rule(rule: &Rule, mode: &str) -> Result<CleanOutcome, String> {
    if rule.is_report() {
        return Err(format!("规则 {} 为只报告项，不允许自动清理", rule.id));
    }
    let mode = if mode == "recycle" { "recycle" } else { "permanent" };

    let mut outcome = CleanOutcome {
        rule_id: rule.id.clone(),
        freed_bytes: 0,
        deleted_files: 0,
        error_count: 0,
        errors: Vec::new(),
    };

    // L2：按规则配置停止系统服务（仅 Windows），记录原本在运行的服务以便恢复。
    let mut stopped_services: Vec<String> = Vec::new();
    #[cfg(windows)]
    {
        if let Some(cfg) = &rule.services {
            if cfg.os.is_empty() || cfg.os.iter().any(|o| o == "windows") {
                for svc in &cfg.stop {
                    if crate::platform::svc::is_running(svc) {
                        crate::platform::svc::stop(svc).map_err(|e| {
                            format!("停止服务 {svc} 失败，清理已中止（通常需要管理员权限）：{e}")
                        })?;
                        stopped_services.push(svc.clone());
                    }
                }
            }
        }
    }

    let work = (|| -> Result<(), String> {
        for target in resolve_rule(rule) {
        let matcher = Matcher::new(&target.include, &target.exclude);
        let path = &target.path;

        if path.is_file() {
            if matcher.matches_path(path, path) {
                delete_file(path, path, mode, &mut outcome);
            }
            continue;
        }
        if !path.is_dir() {
            continue;
        }

        // 1) 收集并删除匹配的文件
        let mut matched_files: Vec<PathBuf> = Vec::new();
        for entry in walkdir::WalkDir::new(path).follow_links(false).into_iter().flatten() {
            if entry.path() == path {
                continue;
            }
            if entry.file_type().is_file() && matcher.matches_path(path, entry.path()) {
                matched_files.push(entry.path().to_path_buf());
            }
        }
        for f in &matched_files {
            delete_file(path, f, mode, &mut outcome);
        }

        // 2) 自底向上删除变空的目录，但绝不删除目标根目录
        let mut empty_dirs: Vec<PathBuf> = Vec::new();
        for entry in walkdir::WalkDir::new(path)
            .follow_links(false)
            .contents_first(true)
            .into_iter()
            .flatten()
        {
            let p = entry.path();
            if p == path || !p.is_dir() {
                continue;
            }
            if std::fs::read_dir(p).map(|mut d| d.next().is_none()).unwrap_or(false) {
                empty_dirs.push(p.to_path_buf());
            }
        }
        for d in empty_dirs {
            if let Err(e) = std::fs::remove_dir(&d) {
                record_error(&mut outcome, &format!("删除目录失败 {}: {e}", d.display()));
            }
        }
        }
        Ok(())
    })();

    // 无论清理成功与否，尝试恢复原本在运行的服务。
    #[cfg(windows)]
    {
        if let Some(cfg) = rule.services.as_ref() {
            if cfg.restart {
                for svc in &stopped_services {
                    if let Err(e) = crate::platform::svc::start(svc) {
                        record_error(
                            &mut outcome,
                            &format!("恢复服务 {svc} 失败，请手动启动：{e}"),
                        );
                    }
                }
            }
        }
    }
    work?;

    Ok(outcome)
}

fn delete_file(root: &Path, file: &Path, mode: &str, outcome: &mut CleanOutcome) {
    let len = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0);
    let result = if mode == "recycle" {
        trash::delete(file).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(file).map_err(|e| e.to_string())
    };
    match result {
        Ok(()) => {
            outcome.freed_bytes = outcome.freed_bytes.saturating_add(len);
            outcome.deleted_files += 1;
        }
        Err(e) => {
            let rel = file.strip_prefix(root).unwrap_or(file);
            record_error(outcome, &format!("{} ({e})", rel.display()));
        }
    }
}

fn record_error(outcome: &mut CleanOutcome, msg: &str) {
    outcome.error_count += 1;
    if outcome.errors.len() < 50 {
        outcome.errors.push(msg.to_string());
    }
}
