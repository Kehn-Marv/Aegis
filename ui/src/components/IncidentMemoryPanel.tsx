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
    <section className="px-6 pb-6">
      <div className="rounded-lg border border-slate-800/80 bg-slate-900/40 p-5">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-widest text-slate-400">
              Incident memory
            </div>
            <div className="mt-1 text-sm text-slate-300">
              Every chain Aegis has seen. Resolve one — even with two short
              sentences — and the next on-call gets a head start.
            </div>
          </div>
          <div className="text-[11px] font-mono text-slate-500">
            {incidents.length} fingerprint{incidents.length === 1 ? "" : "s"}
          </div>
        </div>

        <div className="mt-4 space-y-2">
          {incidents.length === 0 && (
            <div className="rounded border border-slate-800 bg-slate-950/40 p-4 text-sm text-slate-500">
              No incidents fingerprinted yet. Run a cascade pattern in the
              spammer to plant one:
              <pre className="mt-2 overflow-x-auto rounded bg-slate-950/70 p-2 font-mono text-[11px] text-slate-300">
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
                className={`rounded border ${
                  resolved
                    ? "border-emerald-700/40 bg-emerald-500/5"
                    : "border-amber-700/40 bg-amber-500/5"
                } p-3`}
              >
                <div className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2 text-xs">
                      <span className="font-mono text-slate-200">{inc.id}</span>
                      <span className="text-slate-500">·</span>
                      <span className="text-slate-300">
                        root cause:{" "}
                        <span className="text-slate-100 font-semibold">
                          {inc.root_cause_service}
                        </span>
                      </span>
                      <span className="text-slate-500">·</span>
                      <span className="text-slate-500">{formatTime(inc.ts)}</span>
                      {resolved && (
                        <span className="ml-1 rounded bg-emerald-500/20 px-2 py-0.5 text-[10px] font-bold uppercase tracking-widest text-emerald-300">
                          resolved
                        </span>
                      )}
                    </div>
                    <div className="mt-1 text-xs text-slate-400 truncate">
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
                    className="shrink-0 rounded border border-slate-700 px-3 py-1 text-xs text-slate-200 hover:border-slate-500 hover:bg-slate-800"
                  >
                    {open ? "Hide" : resolved ? "View" : "Resolve"}
                  </button>
                </div>

                {open && (
                  <div className="mt-3 space-y-2">
                    {resolved && (
                      <>
                        <div className="text-xs text-slate-300">
                          <span className="text-slate-500">cause: </span>
                          {inc.cause}
                        </div>
                        <div className="text-xs text-slate-300">
                          <span className="text-slate-500">fix: </span>
                          {inc.fix}
                        </div>
                        <div className="text-xs text-slate-500">
                          fixed in {inc.resolved_in_minutes ?? "?"} min
                        </div>
                      </>
                    )}
                    {!resolved && (
                      <>
                        <label className="block text-xs">
                          <span className="text-slate-400">
                            What was the actual cause?
                          </span>
                          <textarea
                            value={cause}
                            onChange={(e) => setCause(e.target.value)}
                            rows={2}
                            className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-100 focus:border-emerald-500/60 focus:outline-none"
                          />
                        </label>
                        <label className="block text-xs">
                          <span className="text-slate-400">What fixed it?</span>
                          <textarea
                            value={fix}
                            onChange={(e) => setFix(e.target.value)}
                            rows={2}
                            className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-100 focus:border-emerald-500/60 focus:outline-none"
                          />
                        </label>
                        <button
                          type="button"
                          disabled={busy || !cause.trim() || !fix.trim()}
                          onClick={() => handleSubmit(inc.id)}
                          className="rounded-md bg-emerald-500/90 px-3 py-1.5 text-xs font-semibold text-slate-950 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400"
                        >
                          Save resolution
                        </button>
                      </>
                    )}

                    <details className="mt-3">
                      <summary className="cursor-pointer text-[11px] uppercase tracking-widest text-slate-500">
                        Causal chain
                      </summary>
                      <ol className="mt-1 space-y-1 text-xs text-slate-300">
                        {inc.chain.map((l, idx) => (
                          <li key={idx}>
                            <span className="font-mono text-slate-100">
                              {l.service}
                            </span>{" "}
                            <span className="text-slate-500">
                              (+{l.ts_offset_secs.toFixed(1)}s)
                            </span>{" "}
                            <span className="text-slate-400">— {l.sample}</span>
                          </li>
                        ))}
                      </ol>
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
