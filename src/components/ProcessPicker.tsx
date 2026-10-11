import { useEffect, useState } from "react";
import { AppWindow, Check } from "lucide-react";
import { api } from "../lib/api";
import type { ProcInfo } from "../lib/types";
import Popover from "./Popover";

function List({ value, onPick }: { value: string | null; onPick: (p: string | null) => void }) {
  const [procs, setProcs] = useState<ProcInfo[] | null>(null);
  const [custom, setCustom] = useState("");
  useEffect(() => { api.listProcesses().then(setProcs).catch(() => setProcs([])); }, []);

  const row = (key: string, label: string, sub: string, active: boolean, pick: () => void) => (
    <button key={key} onClick={pick} className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-raise">
      <span className="w-4 shrink-0">{active && <Check size={14} className="text-accent" />}</span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm">{label}</span>
        {sub && <span className="block truncate text-xs text-mute">{sub}</span>}
      </span>
    </button>
  );

  const missing = value && procs && !procs.some((p) => p.exe.toLowerCase() === value.toLowerCase());
  return (
    <div className="-m-1">
      {row("all", "All apps", "Runs everywhere", value === null, () => onPick(null))}
      <div className="px-2 pb-1 pt-2 text-xs text-mute">Open apps</div>
      {procs === null && <div className="px-2 py-1 text-xs text-mute">Looking...</div>}
      {missing && row("cur", value!, "Not running right now", true, () => onPick(value))}
      <div className="max-h-56 overflow-y-auto">
        {procs?.map((p) => row(p.exe, p.exe, p.title, value?.toLowerCase() === p.exe.toLowerCase(), () => onPick(p.exe)))}
      </div>
      <div className="mt-2 border-t border-line px-1 pt-2">
        <input
          type="text"
          value={custom}
          placeholder="Or type a name, e.g. game.exe"
          onChange={(e) => setCustom(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && custom.trim()) onPick(custom.trim());
          }}
          className="w-full rounded-md border border-line bg-base px-2 py-1 font-mono text-xs outline-none focus:border-accent"
        />
      </div>
    </div>
  );
}

export default function ProcessPicker({ value, onChange }: { value: string | null; onChange: (p: string | null) => void }) {
  return (
    <Popover
      width={290}
      trigger={(_open, toggle) => (
        <button
          onClick={toggle}
          title="Only run this macro while this app is in the foreground"
          className={`flex items-center gap-1.5 rounded-md border border-line bg-panel px-2.5 py-1 text-xs hover:text-fg ${value ? "text-accent" : "text-mute"}`}
        >
          <AppWindow size={13} />
          <span className="max-w-32 truncate">{value ?? "All apps"}</span>
        </button>
      )}
    >
      {(close) => (
        <List
          value={value}
          onPick={(p) => {
            onChange(p);
            close();
          }}
        />
      )}
    </Popover>
  );
}
