import { useState } from "react";

interface Props {
  onSend: (command: string, seconds?: number) => Promise<void>;
  busy: boolean;
  online: boolean;
  overrideActive: boolean;
  diagnosticActive: boolean;
}

/**
 * Compact, secondary control panel.
 *
 * The decision card is the hero on the page; these are the bounded-window
 * tools an engineer reaches for *after* deciding to act. None of them mutate
 * production — they only change what Aegis reports to Splunk.
 */
export function ToolsPanel({
  onSend,
  busy,
  online,
  overrideActive,
  diagnosticActive,
}: Props) {
  const [overrideSecs, setOverrideSecs] = useState(30);
  const [diagSecs, setDiagSecs] = useState(60);

  return (
    <section className="px-6 pb-6">
      <details className="rounded-lg border border-slate-800/80 bg-slate-900/40">
        <summary className="cursor-pointer px-5 py-3 text-[11px] uppercase tracking-widest text-slate-400 hover:text-slate-200">
          Advanced tools (bounded-window, never mutate production)
        </summary>
        <div className="space-y-4 border-t border-slate-800/80 p-5 text-sm">
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-slate-400">Network status:</span>
            <button
              type="button"
              disabled={busy || online}
              onClick={() => onSend("online")}
              className={`px-3 py-1 text-xs rounded ${
                online
                  ? "bg-emerald-500/20 text-emerald-300 ring-1 ring-emerald-500/30"
                  : "border border-slate-700 text-slate-300 hover:bg-slate-800"
              }`}
            >
              Online
            </button>
            <button
              type="button"
              disabled={busy || !online}
              onClick={() => onSend("offline")}
              className={`px-3 py-1 text-xs rounded ${
                !online
                  ? "bg-amber-500/20 text-amber-300 ring-1 ring-amber-500/30"
                  : "border border-slate-700 text-slate-300 hover:bg-slate-800"
              }`}
            >
              Offline
            </button>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <span className="text-slate-400">Diagnostic tracing:</span>
            <input
              type="number"
              min={1}
              max={3600}
              value={diagSecs}
              onChange={(e) => setDiagSecs(Number(e.target.value) || 60)}
              className="w-20 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100"
            />
            <span className="text-xs text-slate-500">seconds</span>
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("diagnostic", diagSecs)}
              className={`px-3 py-1 text-xs rounded ${
                diagnosticActive
                  ? "bg-emerald-500/20 text-emerald-300 ring-1 ring-emerald-500/30"
                  : "border border-slate-700 text-slate-300 hover:bg-slate-800"
              }`}
            >
              {diagnosticActive ? "Active" : "Enable"}
            </button>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <span className="text-slate-400">Raw passthrough (bypass dedup):</span>
            <input
              type="number"
              min={1}
              max={3600}
              value={overrideSecs}
              onChange={(e) => setOverrideSecs(Number(e.target.value) || 30)}
              className="w-20 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100"
            />
            <span className="text-xs text-slate-500">seconds</span>
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("override", overrideSecs)}
              className={`px-3 py-1 text-xs rounded ${
                overrideActive
                  ? "bg-amber-500/20 text-amber-300 ring-1 ring-amber-500/30"
                  : "border border-slate-700 text-slate-300 hover:bg-slate-800"
              }`}
            >
              {overrideActive ? "Active" : "Enable"}
            </button>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <span className="text-slate-400">Reset:</span>
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("reset")}
              className="rounded border border-rose-700/40 px-3 py-1 text-xs text-rose-200 hover:bg-rose-500/10"
            >
              Clear queue + counters
            </button>
          </div>
        </div>
      </details>
    </section>
  );
}
