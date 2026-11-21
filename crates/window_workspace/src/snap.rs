#[cfg(any(windows, test))]
fn snap_axis(start: i32, end: i32, low: i32, high: i32, threshold: i32) -> i32 {
    let near = low - start;
    let far = high - end;
    if near.abs() <= threshold && near.abs() <= far.abs() {
        low
    } else if far.abs() <= threshold {
        start + far
    } else {
        start
    }
}

#[cfg(windows)]
pub(crate) fn arm(threshold: u16) {
    use windows_sys::Win32::UI::{
        Input::KeyboardAndMouse::GetActiveWindow, Shell::SetWindowSubclass,
    };
    // Called on the event thread during a primary press on the native viewport's grip.
    unsafe {
        let window = GetActiveWindow();
        if !window.is_null() {
            SetWindowSubclass(window, Some(after_drag), 1, threshold.into());
        }
    }
}
#[cfg(not(windows))]
pub(crate) fn arm(_: u16) {}

#[cfg(windows)]
unsafe extern "system" fn after_drag(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    w: usize,
    l: isize,
    id: usize,
    data: usize,
) -> isize {
    use windows_sys::Win32::{
        Foundation::RECT,
        Graphics::Gdi::*,
        UI::{Shell::*, WindowsAndMessaging::*},
    };
    if msg != WM_EXITSIZEMOVE && msg != WM_NCDESTROY {
        return DefSubclassProc(hwnd, msg, w, l);
    }
    RemoveWindowSubclass(hwnd, Some(after_drag), id);
    let result = DefSubclassProc(hwnd, msg, w, l);
    if msg == WM_EXITSIZEMOVE && IsWindow(hwnd) != 0 && IsZoomed(hwnd) == 0 {
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let mut rect: RECT = std::mem::zeroed();
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if GetMonitorInfoW(monitor, &mut info) != 0 && GetWindowRect(hwnd, &mut rect) != 0 {
            let area = info.rcWork;
            let x = snap_axis(rect.left, rect.right, area.left, area.right, data as i32);
            let y = snap_axis(rect.top, rect.bottom, area.top, area.bottom, data as i32);
            if (x, y) != (rect.left, rect.top) {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::snap_axis;
    #[test]
    fn edges_and_threshold() {
        assert_eq!(snap_axis(14, 214, 0, 1080, 14), 0);
        assert_eq!(snap_axis(15, 215, 0, 1080, 14), 15);
        assert_eq!(snap_axis(887, 1087, 0, 1080, 14), 880);
        assert_eq!(snap_axis(-1908, -1508, -1920, 0, 14), -1920);
        assert_eq!(snap_axis(800, 1000, 0, 1040, 14), 800);
    }
}

#[cfg(all(test, windows))]
#[test]
fn native_window_snaps_on_move_completion() {
    use windows_sys::Win32::{
        Foundation::RECT,
        Graphics::Gdi::*,
        UI::{Shell::SetWindowSubclass, WindowsAndMessaging::*},
    };
    // The hidden window has its own handle and never takes the user's mouse or focus.
    unsafe {
        let class: Vec<u16> = "STATIC".encode_utf16().chain(Some(0)).collect();
        let null = std::ptr::null_mut();
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            30,
            30,
            300,
            120,
            null,
            null,
            null,
            std::ptr::null(),
        );
        assert!(!hwnd.is_null());
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        assert_ne!(
            GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info),
            0
        );
        let a = info.rcWork;
        assert_ne!(
            SetWindowPos(
                hwnd,
                null,
                a.right - 307,
                a.top + 14,
                300,
                120,
                SWP_NOACTIVATE | SWP_NOZORDER
            ),
            0
        );
        assert_ne!(SetWindowSubclass(hwnd, Some(after_drag), 1, 14), 0);
        SendMessageW(hwnd, WM_EXITSIZEMOVE, 0, 0);
        let mut r: RECT = std::mem::zeroed();
        assert_ne!(GetWindowRect(hwnd, &mut r), 0);
        DestroyWindow(hwnd);
        assert_eq!(
            (r.right, r.top, r.right - r.left, r.bottom - r.top),
            (a.right, a.top, 300, 120)
        );
    }
}
