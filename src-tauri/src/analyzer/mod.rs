//! 大文件与目录占用分析（L0 只读）。
//! 个人目录只扫描、只报告，所有删除动作由用户在系统中自行完成。

use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BigFile {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_ts: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub file_count: u64,
}

/// 大文件扫描默认根目录：桌面/下载/文档/视频/图片/影片。
pub fn default_big_file_roots() -> Vec<PathBuf> {
    let home = dirs::home_dir();
    let mut roots = Vec::new();
    if let Some(h) = home {
        for sub in ["Desktop", "Downloads", "Documents", "Videos", "Pictures", "Movies", "音乐", "视频", "图片", "文档", "下载", "桌面"] {
            let p = h.join(sub);
            if p.is_dir() && !roots.iter().any(|x: &PathBuf| x == &p) {
                roots.push(p);
            }
        }
    }
    roots
}

/// 在给定根目录下查找大于阈值的文件，按大小降序，最多 limit 条。
pub fn big_files(roots: &[PathBuf], min_bytes: u64, limit: usize) -> Vec<BigFile> {
    let mut found: Vec<BigFile> = roots
        .par_iter()
        .filter(|p| p.is_dir())
        .flat_map_iter(|root| {
            walkdir::WalkDir::new(root)
                .follow_links(false)
                .into_iter()
                .flatten()
                .filter(|e| e.file_type().is_file())
                .filter_map(|e| {
                    let meta = e.metadata().ok()?;
                    let size = meta.len();
                    if size < min_bytes {
                        return None;
                    }
                    let modified_ts = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64);
                    Some(BigFile {
                        name: e.file_name().to_string_lossy().to_string(),
                        path: e.path().to_string_lossy().to_string(),
                        size_bytes: size,
                        modified_ts,
                    })
                })
        })
        .collect();
    found.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    found.truncate(limit);
    found
}

/// 统计某个目录一级子项各自的占用大小（目录会递归统计）。
pub fn child_sizes(root: &Path) -> Vec<DirEntry> {
    let Ok(read) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let children: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .collect();

    children
        .into_par_iter()
        .map(|p| {
            let is_dir = p.is_dir();
            let (size_bytes, file_count) = if is_dir {
                dir_size(&p)
            } else {
                (std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0), 1)
            };
            DirEntry {
                name: p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                path: p.to_string_lossy().to_string(),
                is_dir,
                size_bytes,
                file_count,
            }
        })
        .collect()
}

/// 递归统计目录总大小与文件数。
pub fn dir_size(root: &Path) -> (u64, u64) {
    let mut size = 0u64;
    let mut files = 0u64;
    for entry in walkdir::WalkDir::new(root).follow_links(false).into_iter().flatten() {
        if entry.file_type().is_file() {
            size = size.saturating_add(entry.metadata().map(|m| m.len()).unwrap_or(0));
            files += 1;
        }
    }
    (size, files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn big_files_and_child_sizes() {
        let tmp = std::env::temp_dir().join("cclean_analyzer_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sub")).unwrap();
        fs::write(tmp.join("a.bin"), vec![0u8; 2000]).unwrap();
        fs::write(tmp.join("sub/b.bin"), vec![0u8; 5000]).unwrap();

        let found = big_files(&[tmp.clone()], 3000, 100);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "b.bin");

        let children = child_sizes(&tmp);
        assert!(children.iter().any(|c| c.name == "sub" && c.size_bytes >= 5000));
        assert!(children.iter().any(|c| c.name == "a.bin"));

        let _ = fs::remove_dir_all(&tmp);
    }
}
