use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[cfg(windows)]
#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(period: u32) -> u32;
}
#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn Beep(freq: u32, duration: u32) -> i32;
}

pub fn timer_begin(ms: u32) {
    #[cfg(windows)]
    unsafe {
        timeBeginPeriod(ms);
    }
    #[cfg(not(windows))]
    let _ = ms;
}

pub fn beep(freq: u32, ms: u32) {
    std::thread::spawn(move || {
        #[cfg(windows)]
        unsafe {
            Beep(freq, ms);
        }
        #[cfg(not(windows))]
        let _ = (freq, ms);
    });
}

pub fn wait_ms(ms: u64, cancel: &AtomicBool) -> bool {
    if ms == 0 {
        return !cancel.load(Ordering::Relaxed);
    }
    let deadline = Instant::now() + Duration::from_millis(ms);
    loop {
        if cancel.load(Ordering::Relaxed) {
            return false;
        }
        let now = Instant::now();
        if now >= deadline {
            return true;
        }
        let rem = deadline - now;
        if rem > Duration::from_millis(3) {
            std::thread::sleep((rem - Duration::from_millis(2)).min(Duration::from_millis(20)));
        } else {
            std::hint::spin_loop();
        }
    }
}
