import { useMemo, useRef, useState } from "react";
import { ChevronDown, ChevronRight, Folder, FolderPlus, Plus, Power, Search, X } from "lucide-react";
import type { Macro } from "../lib/types";
import { shortTrigger } from "../lib/fmt";

interface Props {
  macros: Macro[];
  folders: string[];
  selectedId: string | null;
  playing: Set<string>;
  renamingId: string | null;
  renamingFolder: string | null;
  onSelect: (id: string) => void;
  onCreate: () => void;
  onCreateFolder: () => void;
  onToggle: (id: string, enabled: boolean) => void;
  onStartRename: (id: string) => void;
  onRename: (id: string, name: string) => void;
  onStartFolderRename: (name: string) => void;
  onFolderRename: (oldName: string, name: string | null) => void;
  onContext: (e: React.MouseEvent, id: string | null) => void;
  onFolderContext: (e: React.MouseEvent, name: string) => void;
  onMove: (id: string, folder: string | null, beforeId: string | null) => void;
  onReorderFolders: (from: string, to: string) => void;
}

type Drag = { kind: "macro" | "folder"; id: string } | null;

function NameInput({ value, onDone }: { value: string; onDone: (v: string | null) => void }) {
  const done = useRef(false);
  const finish = (v: string | null) => {
    if (done.current) return;
    done.current = true;
    onDone(v);
  };
  return (
    <input
      autoFocus
      type="text"
      defaultValue={value}
      onClick={(e) => e.stopPropagation()}
      onBlur={(e) => finish(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        if (e.key === "Escape") finish(null);
      }}
      className="min-w-0 flex-1 rounded bg-base px-1.5 py-0.5 text-sm ring-1 ring-accent outline-none"
    />
  );
}

export default function Sidebar(p: Props) {
  const [query, setQuery] = useState("");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [drag, setDrag] = useState<Drag>(null);
  const [over, setOver] = useState<string | null>(null);

  const allFolders = useMemo(() => {
    const extra = [...new Set(p.macros.map((m) => m.folder).filter((f): f is string => !!f))].filter((f) => !p.folders.includes(f));
    return [...p.folders, ...extra];
  }, [p.folders, p.macros]);

  const q = query.trim().toLowerCase();
  const endDrag = () => { setDrag(null); setOver(null); };

  const macroRow = (m: Macro, indented: boolean) => {
    const active = m.id === p.selectedId;
    return (
      <li
        key={m.id}
        draggable={p.renamingId !== m.id}
        onContextMenu={(e) => p.onContext(e, m.id)}
        onDragStart={(e) => {
          e.stopPropagation();
          setDrag({ kind: "macro", id: m.id });
          e.dataTransfer.effectAllowed = "move";
          e.dataTransfer.setData("text/plain", m.id);
        }}
        onDragOver={(e) => {
          if (drag?.kind !== "macro") return;
          e.preventDefault();
          e.stopPropagation();
          setOver("m:" + m.id);
        }}
        onDrop={(e) => {
          if (drag?.kind !== "macro") return;
          e.preventDefault();
          e.stopPropagation();
          if (drag.id !== m.id) p.onMove(drag.id, m.folder ?? null, m.id);
          endDrag();
        }}
        onDragEnd={endDrag}
        className={`border-t-2 ${over === "m:" + m.id && drag?.id !== m.id ? "border-accent" : "border-transparent"} ${drag?.id === m.id ? "opacity-40" : ""}`}
      >
        <div
          onClick={() => p.onSelect(m.id)}
          onDoubleClick={() => p.onStartRename(m.id)}
          className={`group flex cursor-default items-center gap-2 rounded-md py-1.5 pr-2.5 ${indented ? "pl-6" : "pl-2.5"} ${
            active ? "bg-raise shadow-[inset_2px_0_0_var(--accent)]" : "hover:bg-raise/60"
          }`}
        >
          {p.playing.has(m.id) && <span className="h-1.5 w-1.5 shrink-0 animate-pulse rounded-full bg-accent" />}
          {p.renamingId === m.id ? (
            <NameInput value={m.name} onDone={(v) => p.onRename(m.id, v?.trim() || m.name)} />
          ) : (
            <span className={`min-w-0 flex-1 truncate ${m.enabled ? "" : "text-mute"}`} title={m.folder ? `${m.folder} / ${m.name}` : m.name}>
              {m.name}
            </span>
          )}
          {m.process && <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-mute" title={`Only in ${m.process}`} />}
          <button
            title={m.enabled ? "Disable" : "Enable"}
            onClick={(e) => {
              e.stopPropagation();
              p.onToggle(m.id, !m.enabled);
            }}
            className={`rounded p-0.5 ${m.enabled ? "text-mute opacity-0 group-hover:opacity-100" : "text-rec"} hover:text-fg`}
          >
            <Power size={13} />
          </button>
          <span className="font-mono text-[11px] text-mute">{shortTrigger(m.trigger)}</span>
        </div>
      </li>
    );
  };

  const inFolder = (f: string) => p.macros.filter((m) => m.folder === f);
  const loose = p.macros.filter((m) => !m.folder || !allFolders.includes(m.folder));
  const results = p.macros.filter((m) => m.name.toLowerCase().includes(q) || (m.folder ?? "").toLowerCase().includes(q));

  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-line bg-panel">
      <div className="flex items-center justify-between px-4 pb-1.5 pt-3">
        <span className="text-xs text-mute">Macros</span>
        <span className="flex items-center">
          <button title="New folder" onClick={p.onCreateFolder} className="rounded p-1 text-mute hover:bg-raise hover:text-fg">
            <FolderPlus size={15} />
          </button>
          <button title="New macro" onClick={() => p.onCreate()} className="rounded p-1 text-mute hover:bg-raise hover:text-fg">
            <Plus size={15} />
          </button>
        </span>
      </div>

      <div className="relative mx-3 mb-2">
        <Search size={13} className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-mute" />
        <input
          type="text"
          value={query}
          placeholder="Search"
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Escape" && setQuery("")}
          className="w-full rounded-md border border-line bg-base py-1 pl-7 pr-6 text-xs outline-none placeholder:text-mute focus:border-accent"
        />
        {query && (
          <button onClick={() => setQuery("")} className="absolute right-1.5 top-1/2 -translate-y-1/2 text-mute hover:text-fg">
            <X size={12} />
          </button>
        )}
      </div>

      <div
        className="flex min-h-0 flex-1 flex-col overflow-y-auto px-2 pb-2"
        onContextMenu={(e) => p.onContext(e, null)}
        onDragOver={(e) => {
          if (drag?.kind === "macro") {
            e.preventDefault();
            setOver("root");
          }
        }}
        onDrop={(e) => {
          if (drag?.kind === "macro") {
            e.preventDefault();
            p.onMove(drag.id, null, null);
          }
          endDrag();
        }}
      >
        {q ? (
          <ul>
            {results.map((m) => macroRow(m, false))}
            {results.length === 0 && <li className="px-2.5 py-3 text-xs text-mute">No macros match.</li>}
          </ul>
        ) : (
          <>
            {allFolders.map((f) => {
              const items = inFolder(f);
              const open = !collapsed.has(f);
              const highlight = over === "f:" + f && drag && !(drag.kind === "folder" && drag.id === f);
              return (
                <div key={f}>
                  <div
                    draggable={p.renamingFolder !== f}
                    onClick={() => setCollapsed((c) => { const n = new Set(c); if (n.has(f)) n.delete(f); else n.add(f); return n; })}
                    onDoubleClick={() => p.onStartFolderRename(f)}
                    onContextMenu={(e) => p.onFolderContext(e, f)}
                    onDragStart={(e) => {
                      setDrag({ kind: "folder", id: f });
                      e.dataTransfer.effectAllowed = "move";
                      e.dataTransfer.setData("text/plain", f);
                    }}
                    onDragOver={(e) => {
                      if (!drag) return;
                      e.preventDefault();
                      e.stopPropagation();
                      setOver("f:" + f);
                    }}
                    onDrop={(e) => {
                      if (!drag) return;
                      e.preventDefault();
                      e.stopPropagation();
                      if (drag.kind === "macro") p.onMove(drag.id, f, null);
                      else if (drag.id !== f) p.onReorderFolders(drag.id, f);
                      endDrag();
                    }}
                    onDragEnd={endDrag}
                    className={`group mt-1 flex cursor-default items-center gap-1.5 rounded-md px-1.5 py-1 text-mute hover:bg-raise/60 hover:text-fg ${
                      highlight ? "bg-raise ring-1 ring-accent/60" : ""
                    } ${drag?.kind === "folder" && drag.id === f ? "opacity-40" : ""}`}
                  >
                    {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                    <Folder size={14} />
                    {p.renamingFolder === f ? (
                      <NameInput value={f} onDone={(v) => p.onFolderRename(f, v)} />
                    ) : (
                      <span className="min-w-0 flex-1 truncate text-sm">{f}</span>
                    )}
                    <span className="text-[11px]">{items.length}</span>
                  </div>
                  {open && <ul>{items.map((m) => macroRow(m, true))}</ul>}
                </div>
              );
            })}
            <ul className={allFolders.length ? "mt-1" : ""}>{loose.map((m) => macroRow(m, false))}</ul>
            {p.macros.length === 0 && allFolders.length === 0 && (
              <p className="px-2.5 py-3 text-xs leading-relaxed text-mute">Press + to create a macro. Right-click for more.</p>
            )}
            <div className={`min-h-10 flex-1 rounded-md ${over === "root" ? "bg-raise/50" : ""}`} />
          </>
        )}
      </div>
    </aside>
  );
}
