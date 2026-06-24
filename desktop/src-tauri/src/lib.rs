mod doctor;
mod inject;
mod proxy;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};

use proxy::ProxyState;

/// Show or hide the popover window anchored near the tray.
fn toggle_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
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
        ])
        .setup(|app| {
            // Menu-bar app: no Dock icon.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let toggle = MenuItem::with_id(app, "toggle", "Open Headroom", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &quit])?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!(
                "../icons/tray/tray@2x.png"
            ))?;

            TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("Headroom")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => toggle_window(app),
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
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
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
