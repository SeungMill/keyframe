use crate::{inject::Injector, keys, model::*, state::*, util};
use enigo::{
    Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings as EnigoSettings,
};
use std::{
    collections::HashSet,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

const TAP_HOLD_MS: u64 = 8;

pub fn passes_for(m: &Macro) -> u32 {
    match m.play_mode {
        PlayMode::Once => 1,
        PlayMode::Count => m.repeat_count.max(1),
        PlayMode::Toggle | PlayMode::WhileHeld => 0,
    }
}

pub fn is_playing(st: &AppState, id: &str) -> bool {
    st.playback.lock().contains_key(id)
}

pub fn stop(app: &AppHandle, id: &str) {
    if let Some(flag) = app.state::<AppState>().playback.lock().get(id) {
        flag.store(true, Ordering::SeqCst);
    }
}

pub fn stop_all(app: &AppHandle) {
    for flag in app.state::<AppState>().playback.lock().values() {
        flag.store(true, Ordering::SeqCst);
    }
}

pub fn trigger(app: &AppHandle, id: &str, pressed: bool) {
    let st = app.state::<AppState>();
    if st.capture.lock().is_some() {
        return;
    }
    if matches!(st.recorder.lock().state, RecState::Countdown | RecState::Recording) {
        return;
    }
    let found = st.macros.lock().iter().find(|m| m.id == id).cloned();
    let Some(m) = found else { return };

    if pressed {
        if !crate::procs::scope_ok(m.process.as_deref(), crate::procs::foreground_exe().as_deref()) {
            return;
        }
        if !st.held.lock().insert(id.to_string()) || !m.enabled {
            return;
        }
        let res = match m.play_mode {
            PlayMode::Once | PlayMode::Count => start(app, m.clone(), passes_for(&m)),
            PlayMode::Toggle => {
                if is_playing(&st, id) {
                    stop(app, id);
                    Ok(())
                } else {
                    start(app, m.clone(), 0)
                }
            }
            PlayMode::WhileHeld => {
                if is_playing(&st, id) {
                    Ok(())
                } else {
                    start(app, m.clone(), 0)
                }
            }
        };
        let _ = res;
    } else {
        st.held.lock().remove(id);
        if m.play_mode == PlayMode::WhileHeld {
            stop(app, id);
        }
    }
}

pub fn start(app: &AppHandle, m: Macro, passes: u32) -> Result<(), String> {
    let st = app.state::<AppState>();
    if matches!(st.recorder.lock().state, RecState::Countdown | RecState::Recording) {
        return Err("Can't play while recording.".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));
    {
        let mut pb = st.playback.lock();
        if pb.contains_key(&m.id) {
            return Err("Macro is already playing.".into());
        }
        pb.insert(m.id.clone(), cancel.clone());
    }
    st.paused.lock().insert(m.id.clone(), paused.clone());
    let app2 = app.clone();
    let id = m.id.clone();
    let spawned = thread::Builder::new()
        .name(format!("playback-{}", m.name))
        .spawn(move || run(app2, m, passes, cancel, paused));
    if let Err(e) = spawned {
        st.playback.lock().remove(&id);
        st.paused.lock().remove(&id);
        return Err(e.to_string());
    }
    crate::priority::sync_polling(app);
    Ok(())
}

fn emit(app: &AppHandle, id: &str, state: PlayState, pass: u32, total: u32, idx: usize) {
    let _ = app.emit(
        "playback-state",
        PlaybackPayload { macro_id: id.to_string(), state, pass, total, event_index: idx },
    );
}

fn run(app: AppHandle, m: Macro, passes: u32, cancel: Arc<AtomicBool>, paused: Arc<AtomicBool>) {
    emit(&app, &m.id, PlayState::Playing, 0, passes, 0);
    if let Err(e) = execute(&app, &m, passes, &cancel, paused) {
        toast(&app, "error", format!("Playback of \"{}\" failed: {e}", m.name));
        cancel.store(true, Ordering::SeqCst);
    }
    {
        let st = app.state::<AppState>();
        let mut pb = st.playback.lock();
        if pb.get(&m.id).map(|c| Arc::ptr_eq(c, &cancel)).unwrap_or(false) {
            pb.remove(&m.id);
        }
    }
    crate::priority::finish(&app, &m.id);
    let end = if cancel.load(Ordering::SeqCst) { PlayState::Cancelled } else { PlayState::Done };
    emit(&app, &m.id, end, 0, passes, 0);
}

struct Runner<'a> {
    app: &'a AppHandle,
    macro_id: &'a str,
    enigo: Enigo,
    cancel: &'a AtomicBool,
    inj: Option<Injector>,
    warned: bool,
    paused: Arc<AtomicBool>,
    keys: HashSet<String>,
    btns: HashSet<String>,
    pass: u32,
    total: u32,
    last_emit: Instant,
}

fn execute(app: &AppHandle, m: &Macro, passes: u32, cancel: &AtomicBool, paused: Arc<AtomicBool>) -> Result<(), String> {
    let enigo = Enigo::new(&EnigoSettings::default()).map_err(|e| e.to_string())?;
    let backend = app.state::<AppState>().settings.lock().input_backend;
    let inj = Injector::new(backend, crate::driver::dll_path(app))?;
    let mut r = Runner {
        inj,
        warned: false,
        paused,
        app,
        macro_id: &m.id,
        enigo,
        cancel,
        keys: HashSet::new(),
        btns: HashSet::new(),
        pass: 0,
        total: passes,
        last_emit: Instant::now(),
    };
    let mut pass = 0u32;
    loop {
        pass += 1;
        r.pass = pass;
        emit(app, &m.id, PlayState::Playing, pass, passes, 0);
        if !r.run_pass(&m.events) {
            break;
        }
        if passes != 0 && pass >= passes {
            break;
        }
        if !util::wait_ms(1, cancel) {
            break;
        }
    }
    r.release_all();
    Ok(())
}

fn loop_end(stack: &mut Vec<(usize, u32)>, i: usize) -> usize {
    match stack.pop() {
        Some((start, rem)) => {
            if rem == 0 {
                stack.push((start, 0));
                start
            } else if rem > 1 {
                stack.push((start, rem - 1));
                start
            } else {
                i + 1
            }
        }
        None => i + 1,
    }
}

impl Runner<'_> {
    fn run_pass(&mut self, events: &[MacroEvent]) -> bool {
        let mut stack: Vec<(usize, u32)> = Vec::new();
        let mut i = 0usize;
        loop {
            if i >= events.len() {
                if stack.is_empty() {
                    return true;
                }
                i = loop_end(&mut stack, i);
                continue;
            }
            let ev = &events[i];
            if !self.wait(ev.delay_ms) {
                return false;
            }
            self.progress(i);
            match &ev.kind {
                EventKind::LoopStart { count } => {
                    stack.push((i + 1, *count));
                    i += 1;
                }
                EventKind::LoopEnd => {
                    let next = loop_end(&mut stack, i);
                    if next == i && !self.wait(1) {
                        return false;
                    }
                    i = next;
                }
                kind => {
                    if !self.exec(kind) {
                        return false;
                    }
                    i += 1;
                }
            }
        }
    }

    fn progress(&mut self, idx: usize) {
        if self.last_emit.elapsed() < Duration::from_millis(50) {
            return;
        }
        self.last_emit = Instant::now();
        emit(self.app, self.macro_id, PlayState::Playing, self.pass, self.total, idx);
    }

    fn exec(&mut self, kind: &EventKind) -> bool {
        match kind {
            EventKind::KeyPress { key } => self.key(key, true),
            EventKind::KeyRelease { key } => self.key(key, false),
            EventKind::KeyTap { key } => {
                self.key(key, true);
                let ok = self.wait(TAP_HOLD_MS);
                self.key(key, false);
                return ok;
            }
            EventKind::KeyHold { key, duration_ms } => {
                self.key(key, true);
                let ok = self.wait(*duration_ms);
                self.key(key, false);
                return ok;
            }
            EventKind::MouseDown { button } => self.button(button, true),
            EventKind::MouseUp { button } => self.button(button, false),
            EventKind::MouseHold { button, duration_ms } => {
                self.button(button, true);
                let ok = self.wait(*duration_ms);
                self.button(button, false);
                return ok;
            }
            EventKind::MouseClick { button } => {
                self.button(button, true);
                let ok = self.wait(TAP_HOLD_MS);
                self.button(button, false);
                return ok;
            }
            EventKind::MouseMove { x, y, relative } => {
                if let Some(r) = self.inj.as_mut().map(|i| i.move_mouse(*x, *y, *relative)) {
                    self.report(r);
                } else {
                    let coord = if *relative { Coordinate::Rel } else { Coordinate::Abs };
                    if let Err(e) = self.enigo.move_mouse(*x, *y, coord) {
                        eprintln!("mouse move failed: {e}");
                    }
                }
            }
            EventKind::Scroll { dx, dy } => {
                if let Some(r) = self.inj.as_mut().map(|i| i.scroll(*dx, *dy)) {
                    self.report(r);
                } else {
                    if *dy != 0 {
                        let _ = self.enigo.scroll(-*dy, Axis::Vertical);
                    }
                    if *dx != 0 {
                        let _ = self.enigo.scroll(*dx, Axis::Horizontal);
                    }
                }
            }
            EventKind::Delay { ms } => return self.wait(*ms),
            EventKind::TypeText { text, char_delay_ms } => {
                if *char_delay_ms == 0 {
                    if let Err(e) = self.enigo.text(text) {
                        eprintln!("type text failed: {e}");
                    }
                } else {
                    for ch in text.chars() {
                        let _ = self.enigo.text(&ch.to_string());
                        if !self.wait(*char_delay_ms) {
                            return false;
                        }
                    }
                }
            }
            EventKind::RunProgram { path, args } => {
                if !path.trim().is_empty() {
                    if let Err(e) = Command::new(path).args(split_args(args)).spawn() {
                        toast(self.app, "error", format!("Could not run '{path}': {e}"));
                    }
                }
            }
            EventKind::LoopStart { .. } | EventKind::LoopEnd => {}
        }
        true
    }

    fn key(&mut self, name: &str, down: bool) {
        let Some(vk) = keys::vk(name) else {
            eprintln!("unknown key '{name}', skipped");
            return;
        };
        let app = self.app;
        app.state::<AppState>().note_key_out(name, down);
        if down {
            self.keys.insert(name.to_string());
        } else {
            self.keys.remove(name);
        }
        if let Some(r) = self.inj.as_mut().map(|i| i.key(name, vk, down)) {
            self.report(r);
            return;
        }
        let dir = if down { Direction::Press } else { Direction::Release };
        if let Err(e) = self.enigo.key(Key::Other(vk as u32), dir) {
            eprintln!("key '{name}' failed: {e}");
        }
    }

    fn button(&mut self, name: &str, down: bool) {
        let btn = match name {
            "Left" => Button::Left,
            "Right" => Button::Right,
            "Middle" => Button::Middle,
            "X1" => Button::Back,
            "X2" => Button::Forward,
            _ => return,
        };
        let app = self.app;
        app.state::<AppState>().note_inject(name, down);
        if down {
            self.btns.insert(name.to_string());
        } else {
            self.btns.remove(name);
        }
        if let Some(r) = self.inj.as_mut().map(|i| i.button(name, down)) {
            self.report(r);
            return;
        }
        let dir = if down { Direction::Press } else { Direction::Release };
        if let Err(e) = self.enigo.button(btn, dir) {
            eprintln!("button '{name}' failed: {e}");
        }
    }

    fn report(&mut self, r: Result<(), String>) {
        if let Err(e) = r {
            if !self.warned {
                self.warned = true;
                toast(self.app, "error", e);
            }
            self.cancel.store(true, Ordering::SeqCst);
        }
    }

    fn gate(&mut self) -> Option<Duration> {
        if self.cancel.load(Ordering::Relaxed) {
            return None;
        }
        if !self.paused.load(Ordering::Relaxed) {
            return Some(Duration::ZERO);
        }
        let started = Instant::now();
        let held_keys: Vec<String> = self.keys.iter().cloned().collect();
        let held_btns: Vec<String> = self.btns.iter().cloned().collect();
        for k in &held_keys {
            self.key(k, false);
        }
        for b in &held_btns {
            self.button(b, false);
        }
        while self.paused.load(Ordering::Relaxed) {
            if self.cancel.load(Ordering::Relaxed) {
                return None;
            }
            thread::sleep(Duration::from_millis(5));
        }
        for k in &held_keys {
            self.key(k, true);
        }
        for b in &held_btns {
            self.button(b, true);
        }
        Some(started.elapsed())
    }

    fn wait(&mut self, ms: u64) -> bool {
        let mut deadline = Instant::now() + Duration::from_millis(ms);
        loop {
            match self.gate() {
                None => return false,
                Some(paused_for) => deadline += paused_for,
            }
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            let rem = deadline - now;
            if rem > Duration::from_millis(3) {
                thread::sleep((rem - Duration::from_millis(2)).min(Duration::from_millis(10)));
            } else {
                std::hint::spin_loop();
            }
        }
    }

    fn release_all(&mut self) {
        let keys: Vec<String> = self.keys.drain().collect();
        for k in keys {
            self.key(&k, false);
        }
        let btns: Vec<String> = self.btns.drain().collect();
        for b in btns {
            self.button(&b, false);
        }
    }
}

fn split_args(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut quoted = false;
    let mut has = false;
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                has = true;
            }
            c if c.is_whitespace() && !quoted => {
                if has || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            c => cur.push(c),
        }
    }
    if has || !cur.is_empty() {
        out.push(cur);
    }
    out
}
