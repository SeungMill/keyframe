use serde::{Deserialize, Serialize};

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn yes() -> bool {
    true
}
fn one() -> u32 {
    1
}
fn resume_default() -> u64 {
    400
}
fn default_hotkey() -> String {
    "Ctrl+Shift+R".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum EventKind {
    KeyPress { key: String },
    KeyRelease { key: String },
    KeyTap { key: String },
    KeyHold { key: String, duration_ms: u64 },
    MouseDown { button: String },
    MouseUp { button: String },
    MouseClick { button: String },
    MouseHold { button: String, duration_ms: u64 },
    MouseMove { x: i32, y: i32, relative: bool },
    // dy > 0 scrolls up, dx > 0 scrolls right
    Scroll { dx: i32, dy: i32 },
    Delay { ms: u64 },
    TypeText { text: String, #[serde(default)] char_delay_ms: u64 },
    RunProgram { path: String, #[serde(default)] args: String },
    LoopStart { count: u32 },
    LoopEnd,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MacroEvent {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default)]
    pub delay_ms: u64,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Trigger {
    Keyboard { accelerator: String },
    Mouse { button: String },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PlayMode {
    #[default]
    Once,
    Count,
    Toggle,
    WhileHeld,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Macro {
    #[serde(default = "new_id")]
    pub id: String,
    pub name: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub trigger: Option<Trigger>,
    #[serde(default)]
    pub play_mode: PlayMode,
    #[serde(default = "one")]
    pub repeat_count: u32,
    #[serde(default)]
    pub events: Vec<MacroEvent>,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub process: Option<String>,
    #[serde(default)]
    pub priority_keys: Vec<String>,
    #[serde(default = "resume_default")]
    pub priority_resume_ms: u64,
}

impl Macro {
    pub fn new(name: String) -> Self {
        Macro {
            id: new_id(),
            name,
            enabled: true,
            trigger: None,
            play_mode: PlayMode::Once,
            repeat_count: 1,
            events: vec![],
            folder: None,
            process: None,
            priority_keys: vec![],
            priority_resume_ms: 400,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum DelayMode {
    #[default]
    Record,
    Fixed,
    None,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum MouseMode {
    #[default]
    None,
    Absolute,
    Relative,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum InputBackend {
    #[default]
    #[serde(alias = "scanCode", alias = "driver", alias = "virtualMouse")]
    Standard,
    Interception,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub input_backend: InputBackend,
    pub theme: String,
    pub folders: Vec<String>,
    pub start_minimized: bool,
    pub launch_at_login: bool,
    pub close_to_tray: bool,
    pub always_on_top: bool,
    pub opacity: u8,
    pub compact: bool,
    pub compact_card: Option<String>,
    pub compact_off_open: Vec<String>,
    pub compact_x: Option<i32>,
    pub compact_y: Option<i32>,
    pub delay_mode: DelayMode,
    pub fixed_delay_ms: u64,
    pub mouse_mode: MouseMode,
    pub record_hotkey: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            input_backend: InputBackend::default(),
            theme: "graphite".to_string(),
            folders: vec![],
            start_minimized: false,
            launch_at_login: false,
            close_to_tray: false,
            always_on_top: false,
            opacity: 100,
            compact: false,
            compact_card: None,
            compact_off_open: vec![],
            compact_x: None,
            compact_y: None,
            delay_mode: DelayMode::Record,
            fixed_delay_ms: 50,
            mouse_mode: MouseMode::None,
            record_hotkey: default_hotkey(),
        }
    }
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum RecState {
    #[default]
    Idle,
    Countdown,
    Recording,
    Stopped,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PlayState {
    Playing,
    Done,
    Cancelled,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecordingPayload {
    pub state: RecState,
    pub countdown: u8,
    pub macro_id: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPayload {
    pub macro_id: String,
    pub state: PlayState,
    pub pass: u32,
    pub total: u32,
    pub event_index: usize,
}

#[derive(Serialize, Clone)]
pub struct ToastPayload {
    pub level: String,
    pub message: String,
}

#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CapturePayload {
    Trigger { trigger: Trigger },
    Key { key: String },
    Cancelled,
}
