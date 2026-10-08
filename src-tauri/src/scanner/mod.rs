//! 扫描引擎：把规则目标解析为具体路径，并行统计可清理大小与文件数。
//! 所有操作只读（L0），不删除、不修改任何文件。

use rayon::prelude::*;
use std::path::{Path, PathBuf};

use crate::rules::{self, Rule, Target};

/// 已解析到具体路径的扫描/清理目标。
#[derive(Clone, Debug)]
pub struct ResolvedTarget {
    pub path: PathBuf,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetStat {
    pub path: String,
    pub exists: bool,
    pub size_bytes: u64,
    pub file_count: u64,
    pub error_count: u64,
}

#[derive(Debug)]
pub struct RuleStat {
    pub rule: Rule,
    pub targets: Vec<TargetStat>,
    pub size_bytes: u64,
    pub file_count: u64,
    pub error_count: u64,
}

/// 把一条规则在当前平台上的目标解析为具体路径列表。
pub fn resolve_rule(rule: &Rule) -> Vec<ResolvedTarget> {
    let mut out = Vec::new();
    for target in &rule.targets {
        if !target_applies(target) {
            continue;
        }
        for raw in &target.paths {
            let expanded = rules::expand_path(raw);
            for concrete in rules::resolve_glob(&expanded) {
                out.push(ResolvedTarget {
                    path: concrete,
                    include: target.include.clone(),
                    exclude: target.exclude.clone(),
                });
            }
        }
    }
    out
}

fn target_applies(t: &Target) -> bool {
    t.os.is_empty() || t.os.iter().any(|o| o == rules::current_platform())
}

pub fn scan_rules(rules: &[Rule], ids: Option<&[String]>) -> Vec<RuleStat> {
    rules
        .par_iter()
        .filter(|r| ids.map_or(true, |ids| ids.iter().any(|id| id == &r.id)))
        .map(scan_rule)
        .collect()
}

pub fn scan_rule(rule: &Rule) -> RuleStat {
    let targets = resolve_rule(rule)
        .par_iter()
        .map(scan_path)
        .collect::<Vec<_>>();
    let size_bytes = targets.iter().map(|t| t.size_bytes).sum();
    let file_count = targets.iter().map(|t| t.file_count).sum();
    let error_count = targets.iter().map(|t| t.error_count).sum();
    RuleStat {
        rule: rule.clone(),
        targets,
        size_bytes,
        file_count,
        error_count,
    }
}

pub fn scan_path(rt: &ResolvedTarget) -> TargetStat {
    let matcher = Matcher::new(&rt.include, &rt.exclude);
    let path = &rt.path;
    if !path.exists() {
        return TargetStat {
            path: path.to_string_lossy().to_string(),
            exists: false,
            size_bytes: 0,
            file_count: 0,
            error_count: 0,
        };
    }

    let mut stat = TargetStat {
        path: path.to_string_lossy().to_string(),
        exists: true,
        size_bytes: 0,
        file_count: 0,
        error_count: 0,
    };

    if path.is_file() {
        if matcher.matches_path(path, path) {
            stat.size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            stat.file_count = 1;
        }
        return stat;
    }

    for entry in walkdir::WalkDir::new(path).follow_links(false).into_iter() {
        match entry {
            Ok(e) => {
                if e.path() == path {
                    continue;
                }
                if e.file_type().is_file() && matcher.matches_path(path, e.path()) {
                    stat.size_bytes = stat
                        .size_bytes
                        .saturating_add(e.metadata().map(|m| m.len()).unwrap_or(0));
                    stat.file_count += 1;
                }
            }
            Err(_) => stat.error_count += 1,
        }
    }
    stat
}

/// 简易 glob 匹配器（支持 **、*、?），统一用正斜杠相对路径匹配。
pub struct Matcher {
    include: Vec<regex::Regex>,
    exclude: Vec<regex::Regex>,
}

impl Matcher {
    pub fn new(include: &[String], exclude: &[String]) -> Self {
        Matcher {
            include: include.iter().filter_map(|p| compile_glob(p)).collect(),
            exclude: exclude.iter().filter_map(|p| compile_glob(p)).collect(),
        }
    }

    pub fn matches_path(&self, root: &Path, path: &Path) -> bool {
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel = rel.to_string_lossy().replace('\\', "/");
        let included = self.include.is_empty() || self.include.iter().any(|g| g.is_match(&rel));
        let excluded = self.exclude.iter().any(|g| g.is_match(&rel));
        included && !excluded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_compilation() {
        let incomplete = compile_glob("**/*.incomplete").unwrap();
        assert!(incomplete.is_match("blobs/abc/model.incomplete"));
        assert!(!incomplete.is_match("blobs/abc/model.safetensors"));

        let thumb = compile_glob("thumbcache_*.db").unwrap();
        assert!(thumb.is_match("thumbcache_32.db"));
        assert!(!thumb.is_match("other.db"));

        let m = Matcher::new(&["**/*.incomplete".into()], &[]);
        let root = Path::new("/tmp/hub");
        assert!(m.matches_path(root, &root.join("x/y/z.incomplete")));
        assert!(!m.matches_path(root, &root.join("x/y/z.bin")));
    }
}

/// glob → 正则：`**` 跨目录匹配，`*` 不跨 `/`，`?` 单字符。
fn compile_glob(pattern: &str) -> Option<regex::Regex> {
    let p = pattern.replace('\\', "/");
    let mut rx = String::from("^");
    let mut chars = p.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    // ** 可匹配零段，吞掉紧随的 /
                    if chars.peek() == Some(&'/') {
                        chars.next();
                    }
                    rx.push_str("(?:.*/)?");
                } else {
                    rx.push_str("[^/]*");
                }
            }
            '?' => rx.push('.'),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' | '/' => {
                if c == '/' {
                    rx.push('/');
                } else {
                    rx.push('\\');
                    rx.push(c);
                }
            }
            _ => rx.push(c),
        }
    }
    rx.push('$');
    regex::Regex::new(&rx).ok()
}
