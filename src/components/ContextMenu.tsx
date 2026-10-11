import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { ChevronRight } from "lucide-react";

export interface MenuItem {
  label?: string;
  hint?: string;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  onSelect?: () => void;
  children?: MenuItem[];
}

function Submenu({ items, onClose }: { items: MenuItem[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: false, dy: 0, ready: false });

  useLayoutEffect(() => {
    const r = ref.current?.getBoundingClientRect();
    if (!r) return;
    setPos({
      left: r.right > window.innerWidth - 6,
      dy: Math.min(0, window.innerHeight - 6 - r.bottom),
      ready: true,
    });
  }, []);

  return (
    <div
      ref={ref}
      style={{ top: pos.dy, visibility: pos.ready ? "visible" : "hidden" }}
      className={`absolute z-10 ${pos.left ? "right-full pr-1" : "left-full pl-1"}`}
    >
      <Items items={items} onClose={onClose} />
    </div>
  );
}

function Items({ items, onClose }: { items: MenuItem[]; onClose: () => void }) {
  const [open, setOpen] = useState<number | null>(null);
  return (
    <div className="min-w-44 rounded-lg border border-line bg-panel p-1 text-sm shadow-2xl">
      {items.map((it, i) =>
        it.separator ? (
          <div key={i} className="my-1 h-px bg-line" />
        ) : (
          <div key={i} className="relative" onMouseEnter={() => setOpen(it.children ? i : null)}>
            <button
              disabled={it.disabled}
              onClick={() => {
                if (it.children || it.disabled) return;
                it.onSelect?.();
                onClose();
              }}
              className={`flex w-full items-center justify-between gap-6 rounded-md px-2.5 py-1.5 text-left hover:bg-raise disabled:opacity-35 disabled:hover:bg-transparent ${
                it.danger ? "text-rec" : "text-fg"
              }`}
            >
              <span>{it.label}</span>
              {it.children ? (
                <ChevronRight size={14} className="text-mute" />
              ) : it.hint ? (
                <span className="text-xs text-mute">{it.hint}</span>
              ) : null}
            </button>
            {it.children && open === i && <Submenu items={it.children} onClose={onClose} />}
          </div>
        ),
      )}
    </div>
  );
}

export default function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: MenuItem[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });

  useLayoutEffect(() => {
    const r = ref.current?.getBoundingClientRect();
    if (!r) return;
    setPos({
      left: Math.max(4, Math.min(x, window.innerWidth - r.width - 4)),
      top: Math.max(4, Math.min(y, window.innerHeight - r.height - 4)),
    });
  }, [x, y]);

  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", key);
    window.addEventListener("blur", onClose);
    window.addEventListener("resize", onClose);
    return () => {
      window.removeEventListener("keydown", key);
      window.removeEventListener("blur", onClose);
      window.removeEventListener("resize", onClose);
    };
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-[70]"
      onMouseDown={onClose}
      onContextMenu={(e) => {
        e.preventDefault();
        onClose();
      }}
    >
      <div
        ref={ref}
        style={pos}
        className="fixed"
        onMouseDown={(e) => e.stopPropagation()}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
        }}
      >
        <Items items={items} onClose={onClose} />
      </div>
    </div>
  );
}
