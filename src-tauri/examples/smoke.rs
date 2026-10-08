//! 冒烟测试：在真实机器上加载规则并扫描少量缓存目录（不删除任何文件）。
//! 运行：cargo run --example smoke

use cclean_lib::{ai_report, analyzer, platform, rules, scanner, snapshot, storage::Storage};

fn main() {
    let overview = platform::overview();
    println!("== 系统 ==");
    println!("OS: {} ({}) elevated={}", overview.os, overview.platform, overview.elevated);
    for d in &overview.disks {
        let pct = if d.total_bytes > 0 {
            d.used_bytes * 100 / d.total_bytes
        } else {
            0
        };
        println!(
            "磁盘 {} 已用 {}% 可用 {} / {}",
            d.mount_name,
            pct,
            fmt(d.available_bytes),
            fmt(d.total_bytes)
        );
    }

    let all = rules::load_all();
    println!("\n== 规则 ==\n加载 {} 条（当前平台）", all.len());

    let probe = [
        "system.thumbcache",
        "system.wer_user",
        "gpu.d3d",
        "gpu.nvidia",
        "ai.cursor",
        "ai.vscode",
        "dev.npm",
        "browser.chrome",
        "report.ollama_models",
        "report.hf_models",
        "ai.doubao",
        "ai.workbuddy",
        "ai.codebuddy",
        "ai.zcode",
        "ai.zcode_exec",
        "ai.claude_code",
        "ai.codex_cli",
        "ai.copilot_cli",
        "ai.bito",
        "ai.lingma_cli",
    ];
    let ids: Vec<String> = probe.iter().map(|s| s.to_string()).collect();
    let stats = scanner::scan_rules(&all, Some(&ids));
    println!("\n== 扫描结果（Dry-run，不删除）==");
    for s in &stats {
        let existing = s.targets.iter().filter(|t| t.exists).count();
        println!(
            "{:<26} {:>10}  文件 {:>6}  命中目录 {}/{}  错误 {}",
            s.rule.id,
            fmt(s.size_bytes),
            s.file_count,
            existing,
            s.targets.len(),
            s.error_count
        );
    }

    println!("\n== AI 模型报告（只报告）==");
    for g in ai_report::build_report(&all) {
        if !g.available {
            println!("{:<26} 未安装", g.rule_id);
            continue;
        }
        println!("{:<26} {:>10}  子项 {}", g.rule_id, fmt(g.size_bytes), g.entries.len());
        if !g.cli_output.is_empty() {
            println!("  [ollama list] {}", g.cli_output.replace('\n', " | "));
        }
    }

    println!("\n== 大文件扫描（默认用户目录，≥500MB，前 10）==");
    let roots = analyzer::default_big_file_roots();
    let big = analyzer::big_files(&roots, 500 * 1024 * 1024, 10);
    if big.is_empty() {
        println!("未发现 ≥500MB 的个人文件");
    }
    for f in &big {
        println!("{:>10}  {}", fmt(f.size_bytes), f.path);
    }

    if let Some(home) = dirs::home_dir() {
        println!("\n== 用户目录一级占用 Top 8 ==");
        let mut children = analyzer::child_sizes(&home);
        children.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        for c in children.into_iter().take(8) {
            println!("{:>10}  {}", fmt(c.size_bytes), c.name);
        }
    }

    println!("\n== 快照入库（临时库，不写应用数据）==");
    let db = std::env::temp_dir().join(format!("cclean_smoke_{}.db", std::process::id()));
    let store = Storage::open(&db).expect("打开临时数据库失败");
    let snap = snapshot::take(&store, &all).expect("快照采集失败");
    println!(
        "快照 #{}：{} 剩余 {} / {}",
        snap.id, snap.disk_name, fmt(snap.disk_free as u64), fmt(snap.disk_total as u64)
    );
    let trend = store.trend(10).unwrap();
    println!("趋势查询：快照 {} 条，分项 {} 条", trend.snapshots.len(), trend.items.len());
    store.insert_clean_log("dev.npm", "", 12345, "ok", "").unwrap();
    println!("清理日志：{} 条", store.clean_logs(10).unwrap().len());
    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_file(db.with_extension("db-wal"));
    let _ = std::fs::remove_file(db.with_extension("db-shm"));
}

fn fmt(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes}B");
    }
    let mut v = bytes as f64 / 1024.0;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    format!("{:.1}{}", v, UNITS[i])
}
