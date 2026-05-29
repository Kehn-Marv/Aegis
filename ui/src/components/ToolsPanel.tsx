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

  const activePill = "ring-1";
  const idleBtn =
    "rounded-lg bg-white px-3 py-1 text-xs text-neutral-700 ring-1 ring-neutral-300 transition hover:bg-neutral-50 disabled:cursor-not-allowed disabled:opacity-50";
  const numInput =
    "w-20 rounded-lg border border-neutral-300 bg-white px-2 py-1 text-xs text-neutral-900 outline-none focus:border-[#0071e3] focus:ring-2 focus:ring-[#0071e3]/20";

  return (
    <section className="px-6 pb-6">
      <details className="rounded-2xl bg-white shadow-card ring-1 ring-neutral-200/60">
        <summary className="cursor-pointer px-5 py-3.5 text-[11px] font-medium uppercase tracking-wider text-neutral-400 transition hover:text-neutral-700">
          Advanced tools (bounded-window, never mutate production)
        </summary>
        <div className="space-y-4 border-t border-neutral-200 p-5 text-sm">
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-neutral-500">Network status:</span>
            <button
              type="button"
              disabled={busy || online}
              onClick={() => onSend("online")}
              className={
                online
                  ? `rounded-lg bg-emerald-50 px-3 py-1 text-xs font-medium text-emerald-700 ${activePill} ring-emerald-600/20`
                  : idleBtn
              }
            >
              Online
            </button>
            <button
              type="button"
              disabled={busy || !online}
              onClick={() => onSend("offline")}
              className={
                !online
                  ? `rounded-lg bg-amber-50 px-3 py-1 text-xs font-medium text-amber-700 ${activePill} ring-amber-600/20`
                  : idleBtn
              }
            >
              Offline
            </button>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <span className="text-neutral-500">Diagnostic tracing:</span>
            <input
              type="number"
              min={1}
              max={3600}
              value={diagSecs}
              onChange={(e) => setDiagSecs(Number(e.target.value) || 60)}
              className={numInput}
            />
            <span className="text-xs text-neutral-400">seconds</span>
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("diagnostic", diagSecs)}
              className={
                diagnosticActive
                  ? `rounded-lg bg-emerald-50 px-3 py-1 text-xs font-medium text-emerald-700 ${activePill} ring-emerald-600/20`
                  : idleBtn
              }
            >
              {diagnosticActive ? "Active" : "Enable"}
            </button>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <span className="text-neutral-500">Raw passthrough (bypass dedup):</span>
            <input
              type="number"
              min={1}
              max={3600}
              value={overrideSecs}
              onChange={(e) => setOverrideSecs(Number(e.target.value) || 30)}
              className={numInput}
            />
            <span className="text-xs text-neutral-400">seconds</span>
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("override", overrideSecs)}
              className={
                overrideActive
                  ? `rounded-lg bg-amber-50 px-3 py-1 text-xs font-medium text-amber-700 ${activePill} ring-amber-600/20`
                  : idleBtn
              }
            >
              {overrideActive ? "Active" : "Enable"}
            </button>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <span className="text-neutral-500">Reset:</span>
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("reset")}
              className="rounded-lg bg-white px-3 py-1 text-xs text-rose-600 ring-1 ring-rose-300 transition hover:bg-rose-50"
            >
              Clear queue + counters
            </button>
          </div>
        </div>
      </details>
    </section>
  );
}
