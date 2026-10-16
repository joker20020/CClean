//! AI 模型库只报告：汇总各模型目录占用、列出子目录明细，
//! 检测 ollama 并给出官方删除命令指引。绝不自动删除模型。

use rayon::prelude::*;
use serde::Serialize;
use std::path::PathBuf;

use crate::analyzer;
use crate::rules::Rule;
use crate::scanner;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelGroup {
    pub rule_id: String,
    pub name: String,
    pub description: String,
    pub available: bool,
    pub size_bytes: u64,
    pub file_count: u64,
    pub paths: Vec<String>,
    pub entries: Vec<ModelEntry>,
    pub guidance: String,
    /// ollama list 等命令行输出（如有）
    pub cli_output: String,
}

fn guidance(rule_id: &str) -> &'static str {
    match rule_id {
        "report.ollama_models" => {
            "终端执行 ollama list 查看已装模型，ollama rm <模型名> 删除指定模型；不要直接删除 blobs 目录"
        }
        "report.hf_models" => {
            "可运行 huggingface-cli delete-cache 交互式选择删除；或打开目录确认后手动删除对应模型文件夹"
        }
        "report.modelscope_models" => {
            "魔搭模型无官方清理命令，请打开目录确认后手动删除对应模型文件夹"
        }
        "report.lmstudio_models" => {
            "建议在 LM Studio 的 My Models 页面中删除模型；也可打开目录手动处理"
        }
        "report.torch_hub" => {
            "torch.hub 预训练权重，删除后代码再次运行会自动重新下载"
        }
        _ => "请打开目录确认后手动处理，或使用对应工具自带的模型管理功能",
    }
}

/// 尝试执行 `ollama list`，返回输出（未安装或失败返回空串）。
/// 加超时保护：ollama 首次运行可能拉起后台服务，长时间无响应不应拖住扫描。
fn ollama_list() -> String {
    let mut command = std::process::Command::new("ollama");
    command
        .arg("list")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        crate::platform::no_window(&mut command);
    }
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let timeout = std::time::Duration::from_secs(15);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().unwrap_or_else(|_| {
                    std::process::Output {
                        status,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    }
                });
                if status.success() {
                    let mut s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if s.is_empty() && !stderr.trim().is_empty() {
                        s = stderr.trim().to_string();
                    }
                    return s;
                }
                return String::new();
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return String::new();
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(_) => return String::new(),
        }
    }
}

pub fn build_report(rules: &[Rule]) -> Vec<ModelGroup> {
    crate::taskpool::run(|| {
        let ai_report_rules: Vec<&Rule> = rules
            .iter()
            .filter(|r| r.is_report() && r.category == "ai")
            .collect();

        let ollama_output = ollama_list();

        ai_report_rules
            .par_iter()
            .map(|rule| {
                let targets = scanner::resolve_rule(rule);
                let existing: Vec<PathBuf> =
                    targets.iter().map(|t| t.path.clone()).filter(|p| p.exists()).collect();

                let mut size_bytes = 0u64;
                let mut file_count = 0u64;
                let mut entries: Vec<ModelEntry> = Vec::new();

                for path in &existing {
                    // 目录：统计一级子项（模型仓库/模型文件夹粒度）
                    if path.is_dir() {
                        for child in analyzer::child_sizes(path) {
                            size_bytes = size_bytes.saturating_add(child.size_bytes);
                            file_count = file_count.saturating_add(child.file_count);
                            entries.push(ModelEntry {
                                name: child.name,
                                path: child.path,
                                size_bytes: child.size_bytes,
                                is_dir: child.is_dir,
                            });
                        }
                    } else if let Ok(meta) = std::fs::metadata(path) {
                        size_bytes = size_bytes.saturating_add(meta.len());
                        file_count += 1;
                    }
                }
                entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

                let cli_output = if rule.id == "report.ollama_models" {
                    ollama_output.clone()
                } else {
                    String::new()
                };

                ModelGroup {
                    rule_id: rule.id.clone(),
                    name: rule.name.clone(),
                    description: rule.description.clone(),
                    available: !existing.is_empty() && (size_bytes > 0 || !cli_output.is_empty()),
                    size_bytes,
                    file_count,
                    paths: existing.iter().map(|p| p.to_string_lossy().to_string()).collect(),
                    entries,
                    guidance: guidance(&rule.id).to_string(),
                    cli_output,
                }
            })
            .collect::<Vec<_>>()
    })
}
