use std::ffi::c_void;

#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(process: isize, access: u32, token: *mut isize) -> i32;
    fn GetTokenInformation(token: isize, class: i32, info: *mut c_void, len: u32, returned: *mut u32) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> isize;
    fn CloseHandle(h: isize) -> i32;
}
#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(hwnd: isize, op: *const u16, file: *const u16, params: *const u16, dir: *const u16, show: i32) -> isize;
}

pub fn is_elevated() -> bool {
    unsafe {
        let mut token = 0isize;
        if OpenProcessToken(GetCurrentProcess(), 0x0008, &mut token) == 0 {
            return false;
        }
        let mut elevated = 0u32;
        let mut returned = 0u32;
        let ok = GetTokenInformation(token, 20, &mut elevated as *mut u32 as *mut c_void, 4, &mut returned);
        CloseHandle(token);
        ok != 0 && elevated != 0
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn relaunch_elevated() -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    let dir = exe.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let result = unsafe {
        ShellExecuteW(
            0,
            wide("runas").as_ptr(),
            wide(&exe.to_string_lossy()).as_ptr(),
            wide("--relaunched").as_ptr(),
            wide(&dir).as_ptr(),
            1,
        )
    };
    result > 32
}
