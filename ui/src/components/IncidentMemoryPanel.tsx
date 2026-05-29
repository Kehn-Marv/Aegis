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
      <div className="rounded-2xl bg-white p-5 shadow-card ring-1 ring-neutral-200/60">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-[11px] font-medium uppercase tracking-wider text-neutral-400">
              Incident memory
            </div>
            <div className="mt-1 text-sm text-neutral-600">
              Every chain Aegis has seen. Resolve one — even with two short
              sentences — and the next on-call gets a head start.
            </div>
          </div>
          <div className="font-mono text-[11px] text-neutral-400">
            {incidents.length} fingerprint{incidents.length === 1 ? "" : "s"}
          </div>
        </div>

        <div className="mt-4 space-y-2.5">
          {incidents.length === 0 && (
            <div className="rounded-xl border border-neutral-200 bg-neutral-50/80 p-4 text-sm text-neutral-500">
              No incidents fingerprinted yet. The workload app injects them on
              its own — or plant one by hand:
              <pre className="mt-2 overflow-x-auto rounded-lg bg-neutral-100 p-2.5 font-mono text-[11px] text-neutral-700">
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
                className={`rounded-xl border p-3.5 ${
                  resolved
                    ? "border-emerald-200 bg-emerald-50/60"
                    : "border-amber-200 bg-amber-50/60"
                }`}
              >
                <div className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2 text-xs">
                      <span className="font-mono text-neutral-700">{inc.id}</span>
                      <span className="text-neutral-300">·</span>
                      <span className="text-neutral-500">
                        root cause:{" "}
                        <span className="font-semibold text-neutral-800">
                          {inc.root_cause_service}
                        </span>
                      </span>
                      <span className="text-neutral-300">·</span>
                      <span className="text-neutral-400">{formatTime(inc.ts)}</span>
                      {resolved && (
                        <span className="ml-1 rounded-md bg-emerald-100 px-2 py-0.5 text-[10px] font-bold uppercase tracking-widest text-emerald-700">
                          resolved
                        </span>
                      )}
                    </div>
                    <div className="mt-1 truncate text-xs text-neutral-500">
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
                    className="shrink-0 rounded-lg bg-white px-3 py-1 text-xs text-neutral-700 ring-1 ring-neutral-300 transition hover:bg-neutral-50"
                  >
                    {open ? "Hide" : resolved ? "View" : "Resolve"}
                  </button>
                </div>

                {open && (
                  <div className="mt-3 space-y-2">
                    {resolved && (
                      <>
                        <div className="text-xs text-neutral-700">
                          <span className="text-neutral-400">cause: </span>
                          {inc.cause}
                        </div>
                        <div className="text-xs text-neutral-700">
                          <span className="text-neutral-400">fix: </span>
                          {inc.fix}
                        </div>
                        <div className="text-xs text-neutral-400">
                          fixed in {inc.resolved_in_minutes ?? "?"} min
                        </div>
                      </>
                    )}
                    {!resolved && (
                      <>
                        <label className="block text-xs">
                          <span className="text-neutral-500">
                            What was the actual cause?
                          </span>
                          <textarea
                            value={cause}
                            onChange={(e) => setCause(e.target.value)}
                            rows={2}
                            className="mt-1 w-full rounded-lg border border-neutral-300 bg-white px-2.5 py-1.5 text-sm text-neutral-900 outline-none transition focus:border-[#0071e3] focus:ring-2 focus:ring-[#0071e3]/20"
                          />
                        </label>
                        <label className="block text-xs">
                          <span className="text-neutral-500">What fixed it?</span>
                          <textarea
                            value={fix}
                            onChange={(e) => setFix(e.target.value)}
                            rows={2}
                            className="mt-1 w-full rounded-lg border border-neutral-300 bg-white px-2.5 py-1.5 text-sm text-neutral-900 outline-none transition focus:border-[#0071e3] focus:ring-2 focus:ring-[#0071e3]/20"
                          />
                        </label>
                        <button
                          type="button"
                          disabled={busy || !cause.trim() || !fix.trim()}
                          onClick={() => handleSubmit(inc.id)}
                          className="rounded-lg bg-[#0071e3] px-3.5 py-1.5 text-xs font-semibold text-white transition hover:bg-[#0058b9] disabled:cursor-not-allowed disabled:bg-neutral-300"
                        >
                          Save resolution
                        </button>
                      </>
                    )}

                    <details className="mt-3">
                      <summary className="cursor-pointer text-[11px] font-medium uppercase tracking-wider text-neutral-400">
                        Causal chain
                      </summary>
                      <ol className="mt-1.5 space-y-1 text-xs text-neutral-600">
                        {inc.chain.map((l, idx) => (
                          <li key={idx}>
                            <span className="font-mono text-neutral-800">{l.service}</span>{" "}
                            <span className="text-neutral-400">
                              (+{l.ts_offset_secs.toFixed(1)}s)
                            </span>{" "}
                            <span className="text-neutral-500">— {l.sample}</span>
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
