import { Play, Square, X } from "lucide-react";
import type { Macro, PlaybackPayload, PlayMode } from "../lib/types";
import { totalMs, triggerLabel } from "../lib/fmt";
import ProcessPicker from "./ProcessPicker";
import PriorityPicker from "./PriorityPicker";

interface Props {
  macro: Macro;
  playing?: PlaybackPayload;
  paused?: boolean;
  onSetTrigger: () => void;
  onClearTrigger: () => void;
  onProcess: (p: string | null) => void;
  onAddPriority: () => void;
  onChange: (patch: Partial<Macro>) => void;
  onPlay: () => void;
  onStop: () => void;
}

const MODE_HELP: Record<PlayMode, string> = {
  once: "Plays once per trigger press",
  count: "Plays N times per trigger press",
  toggle: "Press to start looping, press again to stop",
  whileHeld: "Loops while the trigger is held",
};

export default function MacroHeader({ macro: m, playing, paused, ...p }: Props) {
  const secs = totalMs(m.events) / 1000;
  return (
    <div className="flex items-center gap-2 px-4 py-3">
      <h2 className="min-w-0 truncate text-[15px] font-medium">{m.name}</h2>
      <span className="shrink-0 text-xs text-mute">
        {playing
          ? paused
            ? "Paused, resuming when you let go"
            : `Playing, pass ${playing.pass}${playing.total ? ` of ${playing.total}` : ""}`
          : `${m.events.length} actions, ${secs.toFixed(secs < 10 ? 2 : 1)} s`}
      </span>
      <div className="flex-1" />

      <PriorityPicker keys={m.priorityKeys} resumeMs={m.priorityResumeMs} onChange={p.onChange} onAdd={p.onAddPriority} />
      <ProcessPicker value={m.process} onChange={p.onProcess} />

      <span className="group flex items-center rounded-md border border-line bg-panel">
        <button
          onClick={p.onSetTrigger}
          title="Set trigger: press a key combo or mouse button"
          className={`px-2.5 py-1 font-mono text-xs hover:text-fg ${m.trigger ? "text-accent" : "text-mute"}`}
        >
          {m.trigger ? triggerLabel(m.trigger) : "Set trigger"}
        </button>
        {m.trigger && (
          <button title="Clear trigger" onClick={p.onClearTrigger} className="pr-1.5 text-mute opacity-0 hover:text-fg group-hover:opacity-100">
            <X size={12} />
          </button>
        )}
      </span>

      <select
        value={m.playMode}
        title={MODE_HELP[m.playMode]}
        onChange={(e) => p.onChange({ playMode: e.target.value as PlayMode })}
        className="rounded-md border border-line bg-panel px-2 py-1 text-xs"
      >
        <option value="once">Play once</option>
        <option value="count">Play N times</option>
        <option value="toggle">Toggle</option>
        <option value="whileHeld">While held</option>
      </select>
      {m.playMode === "count" && (
        <input
          type="number"
          min={1}
          value={m.repeatCount}
          onChange={(e) => p.onChange({ repeatCount: Math.max(1, parseInt(e.target.value, 10) || 1) })}
          className="w-14 rounded-md border border-line bg-panel px-2 py-1 text-right font-mono text-xs"
        />
      )}

      {playing ? (
        <button title="Stop playback" onClick={p.onStop} className="rounded-md p-1.5 text-rec hover:bg-raise">
          <Square size={17} />
        </button>
      ) : (
        <button title="Test play" disabled={m.events.length === 0} onClick={p.onPlay} className="rounded-md p-1.5 text-accent hover:bg-raise disabled:opacity-30">
          <Play size={17} />
        </button>
      )}
    </div>
  );
}
