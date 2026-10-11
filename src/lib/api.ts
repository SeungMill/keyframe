import { invoke } from "@tauri-apps/api/core";
import type { DriverStatus, Macro, ProcInfo, Settings, Snapshot } from "./types";

export const api = {
  getState: () => invoke<Snapshot>("get_state"),
  createMacro: () => invoke<Macro>("create_macro", { name: null }),
  saveMacro: (data: Macro) => invoke<void>("save_macro", { data }),
  deleteMacro: (id: string) => invoke<void>("delete_macro", { id }),
  selectMacro: (id: string | null) => invoke<void>("select_macro", { id }),
  updateSettings: (settings: Settings) => invoke<void>("update_settings", { settings }),
  startRecording: (macroId: string | null) => invoke<void>("start_recording", { macroId }),
  stopRecording: () => invoke<void>("stop_recording"),
  playMacro: (id: string) => invoke<void>("play_macro", { id }),
  stopPlayback: (id: string | null) => invoke<void>("stop_playback", { id }),
  startCapture: (kind: "trigger" | "key") => invoke<void>("start_capture", { kind }),
  cancelCapture: () => invoke<void>("cancel_capture"),
  importMacros: () => invoke<number>("import_macros"),
  reorderMacros: (ids: string[]) => invoke<void>("reorder_macros", { ids }),
  windowMinimize: () => invoke<void>("window_minimize"),
  windowToggleMaximize: () => invoke<void>("window_toggle_maximize"),
  windowClose: () => invoke<void>("window_close"),
  driverStatus: () => invoke<DriverStatus>("driver_status"),
  driverInstall: (uninstall: boolean) => invoke<string>("driver_install", { uninstall }),
  listProcesses: () => invoke<ProcInfo[]>("list_processes"),
  exportMacros: (ids: string[] | null) => invoke<boolean>("export_macros", { ids }),
};
