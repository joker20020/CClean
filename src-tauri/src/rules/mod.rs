//! 声明式清理规则：YAML 加载、路径变量展开、glob 解析与安全校验。
//!
//! 安全模型：每条规则目标路径展开后必须落在平台白名单根目录之内，
//! 否则整条规则被拒绝加载。report 类规则同样校验，但永远不会被删除。

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(rust_embed::RustEmbed)]
#[folder = "resources/rules/"]
struct RulesAsset;

#[derive(Clone, Debug, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub category: String,
    /// L1 | L2 ；report 类为 L0
    pub risk: String,
    #[serde(default)]
    pub os: Vec<String>,
    #[serde(default = "default_true")]
    pub default_selected: bool,
    /// permanent | recycle
    #[serde(default)]
    pub delete: String,
    #[serde(default)]
    pub description: String,
    /// 空字符串表示可清理规则；"report" 表示只报告
    #[serde(default)]
    pub action: String,
    /// L2 清理前后需要停/启的系统服务（如 Windows Update）
    #[serde(default)]
    pub services: Option<ServicesCfg>,
    #[serde(default)]
    pub targets: Vec<Target>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ServicesCfg {
    #[serde(default)]
    pub os: Vec<String>,
    #[serde(default)]
    pub stop: Vec<String>,
    /// 清理后是否仅重启原本在运行的服务，默认 true
    #[serde(default = "default_true")]
    pub restart: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Target {
    pub paths: Vec<String>,
    #[serde(default)]
    pub os: Vec<String>,
    /// glob，相对于目标路径；为空表示全部
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Rule {
    pub fn is_report(&self) -> bool {
        self.action == "report"
    }

    pub fn delete_mode(&self) -> &str {
        if self.delete == "recycle" {
            "recycle"
        } else {
            "permanent"
        }
    }
}

pub fn current_platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// 加载全部内置规则：平台过滤 + 安全校验。
pub fn load_builtin() -> Vec<Rule> {
    let mut out = Vec::new();
    for fname in RulesAsset::iter().filter(|n| n.ends_with(".yaml")) {
        let asset = match RulesAsset::get(&fname) {
            Some(a) => a,
            None => continue,
        };
        let rules: Vec<Rule> = match serde_yaml::from_slice(asset.data.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[rules] 解析 {} 失败: {e}", fname.as_ref());
                continue;
            }
        };
        for rule in rules {
            if !rule.os.is_empty() && !rule.os.iter().any(|o| o == current_platform()) {
                continue;
            }
            match validate(&rule) {
                Ok(()) => out.push(rule),
                Err(e) => eprintln!("[rules] 规则 {} 被安全校验拒绝: {e}", rule.id),
            }
        }
    }
    out
}

/// 兼容旧调用名。
pub fn load_all() -> Vec<Rule> {
    load_builtin()
}

// ===== 用户自定义规则 =====

/// 用户规则目录：与数据库同级的 rules/ 子目录（统一由 appdir 模块管理）。
pub fn user_rules_dir() -> PathBuf {
    crate::appdir::user_rules_dir()
}

/// 解析用户粘贴的 YAML：支持单条规则文档或规则数组。
pub fn parse_user_yaml(text: &str) -> Result<Vec<Rule>, String> {
    if let Ok(rule) = serde_yaml::from_str::<Rule>(text) {
        return Ok(vec![rule]);
    }
    serde_yaml::from_str::<Vec<Rule>>(text).map_err(|e| format!("YAML 解析失败：{e}"))
}

/// 对外暴露安全校验。
pub fn validate_rule(rule: &Rule) -> Result<(), String> {
    validate(rule)
}

/// 加载内置 + 用户自定义规则；用户规则 id 与内置重复时拒绝加载（防止覆盖内置规则）。
pub fn load_all_with_user() -> Vec<Rule> {
    let builtin = load_builtin();
    let mut out = builtin.clone();
    let dir = user_rules_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("yaml"))
        .collect();
    files.sort();

    for path in files {
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[rules] 读取用户规则 {:?} 失败: {e}", path.file_name());
                continue;
            }
        };
        let rules = match parse_user_yaml(&text) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[rules] 用户规则 {:?} {e}", path.file_name());
                continue;
            }
        };
        for rule in rules {
            if !rule.os.is_empty() && !rule.os.iter().any(|o| o == current_platform()) {
                continue;
            }
            if builtin.iter().any(|r| r.id == rule.id) {
                eprintln!("[rules] 用户规则 {} 的 id 与内置规则冲突，已跳过", rule.id);
                continue;
            }
            if out.iter().any(|r| r.id == rule.id) {
                eprintln!("[rules] 用户规则 id 重复 {}，已跳过", rule.id);
                continue;
            }
            match validate(&rule) {
                Ok(()) => out.push(rule),
                Err(e) => eprintln!("[rules] 用户规则 {} 被安全校验拒绝: {e}", rule.id),
            }
        }
    }
    out
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRuleFile {
    pub filename: String,
    pub content: String,
}

/// 列出用户规则文件（原始 YAML 文本，供前端编辑）。
pub fn list_user_files() -> Vec<UserRuleFile> {
    let dir = user_rules_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("yaml"))
        .collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|p| {
            let filename = p.file_name()?.to_string_lossy().to_string();
            let content = std::fs::read_to_string(&p).ok()?;
            Some(UserRuleFile { filename, content })
        })
        .collect()
}

/// 校验并保存用户规则文件。filename 仅允许字母数字 . _ -，且必须以 .yaml 结尾。
pub fn save_user_file(filename: &str, content: &str) -> Result<(), String> {
    let name = filename.trim();
    if name.contains('/') || name.contains('\\') || name.contains("..") || !name.ends_with(".yaml") {
        return Err("文件名非法：仅允许字母/数字/._- 且以 .yaml 结尾".into());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err("文件名含非法字符".into());
    }

    let rules = parse_user_yaml(content)?;
    if rules.is_empty() {
        return Err("文件中没有任何规则".into());
    }
    let builtin = load_builtin();
    let mut seen = std::collections::HashSet::new();
    for rule in &rules {
        if rule.id.trim().is_empty() || rule.name.trim().is_empty() || rule.targets.is_empty() {
            return Err(format!("规则 {} 缺少 id/name/targets 字段", rule.id));
        }
        if builtin.iter().any(|r| r.id == rule.id) {
            return Err(format!("规则 id {} 与内置规则冲突，请改名", rule.id));
        }
        if !seen.insert(rule.id.clone()) {
            return Err(format!("文件内规则 id 重复：{}", rule.id));
        }
        validate(rule)?;
    }

    let dir = user_rules_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建规则目录失败：{e}"))?;
    std::fs::write(dir.join(name), content).map_err(|e| format!("写入失败：{e}"))?;
    Ok(())
}

/// 删除用户规则文件。
pub fn delete_user_file(filename: &str) -> Result<(), String> {
    let name = filename.trim();
    if name.contains('/') || name.contains('\\') || name.contains("..") || !name.ends_with(".yaml")
    {
        return Err("文件名非法".into());
    }
    let path = user_rules_dir().join(name);
    std::fs::remove_file(path).map_err(|e| format!("删除失败：{e}"))
}

/// 展开路径变量（${TEMP} 等），返回规范化绝对路径（不要求路径存在）。
pub fn expand_path(raw: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let local = dirs::data_local_dir().unwrap_or_else(|| home.clone());
    // Windows 下 config_dir = %APPDATA%\Roaming；macOS 下为 ~/Library/Application Support
    let config = dirs::config_dir().unwrap_or_else(|| home.clone());
    let temp = std::env::temp_dir();

    let mut s = raw.replace('\\', "/");

    let replace = |s: &mut String, key: &str, val: &Path| {
        let token = format!("${{{key}}}");
        if s.contains(&token) {
            *s = s.replace(&token, &val.to_string_lossy().replace('\\', "/"));
        }
    };
    replace(&mut s, "USERPROFILE", &home);
    replace(&mut s, "HOME", &home);
    replace(&mut s, "LOCALAPPDATA", &local);
    replace(&mut s, "APPDATA", &config);
    replace(&mut s, "TEMP", &temp);
    replace(&mut s, "TMPDIR", &temp);
    if cfg!(windows) {
        let windir = std::env::var("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:/Windows"));
        let programdata = std::env::var("PROGRAMDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:/ProgramData"));
        replace(&mut s, "WINDIR", &windir);
        replace(&mut s, "PROGRAMDATA", &programdata);
    }

    normalize(&PathBuf::from(s))
}

/// 规范化：转绝对（相对路径锚定到当前目录）、折叠 . 与 ..。
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        use std::path::Component::*;
        match comp {
            Prefix(c) => out.push(c.as_os_str()),
            RootDir => out.push("/"),
            CurDir => {}
            ParentDir => {
                out.pop();
            }
            Normal(c) => out.push(c),
        }
    }
    out
}

/// 取含通配符路径的静态前缀（第一个含 * ? [ 的段之前）。
fn wildcard_prefix(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        use std::path::Component::*;
        match comp {
            Normal(c) => {
                let s = c.to_string_lossy();
                if s.contains('*') || s.contains('?') || s.contains('[') {
                    break;
                }
                out.push(c);
            }
            Prefix(c) => out.push(c.as_os_str()),
            RootDir => out.push("/"),
            _ => {}
        }
    }
    out
}

/// 展开 glob，返回所有匹配的具体路径；无通配符时原样返回。
pub fn resolve_glob(p: &Path) -> Vec<PathBuf> {
    let s = p.to_string_lossy().to_string();
    let has_glob = s.contains('*') || s.contains('?') || s.contains('[');
    if !has_glob {
        return vec![p.to_path_buf()];
    }
    match glob::glob(&s) {
        Ok(paths) => paths.flatten().collect(),
        Err(_) => Vec::new(),
    }
}

/// 平台白名单根目录：规则只允许落在这些根之下。
fn allowed_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let local = dirs::data_local_dir().unwrap_or_else(|| home.clone());
    let config = dirs::config_dir().unwrap_or_else(|| home.clone());
    let temp = std::env::temp_dir();

    let push = |roots: &mut Vec<PathBuf>, p: PathBuf| {
        let n = normalize(&p);
        if !roots.iter().any(|r| r == &n) {
            roots.push(n);
        }
    };

    push(&mut roots, temp);
    push(&mut roots, local.clone());
    push(&mut roots, config);

    if cfg!(windows) {
        for sub in [
            ".cache",
            ".cargo",
            ".npm",
            ".ollama",
            ".lmstudio",
            ".zcode",
            ".zai",
            ".claude",
            ".codex",
            ".copilot",
            ".bito",
            "miniconda3",
            "anaconda3",
            "Documents/xwechat_files",
            "Documents/WeChat Files",
            "Documents/Tencent Files",
        ] {
            push(&mut roots, home.join(sub));
        }
        let windir = std::env::var("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:/Windows"));
        for sub in ["Temp", "Logs", "Minidump", "SoftwareDistribution", "Prefetch"] {
            push(&mut roots, windir.join(sub));
        }
        push(&mut roots, windir.join("MEMORY.DMP"));
        let programdata = std::env::var("PROGRAMDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:/ProgramData"));
        push(
            &mut roots,
            programdata.join("Microsoft/Windows"),
        );
        push(&mut roots, PathBuf::from("C:/Windows.old"));
        // 系统大文件（只报告，逐文件列入白名单）
        for f in ["hiberfil.sys", "pagefile.sys", "swapfile.sys"] {
            push(&mut roots, PathBuf::from(format!("C:/{f}")));
        }
    } else {
        for sub in [
            "Library/Caches",
            "Library/Logs",
            "Library/Application Support",
            "Library/Developer",
            "Library/Containers/com.docker.docker",
            "Library/Containers/com.openai.chat",
            "Library/Containers/com.anthropic.claudefordesktop",
            "Library/Jupyter",
            ".cache",
            ".npm",
            ".cargo",
            ".ollama",
            ".lmstudio",
            ".zcode",
            ".zai",
            ".claude",
            ".codex",
            ".copilot",
            ".bito",
            ".lingma",
        ] {
            push(&mut roots, home.join(sub));
        }
        push(&mut roots, PathBuf::from("/Library/Caches"));
    }
    roots
}

fn path_starts_with(path: &Path, root: &Path) -> bool {
    let norm = |p: &Path| p.to_string_lossy().replace('\\', "/").to_lowercase();
    let p = norm(path);
    let r = norm(root).trim_end_matches('/').to_string();
    p == r || p.starts_with(&format!("{r}/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_load_and_pass_safety_validation() {
        let all = load_all();
        assert!(all.len() >= 25, "内置规则数量异常: {}", all.len());
        let ids: Vec<&str> = all.iter().map(|r| r.id.as_str()).collect();
        for expected in [
            "system.temp.user",
            "browser.chrome",
            "ai.cursor",
            "ai.hf_incomplete",
            "apps.wechat_cache",
            "gpu.nvidia",
            "report.ollama_models",
        ] {
            assert!(ids.contains(&expected), "缺少规则 {expected}");
        }
        // 所有规则 id 唯一
        let mut sorted = ids.clone();
        sorted.sort();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(before, sorted.len(), "存在重复规则 id");
    }

    #[test]
    fn rejects_paths_outside_whitelist() {
        let bad = Rule {
            id: "test.bad".into(),
            name: "bad".into(),
            category: "system".into(),
            risk: "L1".into(),
            os: vec![],
            default_selected: false,
            delete: String::new(),
            description: String::new(),
            action: String::new(),
            services: None,
            targets: vec![Target {
                paths: vec!["C:/Users".into()],
                os: vec![],
                include: vec![],
                exclude: vec![],
            }],
        };
        assert!(validate(&bad).is_err());

        let parent = Rule {
            id: "test.parent".into(),
            name: "parent".into(),
            category: "system".into(),
            risk: "L1".into(),
            os: vec![],
            default_selected: false,
            delete: String::new(),
            description: String::new(),
            action: String::new(),
            services: None,
            targets: vec![Target {
                paths: vec!["${TEMP}/../Windows".into()],
                os: vec![],
                include: vec![],
                exclude: vec![],
            }],
        };
        assert!(validate(&parent).is_err());
    }
}

fn validate(rule: &Rule) -> Result<(), String> {
    let roots = allowed_roots();
    for target in &rule.targets {
        if !target.os.is_empty() && !target.os.iter().any(|o| o == current_platform()) {
            continue;
        }
        for raw in &target.paths {
            if raw.contains("..") {
                return Err(format!("路径包含 .. : {raw}"));
            }
            let expanded = expand_path(raw);
            if !expanded.is_absolute() {
                return Err(format!("展开后不是绝对路径: {raw}"));
            }
            let prefix = wildcard_prefix(&expanded);
            if !roots.iter().any(|r| path_starts_with(&prefix, r)) {
                return Err(format!("路径不在白名单根目录内: {raw}"));
            }
        }
    }
    Ok(())
}
