import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "./lib/api";
import type {
  CaptureResult, Macro, MacroEvent, PlaybackPayload, RecordingPayload, Settings, ToastPayload,
} from "./lib/types";
import TopBar from "./components/TopBar";
import Sidebar from "./components/Sidebar";
import EventList from "./components/EventList";
import MacroHeader from "./components/MacroHeader";
import CompactView from "./components/CompactView";
import { buildCards } from "./lib/cards";
import ContextMenu, { type MenuItem } from "./components/ContextMenu";
import { applyTheme, DEFAULT_THEME } from "./lib/themes";
import { domCodeToKey, uid } from "./lib/fmt";

interface Toast extends ToastPayload { id: number }

export default function App() {
  const [macros, setMacros] = useState<Macro[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [rec, setRec] = useState<RecordingPayload>({ state: "idle", countdown: 0, macroId: null });
  const [live, setLive] = useState<MacroEvent[]>([]);
  const [playing, setPlaying] = useState<Record<string, PlaybackPayload>>({});
  const [capturing, setCapturing] = useState<"trigger" | "key" | "input" | null>(null);
  const [pausedIds, setPausedIds] = useState<Set<string>>(new Set());
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [maximized, setMaximized] = useState(false);
  const [elevated, setElevated] = useState(false);
  const [clipboard, setClipboard] = useState<MacroEvent[]>([]);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renamingFolder, setRenamingFolder] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; items: MenuItem[] } | null>(null);

  const latest = useRef<Macro[]>([]);
  latest.current = macros;
  const saveTimers = useRef<Record<string, number>>({});
  const captureResolver = useRef<((r: CaptureResult) => void) | null>(null);
  const toastId = useRef(0);

  const pushToast = useCallback((level: ToastPayload["level"], message: string) => {
    const id = ++toastId.current;
    setToasts((t) => [...t, { id, level, message }]);
    window.setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 5000);
  }, []);
  const fail = useCallback((e: unknown) => pushToast("error", String(e)), [pushToast]);

  useEffect(() => {
    let dead = false;
    let unlisten: UnlistenFn[] = [];
    (async () => {
      const snap = await api.getState();
      setMacros(snap.macros);
      setSettings(snap.settings);
      setRec(snap.recording);
      setElevated(snap.elevated);
      setSelectedId(snap.selected ?? snap.macros[0]?.id ?? null);

      const subs = await Promise.all([
        listen<Macro[]>("macros-changed", (e) => setMacros(e.payload)),
        listen<RecordingPayload>("recording-state", (e) => {
          setRec(e.payload);
          if (e.payload.state === "recording") setLive([]);
          if (e.payload.state === "stopped" || e.payload.state === "idle") setLive([]);
          if (e.payload.state === "stopped" && e.payload.macroId) setSelectedId(e.payload.macroId);
        }),
        listen<MacroEvent>("recording-event", (e) => setLive((l) => [...l, e.payload])),
        listen<PlaybackPayload>("playback-state", (e) =>
          setPlaying((p) => {
            const next = { ...p };
            if (e.payload.state === "playing") next[e.payload.macroId] = e.payload;
            else delete next[e.payload.macroId];
            return next;
          }),
        ),
        listen<{ macroId: string; paused: boolean }>("playback-paused", (e) =>
          setPausedIds((s) => {
            const n = new Set(s);
            if (e.payload.paused) n.add(e.payload.macroId);
            else n.delete(e.payload.macroId);
            return n;
          }),
        ),
        listen<CaptureResult>("capture-result", (e) => {
          if (e.payload.kind !== "cancelled") finishCapture(e.payload);
        }),
        listen<boolean>("window-state", (e) => setMaximized(e.payload)),
        listen<ToastPayload>("toast", (e) => pushToast(e.payload.level, e.payload.message)),
      ]);
      if (dead) subs.forEach((f) => f());
      else unlisten = subs;
    })().catch(fail);
    return () => {
      dead = true;
      unlisten.forEach((f) => f());
    };
  }, [fail, pushToast]);

  useEffect(() => { applyTheme(settings?.theme ?? DEFAULT_THEME); }, [settings?.theme]);

  useEffect(() => { api.selectMacro(selectedId).catch(() => {}); }, [selectedId]);

  const scheduleSave = (id: string) => {
    window.clearTimeout(saveTimers.current[id]);
    saveTimers.current[id] = window.setTimeout(() => {
      delete saveTimers.current[id];
      const m = latest.current.find((x) => x.id === id);
      if (m) api.saveMacro(m).catch(fail);
    }, 250);
  };
  const flushSaves = async () => {
    const ids = Object.keys(saveTimers.current);
    for (const id of ids) {
      window.clearTimeout(saveTimers.current[id]);
      delete saveTimers.current[id];
      const m = latest.current.find((x) => x.id === id);
      if (m) await api.saveMacro(m);
    }
  };
  const updateMacro = (id: string, fn: (m: Macro) => Macro) => {
    setMacros((ms) => ms.map((m) => (m.id === id ? fn(m) : m)));
    scheduleSave(id);
  };

  const finishCapture = useCallback((r: CaptureResult) => {
    setCapturing(null);
    captureResolver.current?.(r);
    captureResolver.current = null;
  }, []);

  const requestCapture = useCallback((kind: "trigger" | "key" | "input") => {
    return new Promise<CaptureResult>((resolve) => {
      captureResolver.current = resolve;
      setCapturing(kind);
      api.startCapture(kind).catch((e) => {
        setCapturing(null);
        captureResolver.current = null;
        fail(e);
        resolve({ kind: "cancelled" });
      });
    });
  }, [fail]);

  useEffect(() => {
    if (!capturing) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const isMod = /^(Control|Shift|Alt|Meta)(Left|Right)$/.test(e.code);
      let result: CaptureResult | null = null;
      if (e.code === "Escape") result = { kind: "cancelled" };
      else if (capturing === "key" || capturing === "input") {
        const key = domCodeToKey(e.code);
        if (key) result = { kind: "key", key };
      } else if (!isMod && !e.repeat) {
        const parts = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super", e.code].filter(Boolean) as string[];
        result = { kind: "trigger", trigger: { kind: "keyboard", accelerator: parts.join("+") } };
      }
      if (!result) return;
      const r = result;
      api.cancelCapture().catch(() => {}).finally(() => finishCapture(r));
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [capturing, finishCapture]);

  const sel = macros.find((m) => m.id === selectedId) ?? null;
  const playingIds = new Set(Object.keys(playing));
  const recBusy = rec.state === "countdown" || rec.state === "recording";

  async function createMacro(folder: string | null = null) {
    try {
      const m = await api.createMacro();
      setMacros((ms) => [...ms, { ...m, folder }]);
      if (folder) scheduleSave(m.id);
      setSelectedId(m.id);
    } catch (e) { fail(e); }
  }

  async function deleteMacro(id: string) {
    try {
      await api.deleteMacro(id);
      const rest = macros.filter((m) => m.id !== id);
      setMacros(rest);
      if (selectedId === id) setSelectedId(rest[0]?.id ?? null);
    } catch (e) { fail(e); }
  }

  function moveMacro(id: string, folder: string | null, beforeId: string | null) {
    const from = macros.findIndex((m) => m.id === id);
    if (from < 0) return;
    const next = macros.slice();
    const [mv] = next.splice(from, 1);
    const target = beforeId ? macros.findIndex((m) => m.id === beforeId) : -1;
    next.splice(target >= 0 ? Math.min(target, next.length) : next.length, 0, { ...mv, folder });
    setMacros(next);
    if ((mv.folder ?? null) !== folder) scheduleSave(id);
    api.reorderMacros(next.map((m) => m.id)).catch(fail);
  }

  async function duplicateMacro(src: Macro) {
    try {
      const m = await api.createMacro();
      const copy: Macro = { ...src, id: m.id, name: `${src.name} copy`, trigger: null, events: src.events.map((e) => ({ ...e, id: uid() })) };
      setMacros((ms) => [...ms, copy]);
      await api.saveMacro(copy);
      setSelectedId(m.id);
    } catch (e) { fail(e); }
  }

  function macroMenu(e: React.MouseEvent, id: string | null) {
    e.preventDefault();
    e.stopPropagation();
    const m = id ? macros.find((x) => x.id === id) : null;
    if (m) setSelectedId(m.id);
    const folders = settings?.folders ?? [];
    const items: MenuItem[] = m
      ? [
          { label: "Rename", onSelect: () => setRenamingId(m.id) },
          {
            label: "Move to folder",
            children: [{ label: "No folder", disabled: !m.folder, onSelect: () => moveMacro(m.id, null, null) }].concat(
              folders.map((f) => ({ label: f, disabled: m.folder === f, onSelect: () => moveMacro(m.id, f, null) })),
            ),
          },
          { label: m.enabled ? "Disable" : "Enable", onSelect: () => updateMacro(m.id, (x) => ({ ...x, enabled: !x.enabled })) },
          { label: "Duplicate", onSelect: () => duplicateMacro(m) },
          { separator: true },
          { label: "Delete", danger: true, onSelect: () => { if (window.confirm(`Delete "${m.name}"?`)) deleteMacro(m.id); } },
        ]
      : [{ label: "New macro", onSelect: () => createMacro() }, { label: "New folder", onSelect: createFolder }];
    setMenu({ x: e.clientX, y: e.clientY, items });
  }

  function folderMenu(e: React.MouseEvent, name: string) {
    e.preventDefault();
    e.stopPropagation();
    setMenu({
      x: e.clientX,
      y: e.clientY,
      items: [
        { label: "New macro here", onSelect: () => createMacro(name) },
        { label: "Rename", onSelect: () => setRenamingFolder(name) },
        { separator: true },
        { label: "Delete folder", hint: "keeps macros", danger: true, onSelect: () => deleteFolder(name) },
      ],
    });
  }

  async function startRecording() {
    try {
      await flushSaves();
      await api.startRecording(selectedId);
    } catch (e) { fail(e); }
  }

  function stopAll() {
    if (recBusy) api.stopRecording().catch(fail);
    if (playingIds.size) api.stopPlayback(null).catch(fail);
  }

  async function setTrigger() {
    if (!sel) return;
    const r = await requestCapture("trigger");
    if (r.kind === "trigger") updateMacro(sel.id, (m) => ({ ...m, trigger: r.trigger }));
  }

  async function addPriority(id: string) {
    const r = await requestCapture("input");
    if (r.kind !== "key") return;
    updateMacro(id, (m) => (m.priorityKeys.includes(r.key) ? m : { ...m, priorityKeys: [...m.priorityKeys, r.key] }));
  }

  async function pickToggleHotkey() {
    if (!settings) return;
    const r = await requestCapture("trigger");
    if (r.kind !== "trigger") return;
    if (r.trigger.kind === "keyboard") changeSettings({ ...settings, toggleHotkey: r.trigger.accelerator });
    else pushToast("warn", "The show / hide hotkey has to be a keyboard shortcut.");
  }

  async function pickRecordHotkey() {
    if (!settings) return;
    const r = await requestCapture("trigger");
    if (r.kind !== "trigger") return;
    if (r.trigger.kind === "keyboard") changeSettings({ ...settings, recordHotkey: r.trigger.accelerator });
    else pushToast("warn", "The record hotkey has to be a keyboard shortcut.");
  }

  const changeSettings = (s: Settings) => {
    setSettings(s);
    api.updateSettings(s).catch(fail);
  };

  const settingsRef = useRef(settings);
  settingsRef.current = settings;
  const opacityTimer = useRef<number>();

  const setOpacity = (v: number) => {
    setSettings((cur) => (cur ? { ...cur, opacity: v } : cur));
    window.clearTimeout(opacityTimer.current);
    opacityTimer.current = window.setTimeout(() => {
      const cur = settingsRef.current;
      if (cur) api.updateSettings(cur).catch(fail);
    }, 250);
  };

  async function setCompact(on: boolean) {
    if (!settings) return;
    let next: Settings;
    if (on) {
      const cards = buildCards(macros, settings.folders);
      const valid = cards.some((c) => c.key === settings.compactCard);
      next = { ...settings, compact: true, compactCard: valid ? settings.compactCard : cards[0].key };
    } else {
      next = { ...settings, compact: false };
    }
    setSettings(next);
    try {
      await api.updateSettings(next);
      await api.windowCompact(on, 220);
    } catch (e) {
      fail(e);
    }
  }

  function createFolder() {
    if (!settings) return;
    let n = 1;
    while (settings.folders.includes(`Folder ${n}`)) n++;
    const name = `Folder ${n}`;
    changeSettings({ ...settings, folders: [...settings.folders, name] });
    setRenamingFolder(name);
  }

  function renameFolder(oldName: string, raw: string | null) {
    setRenamingFolder(null);
    const name = raw?.trim();
    if (!settings || !name || name === oldName) return;
    if (settings.folders.includes(name)) {
      pushToast("warn", `A folder called "${name}" already exists.`);
      return;
    }
    changeSettings({ ...settings, folders: settings.folders.map((f) => (f === oldName ? name : f)) });
    const affected = macros.filter((m) => m.folder === oldName).map((m) => m.id);
    setMacros((ms) => ms.map((m) => (m.folder === oldName ? { ...m, folder: name } : m)));
    affected.forEach(scheduleSave);
  }

  function deleteFolder(name: string) {
    if (!settings) return;
    changeSettings({ ...settings, folders: settings.folders.filter((f) => f !== name) });
    const affected = macros.filter((m) => m.folder === name).map((m) => m.id);
    setMacros((ms) => ms.map((m) => (m.folder === name ? { ...m, folder: null } : m)));
    affected.forEach(scheduleSave);
  }

  function reorderFolders(from: string, to: string) {
    if (!settings) return;
    const list = settings.folders.filter((f) => f !== from);
    const at = settings.folders.indexOf(to);
    if (at < 0 || !settings.folders.includes(from)) return;
    list.splice(at, 0, from);
    changeSettings({ ...settings, folders: list });
  }

  const importMacros = async () => {
    try {
      const n = await api.importMacros();
      if (n > 0) pushToast("info", `Imported ${n} macro${n === 1 ? "" : "s"}.`);
    } catch (e) { fail(e); }
  };
  const exportMacros = async (ids: string[] | null) => {
    try {
      await flushSaves();
      if (await api.exportMacros(ids)) pushToast("info", "Exported.");
    } catch (e) { fail(e); }
  };

  if (!settings) return <div className="grid h-full place-items-center text-mute">Loading…</div>;

  return (
    <div
      className={`flex h-full flex-col overflow-hidden bg-base text-fg ${maximized ? "" : "rounded-xl border border-line"}`}
      style={settings.compact ? { opacity: settings.opacity / 100 } : undefined}
      onContextMenu={(e) => {
        const tag = (e.target as HTMLElement).tagName;
        if (tag !== "INPUT" && tag !== "TEXTAREA") e.preventDefault();
      }}
    >
      {settings.compact ? (
        <CompactView
          macros={macros}
          settings={settings}
          playing={playingIds}
          paused={pausedIds}
          onSettings={changeSettings}
          onOpacity={setOpacity}
          onToggle={(id, enabled) => updateMacro(id, (m) => ({ ...m, enabled }))}
          onExpand={() => setCompact(false)}
        />
      ) : (
        <>
          <TopBar
            rec={rec}
            settings={settings}
            anyPlaying={playingIds.size > 0}
            canExport={macros.length > 0}
            onSettings={changeSettings}
            onRecord={startRecording}
            onStop={stopAll}
            onPickHotkey={pickRecordHotkey}
            onPickToggleHotkey={pickToggleHotkey}
            elevated={elevated}
            onRestartAdmin={() => api.restartAsAdmin().catch(fail)}
            onCompact={() => setCompact(true)}
            onImport={importMacros}
            onExportSelected={() => sel && exportMacros([sel.id])}
            onExportAll={() => exportMacros(null)}
          />

          <div className="flex min-h-0 flex-1">
            <Sidebar
              macros={macros}
              folders={settings.folders}
              selectedId={selectedId}
              playing={playingIds}
              renamingId={renamingId}
              renamingFolder={renamingFolder}
              onSelect={setSelectedId}
              onCreate={() => createMacro()}
              onCreateFolder={createFolder}
              onStartFolderRename={setRenamingFolder}
              onFolderRename={renameFolder}
              onFolderContext={folderMenu}
              onReorderFolders={reorderFolders}
              onToggle={(id, enabled) => updateMacro(id, (m) => ({ ...m, enabled }))}
              onStartRename={setRenamingId}
              onRename={(id, name) => {
                updateMacro(id, (m) => ({ ...m, name }));
                setRenamingId(null);
              }}
              onContext={macroMenu}
              onMove={moveMacro}
            />

            <main className="flex min-w-0 flex-1 flex-col">
              {sel ? (
                <>
                  <MacroHeader
                    macro={sel}
                    playing={playing[sel.id]}
                    onSetTrigger={setTrigger}
                    onClearTrigger={() => updateMacro(sel.id, (m) => ({ ...m, trigger: null }))}
                    onProcess={(process) => updateMacro(sel.id, (m) => ({ ...m, process }))}
                    onAddPriority={() => addPriority(sel.id)}
                    paused={pausedIds.has(sel.id)}
                    onChange={(patch) => updateMacro(sel.id, (m) => ({ ...m, ...patch }))}
                    onPlay={() => flushSaves().then(() => api.playMacro(sel.id)).catch(fail)}
                    onStop={() => api.stopPlayback(sel.id).catch(fail)}
                  />
                  <EventList
                    key={sel.id}
                    events={sel.events}
                    live={live}
                    recording={rec.state === "recording" && (rec.macroId ?? selectedId) === sel.id}
                    playingIndex={playing[sel.id]?.eventIndex ?? null}
                    clipboard={clipboard}
                    setClipboard={setClipboard}
                    onChange={(events) => updateMacro(sel.id, (m) => ({ ...m, events }))}
                    requestCapture={requestCapture}
                  />
                </>
              ) : (
                <div className="grid flex-1 place-items-center text-center text-mute">
                  <div>
                    <p className="text-fg">No macro selected</p>
                    <button onClick={() => createMacro()} className="mt-3 rounded-md bg-accent px-4 py-1.5 text-sm font-medium text-accent-fg">
                      New macro
                    </button>
                  </div>
                </div>
              )}
            </main>
          </div>
        </>
      )}

      {rec.state === "countdown" && (
        <div className="pointer-events-none fixed inset-0 z-30 grid place-items-center bg-black/60">
          <div className="text-center">
            <div className="font-mono text-[9rem] leading-none text-rec">{rec.countdown}</div>
            <p className="mt-2 text-mute">Switch to your game. Recording starts at zero.</p>
          </div>
        </div>
      )}

      {capturing && (
        <div className="fixed inset-0 z-50 grid place-items-center bg-black/70">
          <div className="w-[400px] rounded-xl border border-line bg-panel p-6 text-center shadow-2xl">
            <p className="text-lg font-medium">{capturing === "trigger" ? "Press a key combo or mouse button" : capturing === "input" ? "Press a key or mouse button" : "Press a key"}</p>
            <p className="mt-2 text-sm text-mute">
              {capturing === "trigger" ? "Hold Ctrl, Alt or Shift first for a combo. Left click can't be used. Esc cancels." : capturing === "input" ? "Left click can't be used. Esc cancels." : "Esc cancels."}
            </p>
            <button
              onClick={() => api.cancelCapture().catch(() => {}).finally(() => finishCapture({ kind: "cancelled" }))}
              className="mt-5 rounded-md border border-line px-4 py-1.5 text-sm hover:bg-raise"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {menu && <ContextMenu x={menu.x} y={menu.y} items={menu.items} onClose={() => setMenu(null)} />}

      <div className="pointer-events-none fixed bottom-4 right-4 z-[80] flex w-96 flex-col gap-2">
        {toasts.map((t) => (
          <div
            key={t.id}
            className={`rounded-lg border bg-panel px-4 py-2.5 text-sm shadow-xl ${
              t.level === "error" ? "border-rec/60 text-rec" : t.level === "warn" ? "border-accent/60 text-accent" : "border-line text-fg"
            }`}
          >
            {t.message}
          </div>
        ))}
      </div>
    </div>
  );
}
