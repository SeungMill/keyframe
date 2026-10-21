import { useEffect, useLayoutEffect, useMemo, useRef } from "react";
import { ChevronDown, ChevronLeft, ChevronRight, Droplet, Maximize2, Pin, X } from "lucide-react";
import { api } from "../lib/api";
import { buildCards } from "../lib/cards";
import { shortTrigger } from "../lib/fmt";
import type { Macro, Settings } from "../lib/types";

interface Props {
  macros: Macro[];
  settings: Settings;
  playing: Set<string>;
  paused: Set<string>;
  onSettings: (s: Settings) => void;
  onOpacity: (v: number) => void;
  onToggle: (id: string, enabled: boolean) => void;
  onExpand: () => void;
}

function Switch({ on, onChange }: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      role="switch"
      aria-checked={on}
      onClick={() => onChange(!on)}
      className={`relative h-[13px] w-6 shrink-0 rounded-full transition ${on ? "bg-accent" : "bg-line"}`}
    >
      <span
        className={`absolute top-[2px] h-[9px] w-[9px] rounded-full transition-all ${on ? "left-[13px] bg-accent-fg" : "left-[2px] bg-mute"}`}
      />
    </button>
  );
}

interface RowProps {
  m: Macro;
  running: boolean;
  paused: boolean;
  onToggle: (id: string, enabled: boolean) => void;
}

function MacroRow({ m, running, paused, onToggle }: RowProps) {
  return (
    <div className={`flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm ${m.enabled ? "bg-raise" : ""}`}>
      {running && <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${paused ? "bg-mute" : "animate-pulse bg-accent"}`} />}
      <span className={`min-w-0 flex-1 truncate ${m.enabled ? "" : "text-mute"}`}>{m.name}</span>
      <span className="rounded border border-line bg-panel px-1.5 font-mono text-[10.5px] text-mute">{shortTrigger(m.trigger) || "no trigger"}</span>
      <Switch on={m.enabled} onChange={(v) => onToggle(m.id, v)} />
    </div>
  );
}

export default function CompactView({ macros, settings, playing, paused, onSettings, onOpacity, onToggle, onExpand }: Props) {
  const cards = useMemo(() => buildCards(macros, settings.folders), [macros, settings.folders]);
  const idx = Math.max(0, cards.findIndex((c) => c.key === settings.compactCard));
  const card = cards[idx];
  const armed = card.macros.filter((m) => m.enabled);
  const off = card.macros.filter((m) => !m.enabled);
  const cardId = card.key ?? "";
  const showOff = settings.compactOffOpen.includes(cardId);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (card.key !== settings.compactCard) onSettings({ ...settings, compactCard: card.key });
  }, [card.key, settings.compactCard]);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const send = () => api.windowCompactSize(Math.ceil(el.getBoundingClientRect().height) + 2).catch(() => {});
    send();
    const ro = new ResizeObserver(send);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const go = (i: number) => onSettings({ ...settings, compactCard: cards[(i + cards.length) % cards.length].key });
  const iconBtn = "rounded p-1 hover:bg-raise hover:text-fg";
  const row = (m: Macro) => (
    <MacroRow key={m.id} m={m} running={playing.has(m.id)} paused={paused.has(m.id)} onToggle={onToggle} />
  );

  return (
    <div ref={ref}>
      <div data-tauri-drag-region className="flex items-center gap-0.5 border-b border-line bg-panel py-1 pl-3 pr-1.5">
        <span data-tauri-drag-region className="flex-1 text-[11px] text-mute">Keyframe</span>
        <button
          title={settings.alwaysOnTop ? "Always on top: on" : "Always on top: off"}
          onClick={() => onSettings({ ...settings, alwaysOnTop: !settings.alwaysOnTop })}
          className={`${iconBtn} ${settings.alwaysOnTop ? "bg-raise text-accent" : "text-mute"}`}
        >
          <Pin size={14} fill={settings.alwaysOnTop ? "currentColor" : "none"} />
        </button>
        <button title="Full window" onClick={onExpand} className={`${iconBtn} text-mute`}>
          <Maximize2 size={14} />
        </button>
        <button title="Close" onClick={() => api.windowClose()} className={`${iconBtn} text-mute hover:!bg-rec hover:!text-rec-fg`}>
          <X size={14} />
        </button>
      </div>

      <div className="flex items-center gap-1 px-2 pb-2 pt-3">
        <button title="Previous" onClick={() => go(idx - 1)} disabled={cards.length < 2} className={`${iconBtn} text-mute disabled:opacity-0`}>
          <ChevronLeft size={18} />
        </button>
        <div className="min-w-0 flex-1 text-center">
          <div className="truncate text-[15px] font-medium tracking-wide">{card.title}</div>
          <div className="text-[11px] text-mute">
            {armed.length} of {card.macros.length} armed
          </div>
        </div>
        <button title="Next" onClick={() => go(idx + 1)} disabled={cards.length < 2} className={`${iconBtn} text-mute disabled:opacity-0`}>
          <ChevronRight size={18} />
        </button>
      </div>

      {armed.length > 0 && <div className="grid gap-1 px-2 pb-2">{armed.map(row)}</div>}
      {armed.length === 0 && <p className="px-2 pb-2 text-center text-xs text-mute">Nothing armed here.</p>}

      {off.length > 0 && (
        <div className="px-2 pb-2">
          <button
            onClick={() =>
              onSettings({
                ...settings,
                compactOffOpen: showOff ? settings.compactOffOpen.filter((k) => k !== cardId) : [...settings.compactOffOpen, cardId],
              })
            }
            className="flex items-center gap-1 px-2.5 pb-1 text-[11px] text-mute hover:text-fg"
          >
            {showOff ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
            {off.length} off
          </button>
          {showOff && <div className="grid gap-1">{off.map(row)}</div>}
        </div>
      )}

      <div className="flex items-center gap-2 border-t border-line px-3 py-2">
        <Droplet size={13} className="shrink-0 text-mute" />
        <input
          type="range"
          min={30}
          max={100}
          value={settings.opacity}
          onChange={(e) => onOpacity(parseInt(e.target.value, 10))}
          className="min-w-0 flex-1 accent-accent"
        />
        <span className="w-8 text-right font-mono text-[10.5px] text-mute">{settings.opacity}%</span>
      </div>

      {cards.length > 1 && (
        <div className="flex justify-center gap-1.5 border-t border-line py-2">
          {cards.length <= 8 ? (
            cards.map((c, i) => (
              <button
                key={i}
                title={c.title}
                onClick={() => go(i)}
                className={`h-1.5 w-1.5 rounded-full ${i === idx ? "bg-accent" : "border border-mute"}`}
              />
            ))
          ) : (
            <span className="text-[11px] text-mute">
              {idx + 1} / {cards.length}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
