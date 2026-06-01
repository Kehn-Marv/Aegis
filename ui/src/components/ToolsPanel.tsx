import { useState } from "react";

interface Props {
  onSend: (command: string, seconds?: number) => Promise<void>;
  busy: boolean;
  online: boolean;
  overrideActive: boolean;
  diagnosticActive: boolean;
}

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
    <section>
      <details className="console-card">
        <summary className="flex cursor-pointer items-center justify-between text-[10px] font-bold uppercase tracking-[1.4px] text-[#8a8470] transition hover:text-[#2e2a1e]">
          <span>Advanced Tools</span>
          <span className="disclosure-arrow" aria-hidden="true">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <polyline points="6 9 12 15 18 9" />
            </svg>
          </span>
        </summary>
        <div className="mt-4 space-y-4 border-t border-[rgba(0,0,0,0.06)] pt-4 text-sm">
          {/* Network */}
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-xs font-medium text-[#5a5440]">Network:</span>
            <button
              type="button"
              disabled={busy || online}
              onClick={() => onSend("online")}
              className="btn-tertiary"
              style={online ? { borderColor: "rgba(34,160,80,0.25)" } : {}}
            >
              Online
            </button>
            <button
              type="button"
              disabled={busy || !online}
              onClick={() => onSend("offline")}
              className="btn-tertiary"
              style={!online ? { borderColor: "rgba(200,168,32,0.25)" } : {}}
            >
              Offline
            </button>
          </div>

          {/* Diagnostic */}
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-xs font-medium text-[#5a5440]">Diagnostic:</span>
            <input
              type="number"
              min={1} max={3600}
              value={diagSecs}
              onChange={(e) => setDiagSecs(Number(e.target.value) || 60)}
              className="btn-tertiary text-center font-mono !text-[12px] !font-semibold !text-[#2e2a1e] !pr-5"
              style={{ width: 76 }}
            />
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("diagnostic", diagSecs)}
              className="btn-tertiary"
              style={diagnosticActive ? { borderColor: "rgba(34,160,80,0.25)" } : {}}
            >
              {diagnosticActive ? "Active" : "Enable"}
            </button>
          </div>

          {/* Override */}
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-xs font-medium text-[#5a5440]">Raw passthrough:</span>
            <input
              type="number"
              min={1} max={3600}
              value={overrideSecs}
              onChange={(e) => setOverrideSecs(Number(e.target.value) || 30)}
              className="btn-tertiary text-center font-mono !text-[12px] !font-semibold !text-[#2e2a1e] !pr-5"
              style={{ width: 76 }}
            />
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("override", overrideSecs)}
              className="btn-tertiary"
              style={overrideActive ? { borderColor: "rgba(224,120,24,0.25)" } : {}}
            >
              {overrideActive ? "Active" : "Enable"}
            </button>
          </div>

          {/* Reset */}
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-xs font-medium text-[#5a5440]">Reset:</span>
            <button
              type="button"
              disabled={busy}
              onClick={() => onSend("reset")}
              className="btn-tertiary"
              style={{ color: "#c83020", borderColor: "rgba(200,48,32,0.2)" }}
            >
              Clear queue + counters
            </button>
          </div>
        </div>
      </details>
    </section>
  );
}
