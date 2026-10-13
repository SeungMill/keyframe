import type { Trigger } from "./types";

const KEY_LABELS: Record<string, string> = {
  ControlLeft: "Ctrl L", ControlRight: "Ctrl R", ShiftLeft: "Shift L", ShiftRight: "Shift R",
  Alt: "Alt", AltGr: "Alt Gr", MetaLeft: "Win L", MetaRight: "Win R", Return: "Enter",
  Escape: "Esc", Backspace: "Backspace", Space: "Space", Tab: "Tab", CapsLock: "Caps Lock",
  UpArrow: "Up", DownArrow: "Down", LeftArrow: "Left", RightArrow: "Right",
  BackQuote: "`", Minus: "-", Equal: "=", LeftBracket: "[", RightBracket: "]", SemiColon: ";",
  Quote: "'", BackSlash: "\\", Comma: ",", Dot: ".", Slash: "/", KpPlus: "Num +", KpMinus: "Num -",
  KpMultiply: "Num *", KpDivide: "Num /", KpReturn: "Num Enter", KpDelete: "Num .",
};

export function prettyKey(name: string): string {
  if (KEY_LABELS[name]) return KEY_LABELS[name];
  if (/^Key[A-Z]$/.test(name)) return name.slice(3);
  if (/^Num\d$/.test(name)) return name.slice(3);
  if (/^Kp\d$/.test(name)) return "Num " + name.slice(2);
  return name;
}

export function prettyAccel(accel: string): string {
  return accel
    .split("+")
    .map((p) => p.replace(/^Key([A-Z])$/, "$1").replace(/^Digit(\d)$/, "$1").replace(/^Arrow/, ""))
    .join(" + ");
}

export function triggerLabel(t: Trigger | null): string {
  if (!t) return "No trigger";
  return t.kind === "keyboard" ? prettyAccel(t.accelerator) : `Mouse ${t.button}`;
}

export const uid = () => Math.random().toString(36).slice(2, 10) + Date.now().toString(36);

export const COMMON_KEYS = [
  ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").map((c) => "Key" + c),
  ...Array.from({ length: 10 }, (_, i) => "Num" + i),
  ...Array.from({ length: 12 }, (_, i) => "F" + (i + 1)),
  "Space", "Return", "Tab", "Escape", "Backspace", "Delete", "Insert", "Home", "End", "PageUp", "PageDown",
  "UpArrow", "DownArrow", "LeftArrow", "RightArrow", "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight",
  "Alt", "AltGr", "MetaLeft", "CapsLock", "BackQuote", "Minus", "Equal", "LeftBracket", "RightBracket",
  "SemiColon", "Quote", "BackSlash", "Comma", "Dot", "Slash",
  ...Array.from({ length: 10 }, (_, i) => "Kp" + i),
];

export function shortTrigger(t: Trigger | null): string {
  if (!t) return "";
  if (t.kind === "keyboard") return prettyAccel(t.accelerator).replace(/ \+ /g, "+");
  return ({ Middle: "Mid", Right: "RMB" } as Record<string, string>)[t.button] ?? t.button;
}

export function totalMs(events: { delayMs: number; type: string; ms?: number; durationMs?: number }[]): number {
  return events.reduce((s, e) => s + e.delayMs + (e.type === "delay" ? (e.ms ?? 0) : e.type === "keyHold" || e.type === "mouseHold" ? (e.durationMs ?? 0) : 0), 0);
}

const DOM_TO_KEY: Record<string, string> = {
  AltLeft: "Alt", AltRight: "AltGr", ControlLeft: "ControlLeft", ControlRight: "ControlRight",
  ShiftLeft: "ShiftLeft", ShiftRight: "ShiftRight", MetaLeft: "MetaLeft", MetaRight: "MetaRight",
  Enter: "Return", NumpadEnter: "KpReturn", Escape: "Escape", Backspace: "Backspace", Tab: "Tab", Space: "Space",
  CapsLock: "CapsLock", ArrowUp: "UpArrow", ArrowDown: "DownArrow", ArrowLeft: "LeftArrow", ArrowRight: "RightArrow",
  Backquote: "BackQuote", Minus: "Minus", Equal: "Equal", BracketLeft: "LeftBracket", BracketRight: "RightBracket",
  Semicolon: "SemiColon", Quote: "Quote", Backslash: "BackSlash", IntlBackslash: "IntlBackslash", Comma: "Comma",
  Period: "Dot", Slash: "Slash", Delete: "Delete", Insert: "Insert", Home: "Home", End: "End", PageUp: "PageUp",
  PageDown: "PageDown", PrintScreen: "PrintScreen", ScrollLock: "ScrollLock", Pause: "Pause", NumLock: "NumLock",
  NumpadAdd: "KpPlus", NumpadSubtract: "KpMinus", NumpadMultiply: "KpMultiply", NumpadDivide: "KpDivide",
  NumpadDecimal: "KpDelete",
};

export function domCodeToKey(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code;
  let m = /^Digit(\d)$/.exec(code);
  if (m) return "Num" + m[1];
  if (/^F([1-9]|1\d|2[0-4])$/.test(code)) return code;
  m = /^Numpad(\d)$/.exec(code);
  if (m) return "Kp" + m[1];
  return DOM_TO_KEY[code] ?? null;
}
