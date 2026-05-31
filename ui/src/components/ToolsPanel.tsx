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
    <section className="px-4 pb-4">
      <details className="console-card">
        <summary className="cursor-pointer text-[10px] font-black uppercase tracking-[1.4px] text-[#6A6245] transition hover:text-[#3D3520]">
          Advanced Tools (bounded-window, never mutate production)
        </summary>
        <div className="mt-4 space-y-4 border-t border-[rgba(61,53,32,0.1)] pt-4 text-sm">
          {/* Network status */}
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-xs text-[#6A6245]">Network status:</span>
            <button
              type="button"
              disabled={busy || online}
              onClick={() => onSend("online")}
              className="btn-tertiary"
              style={online ? { borderColor: "rgba(34,196,90,0.4)", color: "#22C45A" } : {}}
            >
              {online && <span className="led led-green mr-1.5" style={{ width: 5, height: 5 }} />}
              Online
            </button>
            <button
              type="button"
              disabled={busy || !online}
              onClick={() => onSend("offline")}
              className="btn-tertiary"
              style={!online ? { borderColor: "rgba(212,192,32,0.4)", color: "#D4C020" } : {}}
            >
              {!online && <span className="led led-amber mr-1.5" style={{ width: 5, height: 5 }} />}
              Offline
            </button>
          </div>

          {/* Diagnostic tracing */}
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-xs text-[#6A6245]">Diagnostic tracing:</span>
            <input
              type="number"
              min={1}
              max={3600}
              value={diagSecs}
              onChange={(e) => setDiagSecs(Number(e.target.value) || 60)}
              className="input-recessed"
              style={{ width: 80, padding: "6px 10px", fontSize: 12 }}
            />
            <span className="text-[10px] text-[#b8b098]">seconds</span>
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("diagnostic", diagSecs)}
              className="btn-tertiary"
              style={diagnosticActive ? { borderColor: "rgba(34,196,90,0.4)", color: "#22C45A" } : {}}
            >
              {diagnosticActive && <span className="led led-green mr-1.5" style={{ width: 5, height: 5 }} />}
              {diagnosticActive ? "Active" : "Enable"}
            </button>
          </div>

          {/* Raw passthrough */}
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-xs text-[#6A6245]">Raw passthrough (bypass dedup):</span>
            <input
              type="number"
              min={1}
              max={3600}
              value={overrideSecs}
              onChange={(e) => setOverrideSecs(Number(e.target.value) || 30)}
              className="input-recessed"
              style={{ width: 80, padding: "6px 10px", fontSize: 12 }}
            />
            <span className="text-[10px] text-[#b8b098]">seconds</span>
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("override", overrideSecs)}
              className="btn-tertiary"
              style={overrideActive ? { borderColor: "rgba(232,124,20,0.4)", color: "#E87C14" } : {}}
            >
              {overrideActive && <span className="led led-amber mr-1.5" style={{ width: 5, height: 5 }} />}
              {overrideActive ? "Active" : "Enable"}
            </button>
          </div>

          {/* Reset */}
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-xs text-[#6A6245]">Reset:</span>
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("reset")}
              className="btn-tertiary"
              style={{ color: "#D43020", borderColor: "rgba(212,48,32,0.3)" }}
            >
              Clear queue + counters
            </button>
          </div>
        </div>
      </details>
    </section>
  );
}
