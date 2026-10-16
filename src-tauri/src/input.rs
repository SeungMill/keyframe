use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

pub enum Kind {
    KeyDown(String),
    KeyUp(String),
    ButtonDown(String),
    ButtonUp(String),
    Move { x: i32, y: i32 },
    Wheel { dx: i32, dy: i32 },
}

pub struct InputEvent {
    pub time: SystemTime,
    pub injected: bool,
    pub kind: Kind,
}

pub enum Raw {
    Key { vk: u32, flags: u32, down: bool, time: SystemTime },
    Mouse { msg: u32, x: i32, y: i32, data: u32, flags: u32, time: SystemTime },
}

static TX: OnceLock<Mutex<Sender<Raw>>> = OnceLock::new();

#[repr(C)]
struct MsLl {
    x: i32,
    y: i32,
    data: u32,
    flags: u32,
    time: u32,
    extra: usize,
}

#[repr(C)]
struct Msg {
    hwnd: isize,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt_x: i32,
    pt_y: i32,
}

type HookProc = unsafe extern "system" fn(i32, usize, isize) -> isize;

#[link(name = "user32")]
extern "system" {
    fn SetWindowsHookExW(id: i32, callback: Option<HookProc>, module: isize, thread: u32) -> isize;
    fn GetAsyncKeyState(vk: i32) -> i16;
    fn CallNextHookEx(hook: isize, code: i32, w: usize, l: isize) -> isize;
    fn GetMessageW(msg: *mut Msg, hwnd: isize, min: u32, max: u32) -> i32;
    fn TranslateMessage(msg: *const Msg) -> i32;
    fn DispatchMessageW(msg: *const Msg) -> isize;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> isize;
}

fn send(raw: Raw) {
    if let Some(tx) = TX.get() {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(raw);
        }
    }
}

unsafe extern "system" fn ms_proc(code: i32, w: usize, l: isize) -> isize {
    if code == 0 {
        let m = &*(l as *const MsLl);
        send(Raw::Mouse { msg: w as u32, x: m.x, y: m.y, data: m.data, flags: m.flags, time: SystemTime::now() });
    }
    CallNextHookEx(0, code, w, l)
}

static KEYS_ON: AtomicBool = AtomicBool::new(false);
static PRIO_ON: AtomicBool = AtomicBool::new(false);
static PRIO_MASK: Mutex<[bool; 256]> = Mutex::new([false; 256]);

pub fn set_keys_enabled(on: bool) {
    KEYS_ON.store(on, Ordering::SeqCst);
}

pub fn set_priority_vks(vks: &[u16]) {
    if let Ok(mut mask) = PRIO_MASK.lock() {
        *mask = [false; 256];
        for &v in vks {
            if (v as usize) < 256 {
                mask[v as usize] = true;
            }
        }
    }
    PRIO_ON.store(!vks.is_empty(), Ordering::SeqCst);
}

fn key_is_down(vk: usize) -> bool {
    unsafe { (GetAsyncKeyState(vk as i32) as u16) & 0x8000 != 0 }
}

fn pollable(vk: usize) -> bool {
    !matches!(vk, 0x10..=0x12 | 0x15..=0x1A | 0x1C..=0x1F | 0xE5 | 0xE7 | 0xF6..=0xFE)
}

fn poll_keys() {
    let mut down = [false; 256];
    let mut held_at_start = [false; 256];
    let mut mode = 0u8;
    loop {
        let want = if KEYS_ON.load(Ordering::SeqCst) {
            2
        } else if PRIO_ON.load(Ordering::SeqCst) {
            1
        } else {
            0
        };
        if want == 0 {
            mode = 0;
            std::thread::sleep(Duration::from_millis(25));
            continue;
        }
        let mask = PRIO_MASK.lock().map(|m| *m).unwrap_or([false; 256]);
        let wanted = |vk: usize| pollable(vk) && (want == 2 || mask[vk]);
        if want != mode {
            mode = want;
            for vk in 8..=0xFE {
                let d = wanted(vk) && key_is_down(vk);
                down[vk] = d;
                held_at_start[vk] = d;
            }
        }
        for vk in 8..=0xFE {
            if !wanted(vk) {
                continue;
            }
            let d = key_is_down(vk);
            if d == down[vk] {
                continue;
            }
            down[vk] = d;
            if !d && held_at_start[vk] {
                held_at_start[vk] = false;
                continue;
            }
            send(Raw::Key { vk: vk as u32, flags: 0, down: d, time: SystemTime::now() });
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub fn start(tx: Sender<Raw>) {
    let _ = TX.set(Mutex::new(tx));
    std::thread::Builder::new()
        .name("mouse-hook".into())
        .spawn(|| unsafe {
            let module = GetModuleHandleW(ptr::null());
            if SetWindowsHookExW(14, Some(ms_proc), module, 0) == 0 {
                eprintln!("could not install the mouse hook");
            }
            let mut msg: Msg = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .expect("spawn mouse hook");
    std::thread::Builder::new().name("key-poll".into()).spawn(poll_keys).expect("spawn key poller");
}

fn wheel(hi: u16) -> i32 {
    let d = hi as i16 as i32;
    let n = d / 120;
    if n == 0 { d.signum() } else { n }
}

pub fn convert(raw: Raw) -> Option<InputEvent> {
    match raw {
        Raw::Key { vk, flags, down, time } => {
            let name = crate::keys::name_from_vk(vk, flags & 0x01 != 0);
            let kind = if down { Kind::KeyDown(name) } else { Kind::KeyUp(name) };
            Some(InputEvent { time, injected: flags & 0x10 != 0, kind })
        }
        Raw::Mouse { msg, x, y, data, flags, time } => {
            let hi = (data >> 16) as u16;
            let side = |hi: u16| match hi {
                1 => "X1".to_string(),
                2 => "X2".to_string(),
                n => format!("Button{n}"),
            };
            let kind = match msg {
                0x200 => Kind::Move { x, y },
                0x201 => Kind::ButtonDown("Left".into()),
                0x202 => Kind::ButtonUp("Left".into()),
                0x204 => Kind::ButtonDown("Right".into()),
                0x205 => Kind::ButtonUp("Right".into()),
                0x207 => Kind::ButtonDown("Middle".into()),
                0x208 => Kind::ButtonUp("Middle".into()),
                0x20B => Kind::ButtonDown(side(hi)),
                0x20C => Kind::ButtonUp(side(hi)),
                0x20A => Kind::Wheel { dx: 0, dy: wheel(hi) },
                0x20E => Kind::Wheel { dx: wheel(hi), dy: 0 },
                _ => return None,
            };
            Some(InputEvent { time, injected: flags & 0x03 != 0, kind })
        }
    }
}
