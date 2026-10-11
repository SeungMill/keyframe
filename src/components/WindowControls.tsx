import { Minus, Square, X } from "lucide-react";
import { api } from "../lib/api";

const base = "grid h-7 w-8 place-items-center rounded-md text-mute hover:bg-raise hover:text-fg";

export default function WindowControls() {
  return (
    <div className="ml-2 flex items-center gap-0.5 border-l border-line pl-2">
      <button title="Minimize" className={base} onClick={() => api.windowMinimize()}>
        <Minus size={15} />
      </button>
      <button title="Maximize" className={base} onClick={() => api.windowToggleMaximize()}>
        <Square size={11} />
      </button>
      <button title="Close" className={`${base} hover:!bg-rec hover:!text-rec-fg`} onClick={() => api.windowClose()}>
        <X size={15} />
      </button>
    </div>
  );
}
