export type EventKind =
  | { type: "keyPress"; key: string }
  | { type: "keyRelease"; key: string }
  | { type: "keyTap"; key: string }
  | { type: "keyHold"; key: string; durationMs: number }
  | { type: "mouseDown"; button: string }
  | { type: "mouseUp"; button: string }
  | { type: "mouseClick"; button: string }
  | { type: "mouseHold"; button: string; durationMs: number }
  | { type: "mouseMove"; x: number; y: number; relative: boolean }
  | { type: "scroll"; dx: number; dy: number }
  | { type: "delay"; ms: number }
  | { type: "typeText"; text: string; charDelayMs: number }
  | { type: "runProgram"; path: string; args: string }
  | { type: "loopStart"; count: number }
  | { type: "loopEnd" };

export type MacroEvent = EventKind & { id: string; delayMs: number };

export type Trigger =
  | { kind: "keyboard"; accelerator: string }
  | { kind: "mouse"; button: string };

export type PlayMode = "once" | "count" | "toggle" | "whileHeld";

export interface Macro {
  id: string;
  name: string;
  enabled: boolean;
  trigger: Trigger | null;
  playMode: PlayMode;
  repeatCount: number;
  events: MacroEvent[];
  folder: string | null;
  process: string | null;
  priorityKeys: string[];
  priorityResumeMs: number;
}

export type DelayMode = "record" | "fixed" | "none";
export type MouseMode = "none" | "absolute" | "relative";

export type InputBackend = "standard" | "interception";

export interface Settings {
  inputBackend: InputBackend;
  theme: string;
  folders: string[];
  startMinimized: boolean;
  launchAtLogin: boolean;
  closeToTray: boolean;
  runAsAdmin: boolean;
  toggleHotkey: string;
  alwaysOnTop: boolean;
  opacity: number;
  compact: boolean;
  compactCard: string | null;
  compactOffOpen: string[];
  compactX: number | null;
  compactY: number | null;
  delayMode: DelayMode;
  fixedDelayMs: number;
  mouseMode: MouseMode;
  recordHotkey: string;
}

export type RecState = "idle" | "countdown" | "recording" | "stopped";
export interface RecordingPayload {
  state: RecState;
  countdown: number;
  macroId: string | null;
}

export interface PlaybackPayload {
  macroId: string;
  state: "playing" | "done" | "cancelled";
  pass: number;
  total: number;
  eventIndex: number;
}

export interface Snapshot {
  macros: Macro[];
  settings: Settings;
  recording: RecordingPayload;
  playing: string[];
  selected: string | null;
  elevated: boolean;
}

export type CaptureResult =
  | { kind: "trigger"; trigger: Trigger }
  | { kind: "key"; key: string }
  | { kind: "cancelled" };

export interface ToastPayload {
  level: "error" | "warn" | "info";
  message: string;
}

export interface DriverStatus {
  dllFound: boolean;
  installerFound: boolean;
  active: boolean;
  message: string;
}

export interface ProcInfo {
  exe: string;
  title: string;
}
