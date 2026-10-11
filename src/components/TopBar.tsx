import { MoreHorizontal, Settings as Gear, Square } from "lucide-react";
import type { ReactNode } from "react";
import type { DelayMode, InputBackend, MouseMode, RecordingPayload, Settings } from "../lib/types";
import { prettyAccel } from "../lib/fmt";
import { THEMES } from "../lib/themes";
import Popover from "./Popover";
import WindowControls from "./WindowControls";
import DriverRow from "./DriverRow";

interface Props {
  rec: RecordingPayload;
  settings: Settings;
  anyPlaying: boolean;
  canExport: boolean;
  onSettings: (s: Settings) => void;
  onRecord: () => void;
  onStop: () => void;
  onPickHotkey: () => void;
  onImport: () => void;
  onExportSelected: () => void;
  onExportAll: () => void;
}

const field = "rounded-md border border-line bg-base px-2 py-1 text-sm text-fg";

function IconBtn({ children, active, ...rest }: React.ButtonHTMLAttributes<HTMLButtonElement> & { active?: boolean }) {
  return (
    <button
      {...rest}
      className={`rounded-md p-1.5 hover:bg-raise hover:text-fg disabled:opacity-30 disabled:hover:bg-transparent ${
        active ? "bg-raise text-fg" : "text-mute"
      }`}
    >
      {children}
    </button>
  );
}

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      onClick={(e) => {
        e.preventDefault();
        onChange(!checked);
      }}
      className={`relative h-5 w-9 shrink-0 rounded-full transition ${checked ? "bg-accent" : "bg-line"}`}
    >
      <span className={`absolute top-0.5 h-4 w-4 rounded-full transition-all ${checked ? "left-4.5 bg-accent-fg" : "left-0.5 bg-fg"}`} />
    </button>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex items-center justify-between gap-3 py-1.5">
      <span className="text-sm text-mute">{label}</span>
      {children}
    </label>
  );
}

export default function TopBar(p: Props) {
  const s = p.settings;
  const busy = p.rec.state === "countdown" || p.rec.state === "recording";
  const label = p.rec.state === "countdown" ? `Starting in ${p.rec.countdown}` : p.rec.state === "recording" ? "Recording" : "Record";

  return (
    <header className="flex items-center gap-1.5 border-b border-line bg-panel py-1.5 pl-4 pr-2">
      <h1 data-tauri-drag-region className="text-sm font-medium tracking-wide">Keyframe</h1>
      <div data-tauri-drag-region className="h-8 flex-1" />

      <button
        onClick={p.onRecord}
        disabled={busy}
        className={`mr-1 flex items-center gap-2 rounded-full bg-rec px-4 py-1.5 text-sm font-medium text-rec-fg hover:brightness-110 disabled:cursor-default ${
          p.rec.state === "recording" ? "rec-pulse" : ""
        }`}
      >
        <span className="h-2 w-2 rounded-full bg-rec-fg" />
        {label}
      </button>
      <IconBtn title="Stop" onClick={p.onStop} disabled={!busy && !p.anyPlaying}>
        <Square size={17} />
      </IconBtn>

      <Popover
        width={300}
        trigger={(open, toggle) => (
          <IconBtn title="Settings" onClick={toggle} active={open}>
            <Gear size={17} />
          </IconBtn>
        )}
      >
        {() => (
          <div>
            <div className="mb-1 text-xs text-mute">Theme</div>
            <div className="mb-3 flex gap-2">
              {THEMES.map((t) => (
                <button
                  key={t.id}
                  onClick={() => p.onSettings({ ...s, theme: t.id })}
                  className={`flex flex-1 items-center gap-2 rounded-md border px-2.5 py-1.5 text-sm ${
                    s.theme === t.id ? "border-accent" : "border-line hover:border-mute"
                  }`}
                >
                  <span className="flex -space-x-1">
                    {["bg", "accent", "rec"].map((k) => (
                      <span key={k} className="h-3.5 w-3.5 rounded-full border border-line" style={{ background: t.vars[k] }} />
                    ))}
                  </span>
                  {t.name}
                </button>
              ))}
            </div>

            <div className="mb-0.5 text-xs text-mute">Recording</div>
            <Row label="Delays">
              <select className={field} value={s.delayMode} disabled={busy} onChange={(e) => p.onSettings({ ...s, delayMode: e.target.value as DelayMode })}>
                <option value="record">Real timing</option>
                <option value="fixed">Fixed</option>
                <option value="none">None</option>
              </select>
            </Row>
            {s.delayMode === "fixed" && (
              <Row label="Gap (ms)">
                <input
                  type="number"
                  min={1}
                  disabled={busy}
                  value={s.fixedDelayMs}
                  onChange={(e) => p.onSettings({ ...s, fixedDelayMs: Math.max(1, parseInt(e.target.value, 10) || 1) })}
                  className={`${field} w-20 text-right font-mono`}
                />
              </Row>
            )}
            <Row label="Mouse movement">
              <select className={field} value={s.mouseMode} disabled={busy} onChange={(e) => p.onSettings({ ...s, mouseMode: e.target.value as MouseMode })}>
                <option value="none">Ignore</option>
                <option value="absolute">Absolute</option>
                <option value="relative">Relative</option>
              </select>
            </Row>
            <Row label="Hotkey">
              <button
                onClick={(e) => {
                  e.preventDefault();
                  p.onPickHotkey();
                }}
                title="Click, then press the new shortcut"
                className="rounded border border-line bg-base px-2 py-0.5 font-mono text-xs hover:border-accent"
              >
                {prettyAccel(s.recordHotkey)}
              </button>
            </Row>

            <div className="mb-0.5 mt-2 text-xs text-mute">Playback</div>
            <Row label="Input">
              <select className={field} value={s.inputBackend} onChange={(e) => p.onSettings({ ...s, inputBackend: e.target.value as InputBackend })}>
                <option value="standard">Standard</option>
                <option value="interception">Driver (Interception)</option>
              </select>
            </Row>
            <p className="text-xs leading-relaxed text-mute">
              Driver sends keys and mouse through the Interception driver, so games see them as real hardware input.
            </p>
            {s.inputBackend === "interception" && <DriverRow />}

            <div className="mb-0.5 mt-3 text-xs text-mute">General</div>
            <Row label="Start with Windows">
              <Toggle checked={s.launchAtLogin} onChange={(v) => p.onSettings({ ...s, launchAtLogin: v })} />
            </Row>
            <Row label="Close button hides to tray">
              <Toggle checked={s.closeToTray} onChange={(v) => p.onSettings({ ...s, closeToTray: v })} />
            </Row>
            <Row label="Start minimized to tray">
              <Toggle checked={s.startMinimized} onChange={(v) => p.onSettings({ ...s, startMinimized: v })} />
            </Row>
          </div>
        )}
      </Popover>

      <Popover
        width={170}
        trigger={(open, toggle) => (
          <IconBtn title="More" onClick={toggle} active={open}>
            <MoreHorizontal size={17} />
          </IconBtn>
        )}
      >
        {(close) => (
          <div className="-m-1.5 text-sm">
            {[
              ["Import macros", p.onImport, false],
              ["Export selected", p.onExportSelected, !p.canExport],
              ["Export all", p.onExportAll, !p.canExport],
            ].map(([l, fn, off]) => (
              <button
                key={l as string}
                disabled={off as boolean}
                onClick={() => {
                  (fn as () => void)();
                  close();
                }}
                className="block w-full rounded-md px-2.5 py-1.5 text-left hover:bg-raise disabled:opacity-35 disabled:hover:bg-transparent"
              >
                {l as string}
              </button>
            ))}
          </div>
        )}
      </Popover>

      <WindowControls />
    </header>
  );
}
