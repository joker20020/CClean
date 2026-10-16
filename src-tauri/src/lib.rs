pub mod ai_report;
pub mod analyzer;
pub mod appdir;
pub mod cleaner;
pub mod commands;
pub mod floatwin;
pub mod i18n;
pub mod platform;
pub mod privilege;
pub mod resident;
pub mod rules;
pub mod scanner;
pub mod snapshot;
pub mod storage;
pub mod taskpool;
pub mod watcher;

use std::sync::{Arc, Mutex, RwLock};

use tauri::{Emitter, Manager, Wry, WindowEvent};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

use rules::Rule;
use storage::Storage;

/// 托盘菜单中浮窗相关勾选项的句柄，供设置页/浮窗等其他入口修改后同步勾选状态。
pub struct TrayItemHandles {
    pub float: CheckMenuItem<Wry>,
    pub through: CheckMenuItem<Wry>,
}

pub struct AppState {
    pub rules: Arc<RwLock<Vec<Rule>>>,
    pub storage: Arc<Storage>,
    pub tray_items: Mutex<Option<TrayItemHandles>>,
}

impl AppState {
    /// 重新从磁盘加载内置 + 用户规则（自定义规则增删后调用，扫描/清理立即生效；
    /// 目录监听的 watcher 需重启后刷新）。
    pub fn reload_rules(&self) {
        if let Ok(mut w) = self.rules.write() {
            *w = rules::load_all_with_user();
        }
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

/// 构建（或按当前语言重建）系统托盘图标与菜单。
pub fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let lang = i18n::from_storage(&state.storage);
    let strings = i18n::Strings::get(lang);

    // 语言切换后重建：先移除旧图标，避免出现多个托盘图标
    let _ = app.remove_tray_by_id("main-tray");

    let item_snapshot = MenuItem::with_id(app, "snapshot", strings.tray_snapshot, true, None::<&str>)?;
    let item_show = MenuItem::with_id(app, "show", strings.tray_show, true, None::<&str>)?;
    let float_on = state
        .storage
        .get_setting(floatwin::SETTING_FLOAT_ENABLED)
        .ok()
        .flatten()
        .as_deref()
        == Some("true");
    let through_on = state
        .storage
        .get_setting(floatwin::SETTING_FLOAT_THROUGH)
        .ok()
        .flatten()
        .as_deref()
        == Some("true");
    let item_float = CheckMenuItem::with_id(app, "float", strings.tray_float, true, float_on, None::<&str>)?;
    let item_through = CheckMenuItem::with_id(
        app,
        "float-through",
        strings.tray_through,
        true,
        through_on,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let item_quit = MenuItem::with_id(app, "quit", strings.tray_quit, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &item_snapshot,
            &item_show,
            &sep,
            &item_float,
            &item_through,
            &item_quit,
        ],
    )?;
    // 保存勾选项句柄，供浮窗状态在其他入口（设置页/浮窗按钮）变更时同步托盘
    *state.tray_items.lock().unwrap() = Some(TrayItemHandles {
        float: item_float.clone(),
        through: item_through.clone(),
    });

    TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip(strings.tray_tooltip)
        .on_menu_event({
            let app_h = app.clone();
            let item_float_h = item_float.clone();
            let item_through_h = item_through.clone();
            move |app, event| match event.id.as_ref() {
                "snapshot" => {
                    let state = app.state::<AppState>();
                    let storage = state.storage.clone();
                    let rules = state.rules.read().unwrap().clone();
                    let app_h2 = app_h.clone();
                    std::thread::spawn(move || match snapshot::take(&storage, &rules) {
                        Ok(snap) => {
                            let _ = storage.set_setting(
                                resident::SETTING_LAST_SNAPSHOT,
                                &snap.created_at.to_string(),
                            );
                            let _ = app_h2.emit("snapshot-taken", snap);
                            let lang = i18n::from_storage(&storage);
                            let s = i18n::Strings::get(lang);
                            use tauri_plugin_notification::NotificationExt;
                            let _ = app_h2
                                .notification()
                                .builder()
                                .title(s.snap_title)
                                .body(s.snap_body)
                                .show();
                        }
                        Err(e) => eprintln!("[tray] 快照失败: {e}"),
                    });
                }
                "float" => {
                    let next = !item_float_h.is_checked().unwrap_or(false);
                    match floatwin::set_enabled(app, next) {
                        Ok(v) => {
                            let _ = item_float_h.set_checked(v);
                        }
                        Err(e) => eprintln!("[float] 开关失败: {e}"),
                    }
                }
                "float-through" => {
                    let next = !item_through_h.is_checked().unwrap_or(false);
                    match floatwin::set_click_through(app, next) {
                        Ok(v) => {
                            let _ = item_through_h.set_checked(v);
                        }
                        Err(e) => eprintln!("[float] 穿透切换失败: {e}"),
                    }
                }
                "show" => show_main_window(app),
                "quit" => {
                    if let Some(float) = app.get_webview_window("float") {
                        let _ = float.destroy();
                    }
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let rules = Arc::new(RwLock::new(rules::load_all_with_user()));

    // 数据目录：迁移旧标识目录并确保目录（含 rules/）存在
    // Windows %APPDATA%\<identifier>，macOS ~/Library/Application Support/<identifier>
    let db_dir = appdir::ensure_app_dirs().expect("无法初始化应用数据目录");
    let storage = Arc::new(
        Storage::open(&db_dir.join("cclean.db")).expect("无法打开数据库 cclean.db"),
    );

    tauri::Builder::default()
        // 单实例保护：必须最先注册。已有实例运行时再次启动不会创建第二个
        // WebView2（否则报 0x800700AA “请求的资源在使用中”），而是唤起已有主窗口
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            rules: rules.clone(),
            storage: storage.clone(),
            tray_items: Mutex::new(None),
        })
        .setup(move |app| {
            // 首次启动自动记录一条基线快照（后台线程，不阻塞 UI）
            {
                let app_h = app.handle().clone();
                let storage_h = storage.clone();
                let rules_h = rules.read().unwrap().clone();
                std::thread::spawn(move || {
                    let is_empty = storage_h
                        .list_snapshots(1)
                        .map(|s| s.is_empty())
                        .unwrap_or(true);
                    if is_empty {
                        if let Ok(snap) = snapshot::take(&storage_h, &rules_h) {
                            let _ = storage_h.set_setting(
                                resident::SETTING_LAST_SNAPSHOT,
                                &snap.created_at.to_string(),
                            );
                            let _ = app_h.emit("snapshot-taken", snap);
                        }
                    }
                });
            }

            // 恢复浮窗位置/穿透/可见性
            floatwin::apply_state(app.handle());

            // 系统托盘（按当前语言构建，语言切换时由 set_settings 重建）
            build_tray(app.handle())?;

            // 关闭窗口时：常驻模式隐藏到托盘，否则彻底退出
            // （浮窗也是一个窗口，若仅销毁主窗口，进程会因浮窗存活而驻留，
            //  且主窗口无法再创建——因此非常驻模式必须显式退出整个应用）
            let win = app.get_webview_window("main").unwrap();
            win.on_window_event({
                let app_h = app.handle().clone();
                move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        let state = app_h.state::<AppState>();
                        let resident = state
                            .storage
                            .get_setting(resident::SETTING_RESIDENT)
                            .ok()
                            .flatten()
                            .map(|v| v == "true")
                            .unwrap_or(false);
                        if resident {
                            api.prevent_close();
                            if let Some(w) = app_h.get_webview_window("main") {
                                let _ = w.hide();
                            }
                        } else {
                            // 先立即销毁浮窗窗口，再退出整个应用。
                            // 否则进程结束时浮窗 WebView2 仍存活，Chromium 会报
                            // "Failed to unregister class Chrome_WidgetWin_0 (1412)"
                            if let Some(float) = app_h.get_webview_window("float") {
                                let _ = float.destroy();
                            }
                            app_h.exit(0);
                        }
                    }
                }
            });

            // 启动后立即检查一次低空间（延迟 4 秒，等通知子系统就绪）
            {
                let app_h = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(4));
                    resident::check_now(&app_h);
                });
            }

            // 后台定时快照 / 低空间通知 / 缓存目录监听
            resident::spawn(app.handle().clone());
            watcher::spawn(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_system_overview,
            commands::get_rules,
            commands::scan,
            commands::clean,
            commands::open_path,
            commands::reveal_item,
            commands::get_ai_report,
            commands::list_big_files,
            commands::list_dir_sizes,
            commands::default_roots,
            commands::get_brew_status,
            commands::run_brew_cleanup,
            commands::take_snapshot,
            commands::get_trend,
            commands::get_clean_logs,
            commands::get_fs_events,
            commands::delete_clean_log,
            commands::clear_clean_logs,
            commands::delete_fs_event,
            commands::clear_fs_events,
            commands::get_settings,
            commands::set_settings,
            commands::snooze_low_space,
            commands::list_custom_rules,
            commands::custom_rules_dir,
            commands::save_custom_rule,
            commands::delete_custom_rule,
            commands::validate_custom_rule,
            commands::set_float_enabled,
            commands::set_float_click_through,
            commands::show_main_window,
            commands::get_privilege_status,
            commands::relaunch_elevated_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CClean");
}
