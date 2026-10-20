// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod focus;
mod mouse;
mod switcher;

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use serde::Serialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_dialog::DialogExt;

use config::Config;
use switcher::{Msg, Status, StatusHandle};

struct AppState {
    config: Mutex<Config>,
    config_path: PathBuf,
    status: StatusHandle,
    switcher: Mutex<Sender<Msg>>,
    tray_status: MenuItem<Wry>,
}

#[derive(Serialize)]
struct Snapshot {
    config: Config,
    status: Status,
}

#[tauri::command]
fn get_state(state: State<AppState>) -> Snapshot {
    Snapshot {
        config: state.config.lock().unwrap().clone(),
        status: state.status.lock().unwrap().clone(),
    }
}

#[tauri::command]
fn save_config(app: AppHandle, state: State<AppState>, config: Config) -> Result<(), String> {
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = config.clone();
    let _ = state.switcher.lock().unwrap().send(Msg::Config(config));
    refresh_tray(&app);
    Ok(())
}

/// Lets the user browse for an app and returns the identifier rules match on.
#[tauri::command]
async fn pick_app(app: AppHandle) -> Option<String> {
    let mut dialog = app.dialog().file().set_title("Choose an app");
    if !focus::APP_EXTENSIONS.is_empty() {
        dialog = dialog.add_filter("Applications", focus::APP_EXTENSIONS);
    }
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    let path = dialog.blocking_pick_file()?.into_path().ok()?;
    focus::app_id_for_path(&path)
}

/// Whether the app launches at sign-in. The OS entry is the source of truth, not our config.
#[tauri::command]
fn get_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    }
    .map_err(|e| e.to_string())
}

fn status_text(config: &Config, status: &Status) -> String {
    match (status.connected, status.profile) {
        (false, _) => "Mouse not connected".into(),
        (true, None) => "Mouse asleep".into(),
        (true, Some(p)) => format!("Active: {}", config.profile_name(p)),
    }
}

fn refresh_tray(app: &AppHandle) {
    // The switcher can report before setup has registered the state.
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let text = status_text(&state.config.lock().unwrap(), &state.status.lock().unwrap());
    let _ = state.tray_status.set_text(&text);
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(format!("KC Control — {text}")));
    }
}

fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_settings(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_config,
            pick_app,
            get_autostart,
            set_autostart
        ])
        .setup(|app| {
            let config_path = app.path().app_config_dir()?.join("config.json");
            let config = Config::load(&config_path);

            let tray_status = MenuItem::with_id(app, "status", "Starting…", false, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &tray_status,
                    &PredefinedMenuItem::separator(app)?,
                    &MenuItem::with_id(app, "settings", "Configure…", true, None::<&str>)?,
                    &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
                ],
            )?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().cloned().expect("bundle icon"))
                .tooltip("KC Control")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "settings" => show_settings(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_settings(tray.app_handle());
                    }
                })
                .build(app)?;

            let handle = app.handle().clone();
            let (tx, status) = switcher::spawn(config.clone(), move |status| {
                let _ = handle.emit("status", status);
                refresh_tray(&handle);
            });

            let focus_tx = Mutex::new(tx.clone());
            if let Err(e) = focus::spawn(Box::new(move |app_id| {
                let _ = focus_tx.lock().unwrap().send(Msg::Focus(app_id));
            })) {
                eprintln!("focus tracking unavailable: {e}");
            }

            app.manage(AppState {
                config: Mutex::new(config),
                config_path,
                status,
                switcher: Mutex::new(tx),
                tray_status,
            });
            refresh_tray(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the settings window just hides it; the app lives in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running KC Control");
}
