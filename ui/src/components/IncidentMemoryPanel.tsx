import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Fingerprint } from "../api";

/** Context describing the incident the operator is currently facing, used to
 *  auto-filter the memory list when they jump here from the decision card. */
export interface IncidentFocus {
  rootCauseService: string | null;
  chainId: string | null;
  /** Backend-computed lookalikes (incident id → similarity), strongest signal. */
  similar: { id: string; similarity: number }[];
  /** Changes on every "show me more" click so the effect re-runs each time. */
  nonce: number;
}

interface Props {
  incidents: Fingerprint[];
  onResolve: (id: string, cause: string, fix: string) => Promise<void>;
  busy: boolean;
  focus?: IncidentFocus | null;
}

const PAGE_SIZE = 3;

function formatTime(unixSecs: number): string {
  return new Date(unixSecs * 1000).toLocaleString();
}

/** Rank incidents by how related they are to what the operator is facing.
 *  Backend similarity wins, then same causal chain, then same root-cause
 *  service; recency breaks ties. Only related incidents are kept. */
function rankByRelevance(
  incidents: Fingerprint[],
  focus: IncidentFocus,
): Fingerprint[] {
  const simById = new Map(focus.similar.map((s) => [s.id, s.similarity]));
  return incidents
    .map((inc) => {
      let score = 0;
      const sim = simById.get(inc.id);
      if (sim != null) score += 1000 + sim * 100;
      if (focus.chainId && inc.chain_id === focus.chainId) score += 400;
      if (focus.rootCauseService && inc.root_cause_service === focus.rootCauseService) {
        score += 200;
      }
      return { inc, score };
    })
    .filter((s) => s.score > 0)
    .sort((a, b) => b.score - a.score || b.inc.ts - a.inc.ts)
    .map((s) => s.inc);
}

export function IncidentMemoryPanel({ incidents, onResolve, busy, focus }: Props) {
  const [openId, setOpenId] = useState<string | null>(null);
  const [cause, setCause] = useState("");
  const [fix, setFix] = useState("");
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE);
  const [filterOff, setFilterOff] = useState(false);
  const detailsRef = useRef<HTMLDetailsElement>(null);
  const causeRef = useRef<HTMLTextAreaElement>(null);
  const fixRef = useRef<HTMLTextAreaElement>(null);

  // Grow the text boxes to fit their content so long write-ups never get
  // clipped or wrapped into an awkward scrollbar — they stay sleek up to a
  // sensible cap, after which they scroll internally.
  useLayoutEffect(() => {
    for (const el of [causeRef.current, fixRef.current]) {
      if (!el) continue;
      el.style.height = "auto";
      el.style.height = `${el.scrollHeight}px`;
    }
  }, [openId, cause, fix]);

  // When the operator jumps here from the decision card, pop the section open
  // and reset the filter/pagination so the most relevant incidents lead.
  const focusNonce = focus?.nonce ?? null;
  useEffect(() => {
    if (focusNonce == null) return;
    setFilterOff(false);
    setVisibleCount(PAGE_SIZE);
    const el = detailsRef.current;
    if (el && !el.open) el.open = true;
  }, [focusNonce]);

  const filtering = !!focus && !filterOff;
  const ranked = useMemo(
    () => (filtering && focus ? rankByRelevance(incidents, focus) : []),
    [filtering, focus, incidents],
  );
  const hasMatches = ranked.length > 0;
  const list = filtering && hasMatches ? ranked : incidents;
  const visible = list.slice(0, visibleCount);

  const handleSubmit = async (id: string) => {
    if (!cause.trim() || !fix.trim()) return;
    await onResolve(id, cause.trim(), fix.trim());
    // Keep the panel open with the saved text in place so the operator can
    // confirm the edit (and tweak it again if needed) instead of collapsing.
  };

  return (
    <section>
      <details ref={detailsRef} className="console-card" open>
        <summary className="flex cursor-pointer items-center justify-between">
          <div>
            <div className="eyebrow">Incident Memory</div>
            <div className="mt-1 text-xs text-[#8a8470]">
              Every chain Aegis has seen. Resolve one and the next on-call gets a head start.
            </div>
          </div>
          <div className="flex items-center gap-3">
            <div className="font-mono text-[11px] font-semibold text-[#5a5440]">
              {filtering && hasMatches ? `${ranked.length} / ${incidents.length}` : incidents.length}
            </div>
            <span className="disclosure-arrow" aria-hidden="true">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="6 9 12 15 18 9" />
              </svg>
            </span>
          </div>
        </summary>

        <div className="mt-4 space-y-3 border-t border-[rgba(0,0,0,0.06)] pt-4">
          {incidents.length === 0 && (
            <div className="lcd-panel text-sm text-[rgba(255,250,218,0.55)]">
              No incidents fingerprinted yet. The workload app injects them on
              its own — or fire one manually:
              <pre className="mt-2 overflow-x-auto rounded-md px-3 py-2 font-mono text-[10px] text-[#e07818]"
                style={{ backgroundColor: "rgba(0,0,0,0.3)" }}
              >
{`python demo/log_spammer.py --target tcp://127.0.0.1:5140 --pattern cascade`}
              </pre>
            </div>
          )}

          {filtering && incidents.length > 0 && (
            <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-[rgba(224,120,24,0.22)] bg-[rgba(224,120,24,0.07)] px-3 py-2 text-xs">
              <span className="text-[#5a5440]">
                {hasMatches ? (
                  <>
                    Related to the active incident
                    {focus?.rootCauseService && (
                      <>
                        {" · "}
                        <span className="font-bold text-[#2e2a1e]">
                          {focus.rootCauseService}
                        </span>
                      </>
                    )}
                  </>
                ) : (
                  <>No directly related incidents — showing all.</>
                )}
              </span>
              <button
                type="button"
                onClick={() => setFilterOff(true)}
                className="btn-tertiary !py-1 !px-3 !text-[10px]"
              >
                Show all
              </button>
            </div>
          )}

          {visible.map((inc) => {
            const open = openId === inc.id;
            const resolved = inc.cause && inc.fix;
            return (
              <div
                key={inc.id}
                className="module-card"
                style={
                  resolved
                    ? { borderColor: "rgba(34,160,80,0.2)" }
                    : {}
                }
              >
                <div className="flex items-stretch justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-1.5" title={inc.id}>
                      {inc.services.map((svc, idx) => {
                        const isRoot = svc === inc.root_cause_service;
                        return (
                          <span
                            key={`${svc}-${idx}`}
                            className={isRoot ? `svc-pill${resolved ? " is-resolved" : ""}` : "hop-pill"}
                          >
                            {svc}
                          </span>
                        );
                      })}
                      {resolved && (
                        <span className="hop-pill !text-[8px] !font-extrabold !uppercase !tracking-[1.2px]">
                          Resolved
                        </span>
                      )}
                    </div>

                    {open && (
                      <div className="mt-3 space-y-2 border-t border-[rgba(0,0,0,0.06)] pt-3">
                        <div className="space-y-3">
                          <label className="block">
                            <span className="field-label">Cause</span>
                            <textarea
                              ref={causeRef}
                              value={cause}
                              onChange={(e) => setCause(e.target.value)}
                              rows={2}
                              placeholder="What was the actual cause?"
                              className="input-recessed input-grow mt-1.5"
                            />
                          </label>
                          <label className="block">
                            <span className="field-label">Fix</span>
                            <textarea
                              ref={fixRef}
                              value={fix}
                              onChange={(e) => setFix(e.target.value)}
                              rows={2}
                              placeholder="What fixed it?"
                              className="input-recessed input-grow mt-1.5"
                            />
                          </label>
                          <div className="flex flex-wrap items-center gap-3">
                            <button
                              type="button"
                              disabled={
                                busy ||
                                !cause.trim() ||
                                !fix.trim() ||
                                (cause.trim() === (inc.cause ?? "").trim() &&
                                  fix.trim() === (inc.fix ?? "").trim())
                              }
                              onClick={() => handleSubmit(inc.id)}
                              className="btn-primary"
                            >
                              {resolved ? "Update resolution" : "Save resolution"}
                            </button>
                            {resolved && inc.resolved_in_minutes != null && (
                              <span className="text-[11px] text-[#8a8470]">
                                Originally fixed in {inc.resolved_in_minutes} min
                              </span>
                            )}
                          </div>
                        </div>

                        <details className="mt-3">
                          <summary className="cursor-pointer text-[10px] font-bold uppercase tracking-[1.4px] text-[#8a8470] hover:text-[#5a5440]">
                            Causal chain
                          </summary>
                          <div className="lcd-panel mt-2">
                            <ol className="space-y-1 text-xs">
                              {inc.chain.map((l, idx) => (
                                <li key={idx}>
                                  <span className="font-mono text-[#e07818]">{l.service}</span>{" "}
                                  <span className="text-[rgba(255,250,218,0.3)]">
                                    (+{l.ts_offset_secs.toFixed(1)}s)
                                  </span>{" "}
                                  <span className="text-[rgba(255,250,218,0.45)]">— {l.sample}</span>
                                </li>
                              ))}
                            </ol>
                          </div>
                        </details>
                      </div>
                    )}
                  </div>
                  <div className="flex shrink-0 items-start gap-3 border-l border-[rgba(0,0,0,0.1)] pl-3">
                    <span className="whitespace-nowrap pt-[7px] text-[11px] text-[#8a8470]">
                      {formatTime(inc.ts)}
                    </span>
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
                </div>
              </div>
            );
          })}

          {visibleCount < list.length && (
            <button
              type="button"
              onClick={() => setVisibleCount((c) => c + PAGE_SIZE)}
              className="btn-tertiary w-full justify-center !text-[11px]"
            >
              Load more · {list.length - visibleCount} more
            </button>
          )}
        </div>
      </details>
    </section>
  );
}
