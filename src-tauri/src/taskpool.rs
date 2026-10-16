//! 扫描任务专用线程池。
//!
//! 扫描/统计会大量遍历文件树并占用 CPU 与磁盘 IO。若直接使用 rayon 全局线程池
//! （默认占满全部逻辑核心），在基线快照、AI 报告、页面扫描同时进行时会把
//! WebView 渲染进程也拖到近乎无响应。这里统一使用一个有界的独立线程池：
//! 始终为 UI 与系统保留至少两个核心，且扫描线程上限收敛。

use rayon::ThreadPool;
use std::sync::OnceLock;

fn pool() -> &'static ThreadPool {
    static POOL: OnceLock<ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        // 保留至少 2 个核心给 UI/系统；扫描池最多 4 个线程（IO 密集型任务收益有限）
        let threads = cpus.saturating_sub(2).clamp(1, 4);
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("cclean-scan-{i}"))
            .build()
            .expect("无法创建扫描线程池")
    })
}

/// 在有界扫描线程池内执行一段并行任务。
/// 允许同一线程池嵌套调用（rayon 对同池 install 会直接执行，不会死锁）。
pub fn run<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    pool().install(f)
}
