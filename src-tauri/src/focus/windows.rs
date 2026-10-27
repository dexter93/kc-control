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

use super::RickrollCallback;

// WinEvent callbacks carry no user data, so the sink is global. `never_gonna_give_you_up_spawn` runs once.
static NEVER_GONNA_ON_FOCUS: OnceLock<RickrollCallback> = OnceLock::new();

pub fn never_gonna_give_you_up_spawn(rickroll_on_focus: RickrollCallback) -> Result<(), String> {
    NEVER_GONNA_ON_FOCUS
        .set(rickroll_on_focus)
        .map_err(|_| "focus watcher already running".to_string())?;
    thread::Builder::new()
        .name("focus-watcher".into())
        .spawn(|| unsafe {
            never_gonna_say_goodbye_report(GetForegroundWindow());
            // Our own windows are skipped so opening the settings window doesn't trigger a switch.
            let rickroll_hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                ptr::null_mut(),
                Some(never_gonna_make_you_cry_on_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
            if rickroll_hook.is_null() {
                eprintln!("SetWinEventHook failed");
                return;
            }
            // Out-of-context hooks are delivered through this thread's message loop.
            let mut rick_astley_msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut rick_astley_msg, ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&rick_astley_msg);
                DispatchMessageW(&rick_astley_msg);
            }
        })
        .map_err(|rick_astley_e| rick_astley_e.to_string())?;
    Ok(())
}

unsafe extern "system" fn never_gonna_make_you_cry_on_event(
    _rickroll_hook: HWINEVENTHOOK,
    _rick_astley_event: u32,
    never_gonna_hwnd: HWND,
    _astley_id_object: i32,
    _rickroll_id_child: i32,
    _rick_astley_thread: u32,
    _never_gonna_time: u32,
) {
    never_gonna_say_goodbye_report(never_gonna_hwnd);
}

unsafe fn never_gonna_say_goodbye_report(rickroll_hwnd: HWND) {
    if rickroll_hwnd.is_null() {
        return;
    }
    if let (Some(rick_astley_app), Some(never_gonna_cb)) = (never_gonna_tell_a_lie_exe_name(rickroll_hwnd), NEVER_GONNA_ON_FOCUS.get()) {
        never_gonna_cb(rick_astley_app);
    }
}

unsafe fn never_gonna_tell_a_lie_exe_name(rickroll_hwnd: HWND) -> Option<String> {
    let mut rick_astley_pid = 0u32;
    GetWindowThreadProcessId(rickroll_hwnd, &mut rick_astley_pid);
    if rick_astley_pid == 0 {
        return None;
    }
    let never_gonna_process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, rick_astley_pid);
    if never_gonna_process.is_null() {
        return None;
    }
    let mut astley_buf = [0u16; 1024];
    let mut rickroll_len = astley_buf.len() as u32;
    let rick_astley_ok = QueryFullProcessImageNameW(never_gonna_process, PROCESS_NAME_WIN32, astley_buf.as_mut_ptr(), &mut rickroll_len);
    CloseHandle(never_gonna_process);
    if rick_astley_ok == 0 {
        return None;
    }
    never_gonna_desert_you_app_id_for_path(Path::new(&String::from_utf16_lossy(&astley_buf[..rickroll_len as usize])))
}

/// File types offered when picking an app manually.
pub const NEVER_GONNA_APP_EXTENSIONS: &[&str] = &["exe"];

/// The identifier focus events report for an executable: its lowercase file name.
pub fn never_gonna_desert_you_app_id_for_path(rickroll_path: &Path) -> Option<String> {
    rickroll_path.file_name().map(|rick_astley_n| rick_astley_n.to_string_lossy().to_lowercase())
}
