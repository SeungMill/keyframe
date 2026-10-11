
#[derive(Clone, Copy, PartialEq)]
pub enum Mod {
    Ctrl,
    Shift,
    Alt,
    Super,
}

pub fn modifier(name: &str) -> Option<Mod> {
    match name {
        "ControlLeft" | "ControlRight" => Some(Mod::Ctrl),
        "ShiftLeft" | "ShiftRight" => Some(Mod::Shift),
        "Alt" | "AltGr" => Some(Mod::Alt),
        "MetaLeft" | "MetaRight" => Some(Mod::Super),
        _ => None,
    }
}

pub fn vk(name: &str) -> Option<u16> {
    if let Some(rest) = name.strip_prefix("Key") {
        let mut c = rest.chars();
        if let (Some(ch), None) = (c.next(), c.next()) {
            if ch.is_ascii_uppercase() {
                return Some(ch as u8 as u16);
            }
        }
    }
    if let Some(rest) = name.strip_prefix("Num") {
        if let Ok(n) = rest.parse::<u16>() {
            if n <= 9 {
                return Some(0x30 + n);
            }
        }
    }
    if let Some(rest) = name.strip_prefix("Kp") {
        if let Ok(n) = rest.parse::<u16>() {
            if n <= 9 {
                return Some(0x60 + n);
            }
        }
    }
    if let Some(rest) = name.strip_prefix('F') {
        if let Ok(n) = rest.parse::<u16>() {
            if (1..=24).contains(&n) {
                return Some(0x70 + n - 1);
            }
        }
    }
    if let Some(inner) = name.strip_prefix("Unknown(").and_then(|s| s.strip_suffix(')')) {
        return inner.parse().ok();
    }
    Some(match name {
        "Alt" => 0xA4,
        "AltGr" => 0xA5,
        "Backspace" => 0x08,
        "CapsLock" => 0x14,
        "ControlLeft" => 0xA2,
        "ControlRight" => 0xA3,
        "Delete" => 0x2E,
        "DownArrow" => 0x28,
        "End" => 0x23,
        "Escape" => 0x1B,
        "Home" => 0x24,
        "Insert" => 0x2D,
        "LeftArrow" => 0x25,
        "MetaLeft" => 0x5B,
        "MetaRight" => 0x5C,
        "PageDown" => 0x22,
        "PageUp" => 0x21,
        "Return" | "KpReturn" => 0x0D,
        "RightArrow" => 0x27,
        "ShiftLeft" => 0xA0,
        "ShiftRight" => 0xA1,
        "Space" => 0x20,
        "Tab" => 0x09,
        "UpArrow" => 0x26,
        "PrintScreen" => 0x2C,
        "ScrollLock" => 0x91,
        "Pause" => 0x13,
        "NumLock" => 0x90,
        "BackQuote" => 0xC0,
        "Minus" => 0xBD,
        "Equal" => 0xBB,
        "LeftBracket" => 0xDB,
        "RightBracket" => 0xDD,
        "SemiColon" => 0xBA,
        "Quote" => 0xDE,
        "BackSlash" => 0xDC,
        "IntlBackslash" => 0xE2,
        "Comma" => 0xBC,
        "Dot" => 0xBE,
        "Slash" => 0xBF,
        "KpMinus" => 0x6D,
        "KpPlus" => 0x6B,
        "KpMultiply" => 0x6A,
        "KpDivide" => 0x6F,
        "KpDelete" => 0x6E,
        _ => return None,
    })
}

pub fn accel_key(name: &str) -> Option<String> {
    if let Some(rest) = name.strip_prefix("Key") {
        if rest.len() == 1 {
            return Some(name.to_string());
        }
    }
    if let Some(rest) = name.strip_prefix("Num") {
        if let Ok(n) = rest.parse::<u8>() {
            if n <= 9 {
                return Some(format!("Digit{n}"));
            }
        }
    }
    if let Some(rest) = name.strip_prefix("Kp") {
        if let Ok(n) = rest.parse::<u8>() {
            if n <= 9 {
                return Some(format!("Numpad{n}"));
            }
        }
    }
    if let Some(rest) = name.strip_prefix('F') {
        if let Ok(n) = rest.parse::<u8>() {
            if (1..=24).contains(&n) {
                return Some(name.to_string());
            }
        }
    }
    let s = match name {
        "Space" => "Space",
        "Return" => "Enter",
        "Tab" => "Tab",
        "Backspace" => "Backspace",
        "Delete" => "Delete",
        "Insert" => "Insert",
        "Home" => "Home",
        "End" => "End",
        "PageUp" => "PageUp",
        "PageDown" => "PageDown",
        "UpArrow" => "ArrowUp",
        "DownArrow" => "ArrowDown",
        "LeftArrow" => "ArrowLeft",
        "RightArrow" => "ArrowRight",
        "BackQuote" => "Backquote",
        "Minus" => "Minus",
        "Equal" => "Equal",
        "LeftBracket" => "BracketLeft",
        "RightBracket" => "BracketRight",
        "SemiColon" => "Semicolon",
        "Quote" => "Quote",
        "BackSlash" => "Backslash",
        "Comma" => "Comma",
        "Dot" => "Period",
        "Slash" => "Slash",
        "KpPlus" => "NumpadAdd",
        "KpMinus" => "NumpadSubtract",
        "KpMultiply" => "NumpadMultiply",
        "KpDivide" => "NumpadDivide",
        "CapsLock" => "CapsLock",
        "PrintScreen" => "PrintScreen",
        "ScrollLock" => "ScrollLock",
        "Pause" => "Pause",
        "NumLock" => "NumLock",
        _ => return None,
    };
    Some(s.to_string())
}

pub fn hotkey_key_names(accel: &str) -> Vec<String> {
    let mut out = vec![];
    for tok in accel.split('+') {
        let t = tok.trim();
        match t.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "commandorcontrol" | "cmdorctrl" => {
                out.push("ControlLeft".to_string());
                out.push("ControlRight".to_string());
            }
            "shift" => {
                out.push("ShiftLeft".to_string());
                out.push("ShiftRight".to_string());
            }
            "alt" | "option" => {
                out.push("Alt".to_string());
                out.push("AltGr".to_string());
            }
            "super" | "meta" | "win" | "cmd" => {
                out.push("MetaLeft".to_string());
                out.push("MetaRight".to_string());
            }
            _ => {
                if t.len() == 1 && t.chars().all(|c| c.is_ascii_alphabetic()) {
                    out.push(format!("Key{}", t.to_ascii_uppercase()));
                } else if t.len() == 1 && t.chars().all(|c| c.is_ascii_digit()) {
                    out.push(format!("Num{t}"));
                } else if t.starts_with("Key") || t.starts_with("Digit") || t.starts_with('F') {
                    out.push(t.replace("Digit", "Num"));
                }
            }
        }
    }
    out
}

pub fn name_from_vk(vk: u32, extended: bool) -> String {
    let v = vk as u16;
    match v {
        0x41..=0x5A => return format!("Key{}", v as u8 as char),
        0x30..=0x39 => return format!("Num{}", v - 0x30),
        0x60..=0x69 => return format!("Kp{}", v - 0x60),
        0x70..=0x87 => return format!("F{}", v - 0x70 + 1),
        _ => {}
    }
    let name = match v {
        0xA4 => "Alt",
        0xA5 => "AltGr",
        0x08 => "Backspace",
        0x14 => "CapsLock",
        0xA2 => "ControlLeft",
        0xA3 => "ControlRight",
        0x2E => "Delete",
        0x28 => "DownArrow",
        0x23 => "End",
        0x1B => "Escape",
        0x24 => "Home",
        0x2D => "Insert",
        0x25 => "LeftArrow",
        0x5B => "MetaLeft",
        0x5C => "MetaRight",
        0x22 => "PageDown",
        0x21 => "PageUp",
        0x0D => if extended { "KpReturn" } else { "Return" },
        0x27 => "RightArrow",
        0xA0 => "ShiftLeft",
        0xA1 => "ShiftRight",
        0x20 => "Space",
        0x09 => "Tab",
        0x26 => "UpArrow",
        0x2C => "PrintScreen",
        0x91 => "ScrollLock",
        0x13 => "Pause",
        0x90 => "NumLock",
        0xC0 => "BackQuote",
        0xBD => "Minus",
        0xBB => "Equal",
        0xDB => "LeftBracket",
        0xDD => "RightBracket",
        0xBA => "SemiColon",
        0xDE => "Quote",
        0xDC => "BackSlash",
        0xE2 => "IntlBackslash",
        0xBC => "Comma",
        0xBE => "Dot",
        0xBF => "Slash",
        0x6D => "KpMinus",
        0x6B => "KpPlus",
        0x6A => "KpMultiply",
        0x6F => "KpDivide",
        0x6E => "KpDelete",
        _ => return format!("Unknown({v})"),
    };
    name.to_string()
}
