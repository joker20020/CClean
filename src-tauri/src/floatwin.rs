//! M4 桌面浮窗：透明、置顶、无边框、可切换鼠标穿透、边缘吸附（前端实现）。

use tauri::{AppHandle, Emitter, LogicalPosition, Manager};

pub const SETTING_FLOAT_ENABLED: &str = "float_enabled";
pub const SETTING_FLOAT_THROUGH: &str = "float_click_through";
pub const SETTING_FLOAT_X: &str = "float_x";
pub const SETTING_FLOAT_Y: &str = "float_y";

pub const FLOAT_LABEL: &str = "float";

/// 浮窗状态变化广播事件（启用/关闭、鼠标穿透）。
/// 前端监听后同步设置页开关；后端同时同步托盘菜单勾选状态。
pub const FLOAT_STATE_EVENT: &str = "float-state-changed";

#[derive(serde::Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
struct FloatStatePayload {
    enabled: bool,
    click_through: bool,
}

fn get_setting(app: &AppHandle, key: &str) -> Option<String> {
    let state = app.try_state::<crate::AppState>()?;
    state.storage.get_setting(key).ok().flatten()
}

/// 读取当前浮窗状态。
fn current_state(app: &AppHandle) -> FloatStatePayload {
    FloatStatePayload {
        enabled: get_setting(app, SETTING_FLOAT_ENABLED).as_deref() == Some("true"),
        click_through: get_setting(app, SETTING_FLOAT_THROUGH).as_deref() == Some("true"),
    }
}

/// 同步托盘菜单的勾选状态（设置页、浮窗按钮等其他入口修改时）。
fn sync_tray(app: &AppHandle, state: FloatStatePayload) {
    let Some(app_state) = app.try_state::<crate::AppState>() else {
        return;
    };
    let Ok(guard) = app_state.tray_items.lock() else {
        return;
    };
    if let Some(handles) = guard.as_ref() {
        let _ = handles.float.set_checked(state.enabled);
        let _ = handles.through.set_checked(state.click_through);
    }
}

/// 广播最新浮窗状态并同步托盘菜单。
fn notify_changed(app: &AppHandle) {
    let state = current_state(app);
    sync_tray(app, state);
    let _ = app.emit(FLOAT_STATE_EVENT, state);
}

/// 启动时按设置恢复浮窗位置、穿透状态与可见性。
pub fn apply_state(app: &AppHandle) {
    let Some(win) = app.get_webview_window(FLOAT_LABEL) else {
        return;
    };
    if let (Some(x), Some(y)) = (
        get_setting(app, SETTING_FLOAT_X).and_then(|v| v.parse::<i32>().ok()),
        get_setting(app, SETTING_FLOAT_Y).and_then(|v| v.parse::<i32>().ok()),
    ) {
        let _ = win.set_position(LogicalPosition::new(x as f64, y as f64));
    }
    let through = get_setting(app, SETTING_FLOAT_THROUGH).as_deref() == Some("true");
    let _ = win.set_ignore_cursor_events(through).map_err(|e| {
        eprintln!("[float] 设置穿透失败: {e}");
    });
    let enabled = get_setting(app, SETTING_FLOAT_ENABLED).as_deref() == Some("true");
    if enabled {
        let _ = win.show();
    }
}

/// 开关浮窗，返回最新状态。
pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    let state = app
        .try_state::<crate::AppState>()
        .ok_or_else(|| "应用状态未就绪".to_string())?;
    state
        .storage
        .set_setting(SETTING_FLOAT_ENABLED, if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    let Some(win) = app.get_webview_window(FLOAT_LABEL) else {
        return Ok(enabled);
    };
    if enabled {
        win.show().map_err(|e| e.to_string())?;
        win.set_always_on_top(true).map_err(|e| e.to_string())?;
    } else {
        win.hide().map_err(|e| e.to_string())?;
    }
    notify_changed(app);
    Ok(enabled)
}

/// 切换鼠标穿透。
pub fn set_click_through(app: &AppHandle, through: bool) -> Result<bool, String> {
    let state = app
        .try_state::<crate::AppState>()
        .ok_or_else(|| "应用状态未就绪".to_string())?;
    state
        .storage
        .set_setting(SETTING_FLOAT_THROUGH, if through { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    if let Some(win) = app.get_webview_window(FLOAT_LABEL) {
        win.set_ignore_cursor_events(through)
            .map_err(|e| e.to_string())?;
    }
    notify_changed(app);
    Ok(through)
}
