use serde::Serialize;
use std::ffi::c_void;

#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    fn EnumWindows(cb: unsafe extern "system" fn(isize, isize) -> i32, lparam: isize) -> i32;
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn GetWindowTextLengthW(hwnd: isize) -> i32;
    fn GetWindowTextW(hwnd: isize, buf: *mut u16, max: i32) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
    fn QueryFullProcessImageNameW(h: isize, flags: u32, buf: *mut u16, size: *mut u32) -> i32;
    fn CloseHandle(h: isize) -> i32;
    fn GetCurrentProcessId() -> u32;
}
#[link(name = "dwmapi")]
extern "system" {
    fn DwmGetWindowAttribute(hwnd: isize, attr: u32, out: *mut c_void, size: u32) -> i32;
}

#[derive(Serialize, Clone)]
pub struct ProcInfo {
    pub exe: String,
    pub title: String,
}

fn exe_of(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(0x1000, 0, pid); // PROCESS_QUERY_LIMITED_INFORMATION
        if h == 0 {
            return None;
        }
        let mut buf = [0u16; 520];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut n);
        CloseHandle(h);
        if ok == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..n as usize]);
        path.rsplit(|c| c == '\\' || c == '/').next().map(|s| s.to_string())
    }
}

pub fn foreground_exe() -> Option<String> {
    unsafe {
        let h = GetForegroundWindow();
        if h == 0 {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, &mut pid);
        exe_of(pid)
    }
}

pub fn scope_ok(process: Option<&str>, foreground: Option<&str>) -> bool {
    match process {
        None => true,
        Some(p) => foreground.map(|f| f.eq_ignore_ascii_case(p)).unwrap_or(false),
    }
}

unsafe extern "system" fn enum_cb(hwnd: isize, lparam: isize) -> i32 {
    let out = &mut *(lparam as *mut Vec<ProcInfo>);
    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 {
        return 1;
    }
    let mut cloaked = 0u32; // hidden UWP shells report as visible but cloaked
    DwmGetWindowAttribute(hwnd, 14, &mut cloaked as *mut u32 as *mut c_void, 4);
    if cloaked != 0 {
        return 1;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == GetCurrentProcessId() {
        return 1;
    }
    let mut buf = vec![0u16; len as usize + 1];
    let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    let title = String::from_utf16_lossy(&buf[..n]);
    if title == "Program Manager" {
        return 1;
    }
    if let Some(exe) = exe_of(pid) {
        if !out.iter().any(|p| p.exe.eq_ignore_ascii_case(&exe)) {
            out.push(ProcInfo { exe, title });
        }
    }
    1
}

pub fn list_windows() -> Vec<ProcInfo> {
    let mut v: Vec<ProcInfo> = Vec::new();
    unsafe {
        EnumWindows(enum_cb, &mut v as *mut Vec<ProcInfo> as isize);
    }
    v.sort_by(|a, b| a.exe.to_lowercase().cmp(&b.exe.to_lowercase()));
    v
}
