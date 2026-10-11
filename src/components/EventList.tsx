import { useEffect, useMemo, useRef, useState } from "react";
import {
  ChevronsUpDown, Clock, GripVertical, Keyboard, Mouse, MousePointerClick, Move, Repeat, Terminal, Type, X,
  type LucideIcon,
} from "lucide-react";
import type { CaptureResult, EventKind, MacroEvent } from "../lib/types";
import { prettyKey, uid } from "../lib/fmt";
import ContextMenu, { type MenuItem } from "./ContextMenu";

type EType = MacroEvent["type"];

const META: Record<EType, { label: string; Icon: LucideIcon }> = {
  keyPress: { label: "Key press", Icon: Keyboard },
  keyRelease: { label: "Key release", Icon: Keyboard },
  keyTap: { label: "Key tap", Icon: Keyboard },
  keyHold: { label: "Key hold", Icon: Keyboard },
  mouseDown: { label: "Mouse down", Icon: Mouse },
  mouseUp: { label: "Mouse up", Icon: Mouse },
  mouseClick: { label: "Click", Icon: MousePointerClick },
  mouseMove: { label: "Move", Icon: Move },
  scroll: { label: "Scroll", Icon: ChevronsUpDown },
  delay: { label: "Delay", Icon: Clock },
  typeText: { label: "Type text", Icon: Type },
  runProgram: { label: "Run program", Icon: Terminal },
  loopStart: { label: "Loop", Icon: Repeat },
  loopEnd: { label: "End loop", Icon: Repeat },
};

type AddKind = "keyTap" | "keyHold" | "mouseClick" | "mouseMove" | "scroll" | "delay" | "typeText" | "runProgram";

const ADD_KINDS: { id: AddKind; label: string }[] = [
  { id: "keyTap", label: "Key tap" },
  { id: "keyHold", label: "Key hold" },
  { id: "mouseClick", label: "Mouse click" },
  { id: "mouseMove", label: "Mouse move" },
  { id: "scroll", label: "Scroll" },
  { id: "delay", label: "Delay" },
  { id: "typeText", label: "Type text" },
  { id: "runProgram", label: "Run program" },
];

function defaults(kind: AddKind): EventKind {
  switch (kind) {
    case "keyTap": return { type: "keyTap", key: "" };
    case "keyHold": return { type: "keyHold", key: "", durationMs: 100 };
    case "mouseClick": return { type: "mouseClick", button: "Left" };
    case "mouseMove": return { type: "mouseMove", x: 0, y: 0, relative: false };
    case "scroll": return { type: "scroll", dx: 0, dy: -1 };
    case "delay": return { type: "delay", ms: 25 };
    case "typeText": return { type: "typeText", text: "", charDelayMs: 0 };
    case "runProgram": return { type: "runProgram", path: "", args: "" };
  }
}

const BUTTONS = ["Left", "Right", "Middle", "X1", "X2"];

const numCls =
  "rounded bg-transparent px-1.5 py-0.5 text-right font-mono text-xs text-fg ring-1 ring-transparent hover:ring-line focus:ring-accent outline-none";
const textCls =
  "rounded bg-transparent px-2 py-0.5 font-mono text-xs ring-1 ring-line/60 placeholder:text-mute hover:ring-line focus:ring-accent outline-none";

function Num({ value, onChange, min = 0, w = "w-16" }: { value: number; onChange: (n: number) => void; min?: number; w?: string }) {
  return (
    <input
      type="number"
      min={min}
      value={value}
      onClick={(e) => e.stopPropagation()}
      onChange={(e) => {
        const n = parseInt(e.target.value, 10);
        onChange(Number.isFinite(n) ? Math.max(min, n) : min);
      }}
      className={`${numCls} ${w}`}
    />
  );
}

function ChipButton({ children, onClick, empty, title }: { children: React.ReactNode; onClick: (e: React.MouseEvent) => void; empty?: boolean; title: string }) {
  return (
    <button
      title={title}
      onClick={(e) => {
        e.stopPropagation();
        onClick(e);
      }}
      className={`rounded border px-2 py-0.5 font-mono text-xs hover:border-accent ${empty ? "border-accent/60 text-accent" : "border-line bg-panel"}`}
    >
      {children}
    </button>
  );
}

interface ValueProps {
  ev: MacroEvent;
  set: (p: Record<string, unknown>) => void;
  pickKey: () => void;
  pickButton: (e: React.MouseEvent) => void;
  autoFocus: boolean;
}

function Value({ ev, set, pickKey, pickButton, autoFocus }: ValueProps) {
  const keyChip = (k: string) => (
    <ChipButton title="Click, then press the key you want" empty={!k} onClick={pickKey}>
      {k ? prettyKey(k) : "Choose key"}
    </ChipButton>
  );
  switch (ev.type) {
    case "keyPress":
    case "keyRelease":
    case "keyTap":
      return keyChip(ev.key);
    case "keyHold":
      return (
        <span className="flex items-center gap-2">
          {keyChip(ev.key)}
          <Num value={ev.durationMs} min={1} onChange={(n) => set({ durationMs: n })} />
          <span className="text-xs text-mute">ms</span>
        </span>
      );
    case "mouseDown":
    case "mouseUp":
    case "mouseClick":
      return (
        <ChipButton title="Click to change the button" onClick={pickButton}>
          {ev.button}
        </ChipButton>
      );
    case "mouseMove":
      return (
        <span className="flex items-center gap-1">
          <Num value={ev.x} min={-99999} onChange={(n) => set({ x: n })} />
          <Num value={ev.y} min={-99999} onChange={(n) => set({ y: n })} />
          <button
            onClick={(e) => {
              e.stopPropagation();
              set({ relative: !ev.relative });
            }}
            title="Switch between absolute and relative"
            className="rounded px-1.5 py-0.5 text-xs text-mute hover:text-fg"
          >
            {ev.relative ? "relative" : "absolute"}
          </button>
        </span>
      );
    case "scroll":
      return (
        <span className="flex items-center gap-1 text-xs text-mute">
          <Num value={ev.dy} min={-100} w="w-12" onChange={(n) => set({ dy: n })} /> up/down
          <Num value={ev.dx} min={-100} w="w-12" onChange={(n) => set({ dx: n })} /> left/right
        </span>
      );
    case "delay":
      return (
        <span className="flex items-center gap-1.5">
          <Num value={ev.ms} min={1} onChange={(n) => set({ ms: n })} />
          <span className="text-xs text-mute">ms</span>
        </span>
      );
    case "typeText":
      return (
        <input
          type="text"
          autoFocus={autoFocus}
          placeholder="Text to type"
          value={ev.text}
          onClick={(e) => e.stopPropagation()}
          onChange={(e) => set({ text: e.target.value })}
          className={`${textCls} w-64`}
        />
      );
    case "runProgram":
      return (
        <span className="flex items-center gap-1.5">
          <input
            type="text"
            autoFocus={autoFocus}
            placeholder="Program path"
            value={ev.path}
            onClick={(e) => e.stopPropagation()}
            onChange={(e) => set({ path: e.target.value })}
            className={`${textCls} w-56`}
          />
          <input
            type="text"
            placeholder="Arguments"
            value={ev.args}
            onClick={(e) => e.stopPropagation()}
            onChange={(e) => set({ args: e.target.value })}
            className={`${textCls} w-36`}
          />
        </span>
      );
    case "loopStart":
      return (
        <span className="flex items-center gap-1.5">
          <Num value={ev.count} onChange={(n) => set({ count: n })} w="w-12" />
          <span className="text-xs text-mute">times (0 = until stopped)</span>
        </span>
      );
    case "loopEnd":
      return <span className="text-xs text-mute">end of loop</span>;
  }
}

const GRID = "grid grid-cols-[16px_18px_88px_minmax(0,1fr)_88px_18px] items-center gap-2";

interface Props {
  events: MacroEvent[];
  live: MacroEvent[];
  recording: boolean;
  playingIndex: number | null;
  clipboard: MacroEvent[];
  setClipboard: (e: MacroEvent[]) => void;
  onChange: (events: MacroEvent[]) => void;
  requestCapture: (kind: "key") => Promise<CaptureResult>;
}

export default function EventList({ events, live, recording, playingIndex, clipboard, setClipboard, onChange: emit, requestCapture }: Props) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [menu, setMenu] = useState<{ x: number; y: number; items: MenuItem[] } | null>(null);
  const [focusId, setFocusId] = useState<string | null>(null);
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [dragOver, setDragOver] = useState<number | null>(null);
  const [armed, setArmed] = useState<string | null>(null);
  const anchor = useRef(-1);
  const liveEnd = useRef<HTMLDivElement>(null);
  const eventsRef = useRef(events);
  eventsRef.current = events;

  const history = useRef({ past: [] as MacroEvent[][], future: [] as MacroEvent[][], key: "", t: 0 });
  const lastSent = useRef(events);
  useEffect(() => {
    if (events !== lastSent.current) {
      history.current = { past: [], future: [], key: "", t: 0 };
      lastSent.current = events;
    }
  }, [events]);

  function onChange(next: MacroEvent[], key = "") {
    const h = history.current;
    const now = Date.now();
    if (!(key !== "" && key === h.key && now - h.t < 1200)) {
      h.past.push(eventsRef.current);
      if (h.past.length > 100) h.past.shift();
    }
    h.key = key;
    h.t = now;
    h.future = [];
    lastSent.current = next;
    emit(next);
  }
  function undo() {
    const h = history.current;
    const prev = h.past.pop();
    if (!prev) return;
    h.future.push(eventsRef.current);
    h.key = "";
    lastSent.current = prev;
    emit(prev);
    setSelected(new Set());
  }
  function redo() {
    const h = history.current;
    const next = h.future.pop();
    if (!next) return;
    h.past.push(eventsRef.current);
    h.key = "";
    lastSent.current = next;
    emit(next);
    setSelected(new Set());
  }

  useEffect(() => { liveEnd.current?.scrollIntoView({ block: "end" }); }, [live.length]);

  const depths = useMemo(() => {
    let d = 0;
    return events.map((e) => {
      if (e.type === "loopEnd") d = Math.max(0, d - 1);
      const here = d;
      if (e.type === "loopStart") d++;
      return here;
    });
  }, [events]);

  const selIdx = events.map((e, i) => (selected.has(e.id) ? i : -1)).filter((i) => i >= 0);
  const afterSelection = selIdx.length ? Math.max(...selIdx) + 1 : events.length;

  const update = (i: number, next: MacroEvent) => onChange(events.map((e, k) => (k === i ? next : e)), `e:${events[i].id}`);
  const remove = (i: number) => onChange(events.filter((_, k) => k !== i));
  const insertAt = (at: number, items: MacroEvent[]) => onChange([...events.slice(0, at), ...items, ...events.slice(at)]);
  const clone = (items: MacroEvent[]) => items.map((e) => ({ ...e, id: uid() }));
  const patchById = (id: string, p: Record<string, unknown>) =>
    onChange(eventsRef.current.map((e) => (e.id === id ? ({ ...e, ...p } as MacroEvent) : e)), `e:${id}`);

  async function pickKey(id: string, isNew = false) {
    const r = await requestCapture("key");
    if (r.kind === "key") {
      patchById(id, { key: r.key });
    } else if (isNew) {
      const cur = eventsRef.current.find((e) => e.id === id);
      if (cur && "key" in cur && cur.key === "") onChange(eventsRef.current.filter((e) => e.id !== id));
    }
  }

  function addAction(kind: AddKind, at: number) {
    const ev = { id: uid(), delayMs: 0, ...defaults(kind) } as MacroEvent;
    insertAt(at, [ev]);
    setSelected(new Set([ev.id]));
    if (kind === "keyTap" || kind === "keyHold") pickKey(ev.id, true);
    else if (kind === "typeText" || kind === "runProgram") setFocusId(ev.id);
  }

  function removeSelected() {
    if (!selIdx.length) return;
    onChange(events.filter((e) => !selected.has(e.id)));
    setSelected(new Set());
  }
  const copySelected = () => selIdx.length && setClipboard(selIdx.map((i) => events[i]));
  const paste = (at: number) => clipboard.length && insertAt(at, clone(clipboard));
  const duplicate = () => selIdx.length && insertAt(afterSelection, clone(selIdx.map((i) => events[i])));

  function wrapLoop(count: number) {
    if (!selIdx.length) return;
    const lo = Math.min(...selIdx), hi = Math.max(...selIdx);
    onChange([
      ...events.slice(0, lo),
      { id: uid(), delayMs: 0, type: "loopStart", count },
      ...events.slice(lo, hi + 1),
      { id: uid(), delayMs: 0, type: "loopEnd" },
      ...events.slice(hi + 1),
    ]);
  }

  function drop(to: number) {
    if (dragFrom === null || dragFrom === to) return;
    const next = events.slice();
    const [moved] = next.splice(dragFrom, 1);
    next.splice(to, 0, moved);
    onChange(next);
  }

  function clickRow(i: number, e: React.MouseEvent) {
    const id = events[i].id;
    if (e.shiftKey && anchor.current >= 0) {
      const [a, b] = [Math.min(anchor.current, i), Math.max(anchor.current, i)];
      setSelected(new Set(events.slice(a, b + 1).map((x) => x.id)));
    } else if (e.ctrlKey || e.metaKey) {
      const n = new Set(selected);
      if (n.has(id)) n.delete(id); else n.add(id);
      setSelected(n);
      anchor.current = i;
    } else {
      setSelected(new Set([id]));
      anchor.current = i;
    }
  }

  function onKey(e: React.KeyboardEvent) {
    if ((e.target as HTMLElement).tagName === "INPUT") return;
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === "Escape") setSelected(new Set());
    else if (e.key === "Delete" || e.key === "Backspace") removeSelected();
    else if (mod && e.key.toLowerCase() === "z") { e.preventDefault(); if (e.shiftKey) redo(); else undo(); }
    else if (mod && e.key.toLowerCase() === "y") { e.preventDefault(); redo(); }
    else if (mod && e.key === "a") { e.preventDefault(); setSelected(new Set(events.map((x) => x.id))); }
    else if (mod && e.key === "c") copySelected();
    else if (mod && e.key === "v") paste(afterSelection);
    else if (mod && e.key === "d") { e.preventDefault(); duplicate(); }
  }

  const addMenu = (at: number): MenuItem[] => ADD_KINDS.map((k) => ({ label: k.label, onSelect: () => addAction(k.id, at) }));

  function emptyMenu(e: React.MouseEvent) {
    e.preventDefault();
    setMenu({
      x: e.clientX,
      y: e.clientY,
      items: [
        { label: "Add action", children: addMenu(events.length) },
        { separator: true },
        { label: "Undo", hint: "Ctrl+Z", disabled: !history.current.past.length, onSelect: undo },
        { label: "Redo", hint: "Ctrl+Y", disabled: !history.current.future.length, onSelect: redo },
        { separator: true },
        { label: "Paste", hint: "Ctrl+V", disabled: !clipboard.length, onSelect: () => paste(events.length) },
        { label: "Select all", hint: "Ctrl+A", disabled: !events.length, onSelect: () => setSelected(new Set(events.map((x) => x.id))) },
      ],
    });
  }

  function rowMenu(e: React.MouseEvent, i: number) {
    e.preventDefault();
    e.stopPropagation();
    const targets = selected.has(events[i].id) ? selIdx : [i];
    if (!selected.has(events[i].id)) setSelected(new Set([events[i].id]));
    const lo = Math.min(...targets), hi = Math.max(...targets);
    setMenu({
      x: e.clientX,
      y: e.clientY,
      items: [
        { label: "Insert above", children: addMenu(lo) },
        { label: "Insert below", children: addMenu(hi + 1) },
        { separator: true },
        { label: "Copy", hint: "Ctrl+C", onSelect: () => setClipboard(targets.map((t) => events[t])) },
        { label: "Paste below", hint: "Ctrl+V", disabled: !clipboard.length, onSelect: () => paste(hi + 1) },
        { label: "Duplicate", hint: "Ctrl+D", onSelect: () => insertAt(hi + 1, clone(targets.map((t) => events[t]))) },
        {
          label: "Wrap in loop",
          children: [2, 3, 5, 10].map((n) => ({ label: `${n} times`, onSelect: () => wrapLoop(n) })).concat([{ label: "Until stopped", onSelect: () => wrapLoop(0) }]),
        },
        { separator: true },
        { label: targets.length > 1 ? `Delete ${targets.length} actions` : "Delete", hint: "Del", danger: true, onSelect: () => onChange(events.filter((_, k) => !targets.includes(k))) },
      ],
    });
  }

  function buttonMenu(e: React.MouseEvent, id: string) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setMenu({ x: r.left, y: r.bottom + 4, items: BUTTONS.map((b) => ({ label: b, onSelect: () => patchById(id, { button: b }) })) });
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <div
        tabIndex={0}
        onKeyDown={onKey}
        onContextMenu={emptyMenu}
        onClick={() => setSelected(new Set())}
        className="flex-1 overflow-y-auto px-2 pb-2 outline-none"
      >
        <div className="flex min-h-full flex-col" onDragEnd={() => { setDragFrom(null); setDragOver(null); setArmed(null); }}>
          {events.map((ev, i) => {
            const { Icon, label } = META[ev.type];
            const isSel = selected.has(ev.id);
            const tone = playingIndex === i
              ? "bg-accent/15"
              : isSel
                ? "bg-raise ring-1 ring-accent/50 shadow-[inset_2px_0_0_var(--accent)]"
                : i % 2 === 0 ? "bg-raise/45 hover:bg-raise/80" : "hover:bg-raise/80";
            return (
              <div
                key={ev.id}
                draggable={armed === ev.id}
                onDragStart={(e) => { setDragFrom(i); e.dataTransfer.effectAllowed = "move"; e.dataTransfer.setData("text/plain", String(i)); }}
                onDragOver={(e) => { e.preventDefault(); setDragOver(i); }}
                onDrop={(e) => { e.preventDefault(); drop(i); setDragFrom(null); setDragOver(null); }}
                onClick={(e) => { e.stopPropagation(); clickRow(i, e); }}
                onContextMenu={(e) => rowMenu(e, i)}
                className={`group ${GRID} cursor-default rounded-md border-t-2 px-2 py-1.5 text-sm ${
                  dragOver === i && dragFrom !== null && dragFrom !== i ? "border-accent" : "border-transparent"
                } ${tone} ${dragFrom === i ? "opacity-40" : ""}`}
              >
                <span
                  className="cursor-grab text-mute opacity-0 group-hover:opacity-100"
                  title="Drag to reorder"
                  onMouseDown={() => setArmed(ev.id)}
                  onMouseUp={() => setArmed(null)}
                >
                  <GripVertical size={14} />
                </span>
                <Icon size={15} className="text-mute" />
                <span className="text-xs text-mute">{label}</span>
                <div className="min-w-0" style={{ paddingLeft: depths[i] * 16 }}>
                  <Value
                    ev={ev}
                    autoFocus={focusId === ev.id}
                    set={(p) => update(i, { ...ev, ...p } as MacroEvent)}
                    pickKey={() => pickKey(ev.id)}
                    pickButton={(e) => buttonMenu(e, ev.id)}
                  />
                </div>
                <div className="flex items-center justify-end gap-1">
                  <Num value={ev.delayMs} w="w-14" onChange={(n) => update(i, { ...ev, delayMs: n })} />
                  <span className="text-xs text-mute">ms</span>
                </div>
                <button
                  title="Delete"
                  onClick={(e) => { e.stopPropagation(); remove(i); }}
                  className="text-mute opacity-0 hover:text-rec group-hover:opacity-100"
                >
                  <X size={14} />
                </button>
              </div>
            );
          })}

          {recording && (
            <div className="mt-2">
              <div className="px-2 py-1 text-xs text-rec">Recording. New actions are added when you stop.</div>
              {live.map((ev) => {
                const { Icon, label } = META[ev.type];
                return (
                  <div key={ev.id} className={`${GRID} px-2 py-0.5 text-xs text-mute`}>
                    <span /><Icon size={14} /><span>{label}</span>
                    <span className="font-mono">
                      {"key" in ev ? prettyKey(ev.key) : "button" in ev ? ev.button : ev.type === "mouseMove" ? `${ev.x}, ${ev.y}` : ev.type === "scroll" ? `${ev.dy}` : ""}
                    </span>
                    <span className="text-right font-mono">{ev.delayMs} ms</span>
                    <span />
                  </div>
                );
              })}
              <div ref={liveEnd} />
            </div>
          )}

          {!recording && events.length === 0 && (
            <p className="mx-auto mt-16 max-w-xs text-center text-sm text-mute">
              Nothing here yet. Press Record, or right-click anywhere to add an action.
            </p>
          )}
          <div className="min-h-16 flex-1" />
        </div>
      </div>

      {menu && <ContextMenu x={menu.x} y={menu.y} items={menu.items} onClose={() => setMenu(null)} />}
    </section>
  );
}
