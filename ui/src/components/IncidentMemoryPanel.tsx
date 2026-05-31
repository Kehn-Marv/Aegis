import { useState } from "react";
import type { Fingerprint } from "../api";

interface Props {
  incidents: Fingerprint[];
  onResolve: (id: string, cause: string, fix: string) => Promise<void>;
  busy: boolean;
}

function formatTime(unixSecs: number): string {
  return new Date(unixSecs * 1000).toLocaleString();
}

export function IncidentMemoryPanel({ incidents, onResolve, busy }: Props) {
  const [openId, setOpenId] = useState<string | null>(null);
  const [cause, setCause] = useState("");
  const [fix, setFix] = useState("");

  const handleSubmit = async (id: string) => {
    if (!cause.trim() || !fix.trim()) return;
    await onResolve(id, cause.trim(), fix.trim());
    setCause("");
    setFix("");
    setOpenId(null);
  };

  return (
    <section className="px-4 pb-4">
      <div className="console-card">
        <div className="flex items-center justify-between">
          <div>
            <div className="eyebrow">Incident Memory</div>
            <div className="mt-1 text-xs text-[#6A6245]">
              Every chain Aegis has seen. Resolve one — even with two short
              sentences — and the next on-call gets a head start.
            </div>
          </div>
          <div className="font-mono text-[10px] text-[#6A6245]">
            {incidents.length} fingerprint{incidents.length === 1 ? "" : "s"}
          </div>
        </div>

        <div className="mt-4 space-y-3">
          {incidents.length === 0 && (
            <div className="lcd-panel text-sm text-[rgba(255,250,218,0.6)]">
              No incidents fingerprinted yet. The workload app injects them on
              its own — or plant one by hand:
              <pre className="mt-2 overflow-x-auto rounded px-3 py-2 font-mono text-[10px] text-[#E87C14]"
                style={{ backgroundColor: "rgba(0,0,0,0.3)" }}
              >
{`python demo/log_spammer.py --target tcp://127.0.0.1:5140 --pattern cascade`}
              </pre>
            </div>
          )}

          {incidents.map((inc) => {
            const open = openId === inc.id;
            const resolved = inc.cause && inc.fix;
            return (
              <div
                key={inc.id}
                className="module-card"
                style={
                  resolved
                    ? { borderColor: "rgba(34,196,90,0.3)" }
                    : { borderColor: "rgba(232,124,20,0.3)" }
                }
              >
                <div className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2 text-xs">
                      <span
                        className={resolved ? "led led-green" : "led led-amber"}
                        style={{ width: 6, height: 6 }}
                      />
                      <span className="font-mono text-[#6A6245]">{inc.id}</span>
                      <span className="text-[#b8b098]">·</span>
                      <span className="text-[#6A6245]">
                        root cause:{" "}
                        <span className="font-bold text-[#E87C14]">
                          {inc.root_cause_service}
                        </span>
                      </span>
                      <span className="text-[#b8b098]">·</span>
                      <span className="text-[#b8b098]">{formatTime(inc.ts)}</span>
                      {resolved && (
                        <span className="badge badge-green ml-1" style={{ fontSize: "8px", padding: "2px 8px" }}>
                          Resolved
                        </span>
                      )}
                    </div>
                    <div className="mt-1 truncate text-xs text-[#6A6245]">
                      chain: {inc.services.join(" → ")}
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      setOpenId(open ? null : inc.id);
                      setCause(inc.cause ?? "");
                      setFix(inc.fix ?? "");
                    }}
                    className="btn-tertiary shrink-0"
                  >
                    {open ? "Hide" : resolved ? "View" : "Resolve"}
                  </button>
                </div>

                {open && (
                  <div className="mt-3 space-y-2">
                    {resolved && (
                      <>
                        <div className="text-xs text-[#3D3520]">
                          <span className="text-[#6A6245]">cause: </span>
                          {inc.cause}
                        </div>
                        <div className="text-xs text-[#3D3520]">
                          <span className="text-[#6A6245]">fix: </span>
                          {inc.fix}
                        </div>
                        <div className="text-xs text-[#b8b098]">
                          fixed in {inc.resolved_in_minutes ?? "?"} min
                        </div>
                      </>
                    )}
                    {!resolved && (
                      <>
                        <label className="block text-xs">
                          <span className="text-[#6A6245]">What was the actual cause?</span>
                          <textarea
                            value={cause}
                            onChange={(e) => setCause(e.target.value)}
                            rows={2}
                            className="input-recessed mt-1"
                          />
                        </label>
                        <label className="block text-xs">
                          <span className="text-[#6A6245]">What fixed it?</span>
                          <textarea
                            value={fix}
                            onChange={(e) => setFix(e.target.value)}
                            rows={2}
                            className="input-recessed mt-1"
                          />
                        </label>
                        <button
                          type="button"
                          disabled={busy || !cause.trim() || !fix.trim()}
                          onClick={() => handleSubmit(inc.id)}
                          className="btn-primary"
                        >
                          Save resolution
                        </button>
                      </>
                    )}

                    <details className="mt-3">
                      <summary className="cursor-pointer text-[10px] font-black uppercase tracking-[1.4px] text-[#6A6245]">
                        Causal chain
                      </summary>
                      <div className="lcd-panel mt-2">
                        <ol className="space-y-1 text-xs">
                          {inc.chain.map((l, idx) => (
                            <li key={idx}>
                              <span className="font-mono text-[#E87C14]">{l.service}</span>{" "}
                              <span className="text-[rgba(255,250,218,0.35)]">
                                (+{l.ts_offset_secs.toFixed(1)}s)
                              </span>{" "}
                              <span className="text-[rgba(255,250,218,0.5)]">— {l.sample}</span>
                            </li>
                          ))}
                        </ol>
                      </div>
                    </details>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
