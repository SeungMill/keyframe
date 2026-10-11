use crate::model::InputBackend;
use std::ffi::c_void;
use std::path::{Path, PathBuf};

#[link(name = "user32")]
extern "system" {
    fn MapVirtualKeyW(code: u32, map_type: u32) -> u32;
    fn GetSystemMetrics(index: i32) -> i32;
}

fn scan(vk: u16, name: &str) -> (u16, bool) {
    if name == "KpReturn" {
        return (0x1C, true);
    }
    let r = unsafe { MapVirtualKeyW(vk as u32, 4) }; // MAPVK_VK_TO_VSC_EX
    let ext = (r >> 8) == 0xE0 || matches!(vk, 0x21..=0x28 | 0x2D | 0x2E | 0x5B | 0x5C | 0x6F | 0xA3 | 0xA5);
    ((r & 0xFF) as u16, ext)
}

fn virtual_screen() -> (i32, i32, i32, i32) {
    unsafe { (GetSystemMetrics(76), GetSystemMetrics(77), GetSystemMetrics(78).max(2), GetSystemMetrics(79).max(2)) }
}
fn norm(v: i32, origin: i32, size: i32) -> i32 {
    (((v - origin) as i64 * 65535) / (size as i64 - 1)).clamp(0, 65535) as i32
}

fn icp_button(name: &str) -> Option<(u16, u16)> {
    Some(match name {
        "Left" => (0x001, 0x002),
        "Right" => (0x004, 0x008),
        "Middle" => (0x010, 0x020),
        "X1" => (0x040, 0x080),
        "X2" => (0x100, 0x200),
        _ => return None,
    })
}

pub struct Injector {
    drv: InterceptionDriver,
}

impl Injector {
    pub fn new(backend: InputBackend, dll: Option<PathBuf>) -> Result<Option<Injector>, String> {
        match backend {
            InputBackend::Standard => Ok(None),
            InputBackend::Interception => {
                let path = dll.ok_or("interception.dll is missing from the app folder (reinstall Keyframe).")?;
                Ok(Some(Injector { drv: InterceptionDriver::load(&path)? }))
            }
        }
    }

    pub fn key(&mut self, name: &str, vk: u16, down: bool) -> Result<(), String> {
        let (code, ext) = scan(vk, name);
        if code == 0 {
            return Err(format!("No scan code for key '{name}'."));
        }
        let state = (!down) as u16 | if ext { 0x02 } else { 0 };
        self.drv.send(false, &key_stroke(code, state))
    }

    pub fn button(&mut self, name: &str, down: bool) -> Result<(), String> {
        let Some((d, u)) = icp_button(name) else { return Ok(()) };
        self.drv.send(true, &mouse_stroke(if down { d } else { u }, 0, 0, 0, 0))
    }

    pub fn move_mouse(&mut self, x: i32, y: i32, relative: bool) -> Result<(), String> {
        if relative {
            self.drv.send(true, &mouse_stroke(0, 0, 0, x, y))
        } else {
            let (vx, vy, vw, vh) = virtual_screen();
            self.drv.send(true, &mouse_stroke(0, 0x1 | 0x2, 0, norm(x, vx, vw), norm(y, vy, vh)))
        }
    }

    pub fn scroll(&mut self, dx: i32, dy: i32) -> Result<(), String> {
        if dy != 0 {
            self.drv.send(true, &mouse_stroke(0x400, 0, (dy * 120).clamp(-32768, 32767) as i16, 0, 0))?;
        }
        if dx != 0 {
            self.drv.send(true, &mouse_stroke(0x800, 0, (dx * 120).clamp(-32768, 32767) as i16, 0, 0))?;
        }
        Ok(())
    }
}

// key stroke: u16 code, u16 state, u32 info. mouse stroke: u16 state, u16 flags, i16 rolling, i32 x, i32 y, u32 info.
// both live in the same 20-byte buffer.
fn key_stroke(code: u16, state: u16) -> [u8; 20] {
    let mut b = [0u8; 20];
    b[0..2].copy_from_slice(&code.to_le_bytes());
    b[2..4].copy_from_slice(&state.to_le_bytes());
    b
}
fn mouse_stroke(state: u16, flags: u16, rolling: i16, x: i32, y: i32) -> [u8; 20] {
    let mut b = [0u8; 20];
    b[0..2].copy_from_slice(&state.to_le_bytes());
    b[2..4].copy_from_slice(&flags.to_le_bytes());
    b[4..6].copy_from_slice(&rolling.to_le_bytes());
    b[8..12].copy_from_slice(&x.to_le_bytes());
    b[12..16].copy_from_slice(&y.to_le_bytes());
    b
}

pub struct InterceptionDriver {
    _lib: libloading::Library,
    ctx: *mut c_void,
    send_fn: unsafe extern "C" fn(*mut c_void, i32, *const c_void, u32) -> i32,
    destroy_fn: unsafe extern "C" fn(*mut c_void),
    kbd: Option<i32>,
    mouse: Option<i32>,
}

pub fn probe(dll: &Path) -> Result<(), String> {
    InterceptionDriver::load(dll).map(|_| ())
}

impl InterceptionDriver {
    fn load(dll: &Path) -> Result<Self, String> {
        unsafe {
            let lib = libloading::Library::new(dll).map_err(|e| format!("Could not load interception.dll: {e}"))?;
            let create = *lib
                .get::<unsafe extern "C" fn() -> *mut c_void>(b"interception_create_context\0")
                .map_err(|e| e.to_string())?;
            let destroy_fn = *lib
                .get::<unsafe extern "C" fn(*mut c_void)>(b"interception_destroy_context\0")
                .map_err(|e| e.to_string())?;
            let send_fn = *lib
                .get::<unsafe extern "C" fn(*mut c_void, i32, *const c_void, u32) -> i32>(b"interception_send\0")
                .map_err(|e| e.to_string())?;
            let ctx = create();
            if ctx.is_null() {
                return Err("The Interception driver is not active. Use Settings > Playback > Install driver, then restart Windows.".into());
            }
            Ok(InterceptionDriver { _lib: lib, ctx, send_fn, destroy_fn, kbd: None, mouse: None })
        }
    }

    fn send(&mut self, mouse: bool, stroke: &[u8; 20]) -> Result<(), String> {
        let (send, ctx) = (self.send_fn, self.ctx);
        let (cached, range) = if mouse { (&mut self.mouse, 11..=20) } else { (&mut self.kbd, 1..=10) };
        let p = stroke.as_ptr() as *const c_void;
        if let Some(d) = *cached {
            if unsafe { send(ctx, d, p, 1) } > 0 {
                return Ok(());
            }
        }
        for d in range {
            if unsafe { send(ctx, d, p, 1) } > 0 {
                *cached = Some(d);
                return Ok(());
            }
        }
        Err(format!(
            "Interception accepted no {} device. Is the driver installed and the PC rebooted afterwards?",
            if mouse { "mouse" } else { "keyboard" }
        ))
    }
}

impl Drop for InterceptionDriver {
    fn drop(&mut self) {
        unsafe { (self.destroy_fn)(self.ctx) }
    }
}
