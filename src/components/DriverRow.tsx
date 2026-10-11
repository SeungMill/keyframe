import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { DriverStatus } from "../lib/types";

export default function DriverRow() {
  const [st, setSt] = useState<DriverStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  const refresh = () => api.driverStatus().then(setSt).catch(() => {});
  useEffect(() => { refresh(); }, []);

  async function run(uninstall: boolean) {
    setBusy(true);
    setNote(null);
    try {
      setNote(await api.driverInstall(uninstall));
    } catch (e) {
      setNote(String(e));
    } finally {
      setBusy(false);
      refresh();
    }
  }

  return (
    <div className="mt-2 rounded-md border border-line bg-base p-2.5 text-xs">
      <div className="flex items-center gap-2">
        <span className={`h-2 w-2 shrink-0 rounded-full ${st?.active ? "bg-accent" : "bg-mute"}`} />
        <span>{st ? st.message : "Checking driver..."}</span>
      </div>
      <div className="mt-2 flex gap-2">
        <button
          disabled={busy || !st?.installerFound}
          onClick={() => run(false)}
          className="rounded-md bg-accent px-2.5 py-1 font-medium text-accent-fg hover:brightness-110 disabled:opacity-40"
        >
          {busy ? "Working..." : st?.active ? "Reinstall" : "Install driver"}
        </button>
        {st?.active && (
          <button disabled={busy} onClick={() => run(true)} className="rounded-md border border-line px-2.5 py-1 text-mute hover:text-fg disabled:opacity-40">
            Uninstall
          </button>
        )}
      </div>
      {note && <p className="mt-2 leading-relaxed text-mute">{note}</p>}
      <p className="mt-2 leading-relaxed text-mute">Interception is bundled for personal, non-commercial use only (LGPL 3.0).</p>
    </div>
  );
}
