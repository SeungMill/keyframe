import { useState, type ReactNode } from "react";

interface Props {
  trigger: (open: boolean, toggle: () => void) => ReactNode;
  children: (close: () => void) => ReactNode;
  width?: number;
}

export default function Popover({ trigger, children, width = 280 }: Props) {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative">
      {trigger(open, () => setOpen((o) => !o))}
      {open && (
        <>
          <div className="fixed inset-0 z-40" onMouseDown={() => setOpen(false)} />
          <div
            style={{ width }}
            className="absolute right-0 top-full z-50 mt-2 max-h-[calc(100vh-5rem)] overflow-y-auto rounded-xl border border-line bg-panel p-3 shadow-2xl"
          >
            {children(() => setOpen(false))}
          </div>
        </>
      )}
    </div>
  );
}
