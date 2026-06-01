import { useState, useCallback } from "react";
import type { DecisionCard, GatewayStatus, IncidentMatch } from "../api";

interface Props {
  status: GatewayStatus | null;
  reachable: boolean;
  onAcknowledge: () => Promise<void>;
  onShowMore: () => void;
  onDifferent: () => void;
  busy: boolean;
}

function formatAgo(unixSecs: number): string {
  const delta = Math.max(0, Math.floor(Date.now() / 1000 - unixSecs));
  if (delta < 60) return `${delta}s ago`;
  if (delta < 3600) return `${Math.floor(delta / 60)} min ago`;
  if (delta < 86400) return `${Math.floor(delta / 3600)} h ago`;
  return `${Math.floor(delta / 86400)} d ago`;
}

function Match({ match }: { match: IncidentMatch }) {
  const sim = Math.round(match.similarity * 100);
  const ago = formatAgo(match.past_ts);
  return (
    <div className="module-card">
      <div className="flex items-center justify-between text-xs">
        <div className="font-mono font-bold text-[#2e2a1e]">{sim}% match</div>
        <div className="text-[#8a8470]">{ago}</div>
      </div>
      <div className="mt-1.5 text-xs text-[#5a5440]">
        root cause:{" "}
        <span className="font-bold text-[#2e2a1e]">{match.past_root_cause_service}</span>
        {match.past_resolved_in_minutes != null && (
          <> · fixed in {match.past_resolved_in_minutes} min</>
        )}
      </div>
      {match.past_cause ? (
        <div className="mt-2 text-xs text-[#2e2a1e]">
          <span className="text-[#8a8470]">cause:</span> {match.past_cause}
        </div>
      ) : (
        <div className="mt-2 text-xs italic text-[#8a8470]">
          No resolution recorded — resolve this one so the next on-call has a head start.
        </div>
      )}
      {match.past_fix && (
        <div className="mt-1 text-xs text-[#2e2a1e]">
          <span className="text-[#8a8470]">fix:</span> {match.past_fix}
        </div>
      )}
    </div>
  );
}



export function DecisionCardPanel({
  status,
  reachable,
  onAcknowledge,
  onShowMore,
  onDifferent,
  busy,
}: Props) {
  const [showRaw, setShowRaw] = useState(false);
  const [toast, setToast] = useState<{ msg: string; visible: boolean } | null>(null);
  const card: DecisionCard | null = status?.decision ?? null;

  const showToast = useCallback((msg: string) => {
    setToast({ msg, visible: true });
    setTimeout(() => setToast((t) => t ? { ...t, visible: false } : null), 2200);
    setTimeout(() => setToast(null), 2600);
  }, []);

  const handleAck = useCallback(async () => {
    await onAcknowledge();
    showToast("Acknowledged — you're on it");
  }, [onAcknowledge, showToast]);

  const handleShowMore = useCallback(() => {
    onShowMore();
    showToast("Filtered memory to related incidents");
  }, [onShowMore, showToast]);

  const handleDifferent = useCallback(() => {
    onDifferent();
    showToast("Feedback noted — marked as different");
  }, [onDifferent, showToast]);

  if (!reachable) {
    return (
      <section>
        <div className="console-card" style={{ borderColor: "rgba(200,48,32,0.2)" }}>
          <div className="lcd-panel flex items-center gap-3">
            <span className="led led-red" />
            <span className="text-sm text-[#c83020]">
              Gateway unreachable. Make sure{" "}
              <span className="font-mono text-[#fffada]">aegis-daemon</span>{" "}
              is running on{" "}
              <span className="font-mono text-[#fffada]">127.0.0.1:7321</span>.
            </span>
          </div>
        </div>
      </section>
    );
  }

  if (!card || card.state === "green") {
    return (
      <section>
        <div className="console-card">
          <div className="mb-3">
            <span className="eyebrow">System Status</span>
          </div>
          <div className="lcd-panel scan-sweep">
            <div className="flex items-center gap-2">
              <span className="text-sm font-bold text-[#e07818]">SYSTEM READY</span>
              <span className="ml-auto font-mono text-[10px] text-[rgba(255,250,218,0.3)]">
                v0.2.0
              </span>
            </div>
            <p className="mt-3 text-sm leading-relaxed text-[rgba(255,250,218,0.6)]">
              {card?.headline ??
                "No causal chains, no silent services, dedup is working. Aegis is watching for first-fire patterns."}
            </p>
            {status && (
              <div className="mt-4 grid grid-cols-3 gap-px overflow-hidden rounded-md border border-[rgba(255,250,218,0.08)] bg-[rgba(255,250,218,0.06)]">
                {[
                  { k: "Noise stopped", v: `${status.dedup_savings_pct.toFixed(0)}%` },
                  { k: "Queue", v: String(status.queue_depth) },
                  { k: "Remembered", v: String(status.incidents_remembered) },
                ].map((s) => (
                  <div key={s.k} className="bg-[#141408] px-3 py-2">
                    <div className="text-[8px] font-bold uppercase tracking-[1.2px] text-[rgba(255,250,218,0.3)]">
                      {s.k}
                    </div>
                    <div className="mt-0.5 font-mono text-base font-black tabular-nums text-[#e07818]">
                      {s.v}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </section>
    );
  }

  const isRed = card.state === "red";
  return (
    <section>
      <div
        className="console-card"
        style={
          isRed
            ? { borderColor: "rgba(200,48,32,0.2)" }
            : { borderColor: "rgba(200,168,32,0.2)" }
        }
      >
        {/* Header */}
        <div className="mb-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="eyebrow">Decision Card</span>
          </div>
          <button
            type="button"
            onClick={() => setShowRaw((v) => !v)}
            className="font-mono text-[10px] text-[#8a8470] transition hover:text-[#2e2a1e]"
          >
            {showRaw ? "HIDE RAW" : "SHOW RAW"}
          </button>
        </div>

        {/* Main LCD */}
        <div
          className="lcd-panel scan-sweep"
          style={
            isRed
              ? { boxShadow: "0 0 20px rgba(200,48,32,0.15) inset, 0 3px 10px rgba(0,0,0,0.6) inset" }
              : { boxShadow: "0 0 20px rgba(200,168,32,0.1) inset, 0 3px 10px rgba(0,0,0,0.6) inset" }
          }
        >
          <div className="flex items-center gap-2">
            <span
              className="text-[10px] font-bold uppercase tracking-[1.4px]"
              style={{ color: isRed ? "#c83020" : "#c8a820" }}
            >
              {isRed ? "ALERT" : "WARNING"}
            </span>
          </div>
          <h2 className="mt-2 text-2xl font-black uppercase tracking-[3px] text-[#e07818]">
            {card.root_cause_service ?? "Active incident"}
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-[rgba(255,250,218,0.75)]">
            {card.headline}
          </p>

          {card.business_impact && (
            <div className="mt-4 rounded-md px-3 py-2 text-xs text-[rgba(255,250,218,0.55)]"
              style={{ backgroundColor: "rgba(0,0,0,0.25)", border: "1px solid rgba(255,250,218,0.08)" }}
            >
              <span className="text-[rgba(255,250,218,0.3)]">why this matters: </span>
              {card.business_impact}
            </div>
          )}
        </div>

        {/* Suggested step — light module card */}
        <div className="mt-4">
          <div className="eyebrow">Suggested Next Step</div>
          <div className="module-card mt-2 text-sm leading-relaxed text-[#2e2a1e]">
            {card.suggested_next_step}
          </div>
        </div>

        {/* Similar past incidents */}
        {card.similar_incidents.length > 0 && (
          <div className="mt-4">
            <div className="eyebrow">Similar Past Incidents</div>
            <div className="mt-2 grid gap-3 md:grid-cols-2">
              {card.similar_incidents.slice(0, 3).map((m) => (
                <Match key={m.incident_id} match={m} />
              ))}
            </div>
          </div>
        )}

        {/* Actions */}
        <div className="mt-6 flex flex-wrap gap-3">
          <button type="button" disabled={busy} onClick={handleAck} className="btn-primary">
            {busy ? "Sending…" : "I'm on it"}
          </button>
          <button type="button" onClick={handleShowMore} className="btn-secondary">
            Show me more
          </button>
          <button type="button" onClick={handleDifferent} className="btn-secondary">
            Looks different
          </button>
        </div>

        {/* Toast notification */}
        {toast && (
          <div
            className="mt-3 flex items-center gap-2 rounded-lg px-4 py-2.5 text-xs font-semibold transition-all duration-300"
            style={{
              background: "linear-gradient(180deg, #2a2618, #1e1c12)",
              border: "1px solid rgba(200,144,96,0.2)",
              color: "#fffada",
              opacity: toast.visible ? 1 : 0,
              transform: toast.visible ? "translateY(0)" : "translateY(-6px)",
              boxShadow: "0 2px 8px rgba(0,0,0,0.2)",
            }}
          >
            <span style={{ color: "#c89060" }}>✓</span>
            <span>{toast.msg}</span>
          </div>
        )}

        {showRaw && (
          <div className="lcd-panel mt-4">
            <pre className="overflow-x-auto font-mono text-[10px] leading-relaxed text-[rgba(255,250,218,0.5)]">
              {JSON.stringify(card, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </section>
  );
}
