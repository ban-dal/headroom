mod doctor;
mod inject;
mod proxy;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{
    Manager, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

use proxy::ProxyState;

const DASHBOARD_URL: &str = "http://127.0.0.1:8787/dashboard";

/// Show or hide the popover window anchored near the tray.
fn toggle_window(app: &tauri::AppHandle, anchor: Option<PhysicalPosition<f64>>) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            if let Some(anchor) = anchor {
                if let Ok(size) = win.outer_size() {
                    let mut x = anchor.x - f64::from(size.width) / 2.0;
                    let mut y = anchor.y + 8.0;

                    let monitor = app
                        .monitor_from_point(anchor.x, anchor.y)
                        .ok()
                        .flatten()
                        .or_else(|| win.current_monitor().ok().flatten());

                    if let Some(monitor) = monitor {
                        let work = monitor.work_area();
                        let min_x = f64::from(work.position.x);
                        let min_y = f64::from(work.position.y);
                        let max_x = min_x + f64::from(work.size.width) - f64::from(size.width);
                        let max_y = min_y + f64::from(work.size.height) - f64::from(size.height);
                        x = x.clamp(min_x, max_x);
                        y = y.clamp(min_y, max_y);
                    }

                    let _ = win.set_position(Position::Physical(PhysicalPosition::new(
                        x.round() as i32,
                        y.round() as i32,
                    )));
                }
            }
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

#[tauri::command]
fn set_pinned(app: tauri::AppHandle, pinned: bool) -> Result<bool, String> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    win.set_always_on_top(pinned).map_err(|e| e.to_string())?;
    Ok(pinned)
}

#[tauri::command]
fn open_dashboard_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("dashboard") {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(
        &app,
        "dashboard",
        WebviewUrl::External(DASHBOARD_URL.parse::<Url>().map_err(|e| e.to_string())?),
    )
    .title("Headroom Dashboard")
    .inner_size(1120.0, 760.0)
    .resizable(true)
    .decorations(true)
    .always_on_top(false)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ProxyState::default())
        .invoke_handler(tauri::generate_handler![
            proxy::start_proxy,
            proxy::stop_proxy,
            proxy::proxy_path,
            doctor::doctor_status,
            inject::intercept_on,
            inject::intercept_off,
            set_pinned,
            open_dashboard_window,
        ])
        .setup(|app| {
            // Menu-bar app: no Dock icon.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let toggle = MenuItem::with_id(app, "toggle", "Open Headroom", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &quit])?;

            let tray_icon =
                tauri::image::Image::from_bytes(include_bytes!("../icons/tray/tray@2x.png"))?;

            TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("Headroom")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => toggle_window(app, None),
                    "quit" => {
                        if let Some(state) = app.try_state::<ProxyState>() {
                            let _ = proxy::stop_proxy(state);
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle(), Some(position));
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide instead of closing so the app keeps living in the menu bar.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            // Reap the proxy child when the app fully exits.
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app.try_state::<ProxyState>() {
                    let _ = proxy::stop_proxy(state);
                }
            }
        });
}
