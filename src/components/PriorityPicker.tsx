import { Pause, Plus, X } from "lucide-react";
import type { Macro } from "../lib/types";
import { prettyKey } from "../lib/fmt";
import Popover from "./Popover";

interface Props {
  keys: string[];
  resumeMs: number;
  onChange: (patch: Partial<Macro>) => void;
  onAdd: () => void;
}

export default function PriorityPicker({ keys, resumeMs, onChange, onAdd }: Props) {
  const label = keys.length === 0 ? "Priority keys" : keys.length === 1 ? prettyKey(keys[0]) : `${keys.length} keys`;
  return (
    <Popover
      width={300}
      trigger={(_open, toggle) => (
        <button
          onClick={toggle}
          title="Keys that pause this macro while you hold them"
          className={`flex items-center gap-1.5 rounded-md border border-line bg-panel px-2.5 py-1 text-xs hover:text-fg ${
            keys.length ? "text-accent" : "text-mute"
          }`}
        >
          <Pause size={13} />
          <span className="max-w-24 truncate">{label}</span>
        </button>
      )}
    >
      {() => (
        <div>
          <p className="mb-2 text-xs leading-relaxed text-mute">
            Hold one of these and this macro pauses so you can do something else. It picks up again shortly after you let go.
          </p>
          <div className="mb-2 flex flex-wrap gap-1.5">
            {keys.length === 0 && <span className="text-xs text-mute">None yet.</span>}
            {keys.map((k) => (
              <span key={k} className="flex items-center gap-1 rounded border border-line bg-base px-2 py-0.5 font-mono text-xs">
                {prettyKey(k)}
                <button
                  title="Remove"
                  onClick={() => onChange({ priorityKeys: keys.filter((x) => x !== k) })}
                  className="text-mute hover:text-rec"
                >
                  <X size={11} />
                </button>
              </span>
            ))}
          </div>
          <button
            onClick={onAdd}
            className="flex items-center gap-1.5 rounded-md border border-line px-2.5 py-1 text-xs hover:border-accent"
          >
            <Plus size={12} /> Add a key or mouse button
          </button>
          <label className="mt-3 flex items-center justify-between gap-3 text-xs text-mute">
            Resume after
            <span className="flex items-center gap-1.5">
              <input
                type="number"
                min={0}
                value={resumeMs}
                onChange={(e) => onChange({ priorityResumeMs: Math.max(0, parseInt(e.target.value, 10) || 0) })}
                className="w-20 rounded-md border border-line bg-base px-2 py-1 text-right font-mono text-xs text-fg outline-none focus:border-accent"
              />
              ms
            </span>
          </label>
        </div>
      )}
    </Popover>
  );
}
