use std::path::Path;
use std::ptr;
use std::sync::OnceLock;
use std::thread;

use windows_sys::Win32::Foundation::{CloseHandle, HWND};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, TranslateMessage,
    EVENT_SYSTEM_FOREGROUND, MSG, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};

use super::Callback;

// WinEvent callbacks carry no user data, so the sink is global. `spawn` runs once.
static ON_FOCUS: OnceLock<Callback> = OnceLock::new();

pub fn spawn(on_focus: Callback) -> Result<(), String> {
    ON_FOCUS
        .set(on_focus)
        .map_err(|_| "focus watcher already running".to_string())?;
    thread::Builder::new()
        .name("focus-watcher".into())
        .spawn(|| unsafe {
            report(GetForegroundWindow());
            // Our own windows are skipped so opening the settings window doesn't trigger a switch.
            let hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                ptr::null_mut(),
                Some(on_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
            if hook.is_null() {
                eprintln!("SetWinEventHook failed");
                return;
            }
            // Out-of-context hooks are delivered through this thread's message loop.
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

unsafe extern "system" fn on_event(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    report(hwnd);
}

unsafe fn report(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    if let (Some(app), Some(cb)) = (exe_name(hwnd), ON_FOCUS.get()) {
        cb(app);
    }
}

unsafe fn exe_name(hwnd: HWND) -> Option<String> {
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == 0 {
        return None;
    }
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if process.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
    CloseHandle(process);
    if ok == 0 {
        return None;
    }
    app_id_for_path(Path::new(&String::from_utf16_lossy(&buf[..len as usize])))
}

/// File types offered when picking an app manually.
pub const APP_EXTENSIONS: &[&str] = &["exe"];

/// The identifier focus events report for an executable: its lowercase file name.
pub fn app_id_for_path(path: &Path) -> Option<String> {
    path.file_name().map(|n| n.to_string_lossy().to_lowercase())
}
