import { useState, useCallback, useRef, useEffect } from "react";
import { createPortal } from "react-dom";
import type { DecisionCard, GatewayStatus, IncidentMatch } from "../api";

interface Props {
  status: GatewayStatus | null;
  reachable: boolean;
  onAcknowledge: () => Promise<void>;
  onShowMore: () => void;
  onDifferent: () => void;
  busy: boolean;
  /** Called when the card's 5-second close timer fires after an action */
  onActionTaken?: () => void;
}

function formatAgo(unixSecs: number): string {
  const delta = Math.max(0, Math.floor(Date.now() / 1000 - unixSecs));
  if (delta < 60) return `${delta}s ago`;
  if (delta < 3600) return `${Math.floor(delta / 60)} min ago`;
  if (delta < 86400) return `${Math.floor(delta / 3600)} h ago`;
  return `${Math.floor(delta / 86400)} d ago`;
}

/** Sort similar incidents: those with a fix first, then by resolution speed (faster = earlier). */
function sortMatches(matches: IncidentMatch[]): IncidentMatch[] {
  return [...matches].sort((a, b) => {
    const aHasFix = a.past_fix != null ? 1 : 0;
    const bHasFix = b.past_fix != null ? 1 : 0;
    if (bHasFix !== aHasFix) return bHasFix - aHasFix; // fixes first
    // Both have fix or both don't — sort by resolution speed (null = slower)
    const aMin = a.past_resolved_in_minutes ?? Infinity;
    const bMin = b.past_resolved_in_minutes ?? Infinity;
    if (aMin !== bMin) return aMin - bMin; // faster first
    return b.past_ts - a.past_ts; // most recent first as final tiebreak
  });
}

function Match({ match }: { match: IncidentMatch }) {
  const sim = Math.round(match.similarity * 100);
  const ago = formatAgo(match.past_ts);
  const hasFix = match.past_fix != null;
  return (
    <div
      className="module-card"
      style={hasFix ? { borderColor: "rgba(34,160,80,0.25)" } : {}}
    >
      <div className="flex items-center justify-between text-xs">
        <div className="font-mono font-bold text-[#2e2a1e]">{sim}% match</div>
        <div className="flex items-center gap-2">
          {hasFix && (
            <span
              style={{
                background: "rgba(34,160,80,0.14)",
                color: "#22a050",
                border: "1px solid rgba(34,160,80,0.3)",
                fontSize: 9,
                fontWeight: 800,
                letterSpacing: "1px",
                textTransform: "uppercase",
                padding: "2px 7px",
                borderRadius: 999,
              }}
            >
              Fix recorded
            </span>
          )}
          <div className="text-[#8a8470]">{ago}</div>
        </div>
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

/* ─── Centred backdrop toast ─────────────────────────────────────────────── */
interface CentreToastProps {
  msg: string;
  sub?: string;
  visible: boolean;
  icon?: string;
  onClose?: () => void;
}

function CentreToast({ msg, sub, visible, icon = "ℹ", onClose }: CentreToastProps) {
  return createPortal(
    <div
      aria-live="assertive"
      role="alertdialog"
      onClick={onClose}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 9999,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        pointerEvents: visible ? "auto" : "none",
        // blurred backdrop
        backdropFilter: visible ? "blur(6px) saturate(0.7)" : "blur(0px)",
        backgroundColor: visible ? "rgba(13,13,7,0.45)" : "rgba(13,13,7,0)",
        transition: "backdrop-filter 0.3s ease, background-color 0.3s ease",
      }}
    >
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 12,
          padding: "28px 36px",
          borderRadius: 20,
          maxWidth: "min(420px, calc(100vw - 48px))",
          textAlign: "center",
          background: "linear-gradient(160deg, #2a2618 0%, #1a1a0e 100%)",
          border: "1px solid rgba(200,144,96,0.2)",
          boxShadow:
            "0 24px 64px -12px rgba(0,0,0,0.65), 0 4px 16px rgba(0,0,0,0.3), 0 0 0 1px rgba(255,250,218,0.04) inset",
          opacity: visible ? 1 : 0,
          transform: visible ? "scale(1) translateY(0)" : "scale(0.9) translateY(16px)",
          transition:
            "opacity 0.3s cubic-bezier(0.22,1,0.36,1), transform 0.35s cubic-bezier(0.22,1,0.36,1)",
          pointerEvents: "none",
        }}
      >
        <div
          style={{
            width: 44,
            height: 44,
            borderRadius: "50%",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            background: "rgba(200,144,96,0.12)",
            border: "1px solid rgba(200,144,96,0.25)",
            fontSize: 20,
            color: "#c89060",
          }}
        >
          {icon}
        </div>
        <div
          style={{
            fontSize: 15,
            fontWeight: 700,
            color: "#fffada",
            lineHeight: 1.4,
          }}
        >
          {msg}
        </div>
        {sub && (
          <div
            style={{
              fontSize: 12,
              color: "rgba(255,250,218,0.5)",
              lineHeight: 1.5,
            }}
          >
            {sub}
          </div>
        )}
        <div
          style={{
            marginTop: 4,
            fontSize: 10,
            color: "rgba(255,250,218,0.25)",
            letterSpacing: "0.8px",
            textTransform: "uppercase",
          }}
        >
          Click anywhere to dismiss
        </div>
      </div>
    </div>,
    document.body,
  );
}

/* ─── Main component ─────────────────────────────────────────────────────── */
export function DecisionCardPanel({
  status,
  reachable,
  onAcknowledge,
  onShowMore,
  onDifferent,
  busy,
  onActionTaken,
}: Props) {
  const [showRaw, setShowRaw] = useState(false);
  const [centreToast, setCentreToast] = useState<{
    msg: string;
    sub?: string;
    icon?: string;
    visible: boolean;
  } | null>(null);
  /** Set to true once ANY action button is clicked; card hides 5s later */
  const [actionDone, setActionDone] = useState(false);
  /** When true, show the normal idle/System-Status view instead of the incident card */
  const [dismissed, setDismissed] = useState(false);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  /** Track the decision card ID we've rendered so we can reset state on a new one */
  const lastCardIdRef = useRef<string | null>(null);
  /** Track whether Show Me More was clicked for the current card */
  const [showMoreActive, setShowMoreActive] = useState(false);

  /** Pinned snapshot of the last real incident card.
   *  We capture it here instead of reading status.decision directly so that
   *  background polling clearing the decision (auto-resolve, timeout, etc.)
   *  can't cause the card to disappear without the user clicking anything. */
  const [pinnedCard, setPinnedCard] = useState<DecisionCard | null>(null);

  const incomingCard: DecisionCard | null = status?.decision ?? null;

  // Only update the pinned card when a brand-new active incident arrives.
  // A "new" card means a different decision_id with a non-green state.
  // When the backend clears the decision (null / green) we deliberately
  // ignore it — the card stays until the user clicks a button.
  useEffect(() => {
    if (
      incomingCard &&
      incomingCard.state !== "green" &&
      incomingCard.decision_id !== lastCardIdRef.current
    ) {
      lastCardIdRef.current = incomingCard.decision_id;
      setPinnedCard(incomingCard);
      setActionDone(false);
      setDismissed(false);
      setShowMoreActive(false);
      if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    }
  }, [incomingCard]);

  // Use the pinned card for all rendering — never the live polling value.
  const card = dismissed ? null : pinnedCard;

  const showCentreToast = useCallback(
    (msg: string, sub?: string, icon?: string) => {
      setCentreToast({ msg, sub: sub, icon: icon ?? "ℹ", visible: true });
    },
    [],
  );

  const dismissCentreToast = useCallback(() => {
    setCentreToast((t) => (t ? { ...t, visible: false } : null));
    setTimeout(() => setCentreToast(null), 350);
  }, []);

  /** Mark action done → start 5-second countdown, then show idle System Status view */
  const markActionDone = useCallback(() => {
    setActionDone(true);
    if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    closeTimerRef.current = setTimeout(() => {
      setDismissed(true);
      onActionTaken?.();
    }, 5000);
  }, [onActionTaken]);

  const handleAck = useCallback(async () => {
    await onAcknowledge();
    markActionDone();
    showCentreToast(
      "You're on it",
      "Acknowledged — the team has been notified.",
      "✓",
    );
    setTimeout(dismissCentreToast, 2800);
  }, [onAcknowledge, markActionDone, showCentreToast, dismissCentreToast]);

  const handleShowMore = useCallback(() => {
    const hasPastIncidents =
      card != null && card.similar_incidents.length > 0;

    if (!hasPastIncidents) {
      // Fresh incident — no memory to show
      showCentreToast(
        "No past incidents on record",
        "This looks like a fresh case. Resolve it and the next on-call will have a head start.",
        "📋",
      );
      setTimeout(dismissCentreToast, 3500);
      return;
    }

    setShowMoreActive(true);
    onShowMore();
    markActionDone();
    showCentreToast(
      "Filtered memory to related incidents",
      "Scroll down to see the incident history for this root cause.",
      "🔍",
    );
    setTimeout(dismissCentreToast, 2800);
  }, [card, onShowMore, markActionDone, showCentreToast, dismissCentreToast]);

  const handleDifferent = useCallback(() => {
    onDifferent();
    markActionDone();
    showCentreToast(
      "Feedback noted",
      "Marked as 'looks different' — Aegis will refine its matching.",
      "↩",
    );
    setTimeout(dismissCentreToast, 2800);
  }, [onDifferent, markActionDone, showCentreToast, dismissCentreToast]);

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

  if (!card || card.state === "green" || dismissed) {
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
  const hasPastIncidents = card.similar_incidents.length > 0;
  const sortedMatches = sortMatches(card.similar_incidents);

  return (
    <>
      {/* Blurred-backdrop centre toast */}
      {centreToast && (
        <CentreToast
          msg={centreToast.msg}
          sub={centreToast.sub}
          visible={centreToast.visible}
          icon={centreToast.icon}
          onClose={dismissCentreToast}
        />
      )}

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
              {actionDone && (
                <span
                  style={{
                    fontSize: 10,
                    fontWeight: 700,
                    letterSpacing: "0.8px",
                    color: "rgba(34,160,80,0.8)",
                    background: "rgba(34,160,80,0.1)",
                    border: "1px solid rgba(34,160,80,0.25)",
                    borderRadius: 999,
                    padding: "2px 8px",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 4,
                  }}
                >
                  <span
                    style={{
                      display: "inline-block",
                      width: 6,
                      height: 6,
                      borderRadius: "50%",
                      background: "#22a050",
                      animation: "pulse-green 2s ease-in-out infinite",
                    }}
                  />
                  Closing in 5s…
                </span>
              )}
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
              <div
                className="mt-4 rounded-md px-3 py-2 text-xs text-[rgba(255,250,218,0.55)]"
                style={{ backgroundColor: "rgba(0,0,0,0.25)", border: "1px solid rgba(255,250,218,0.08)" }}
              >
                <span className="text-[rgba(255,250,218,0.3)]">why this matters: </span>
                {card.business_impact}
              </div>
            )}
          </div>

          {/* Suggested step */}
          <div className="mt-4">
            <div className="eyebrow">Suggested Next Step</div>
            <div className="module-card mt-2 text-sm leading-relaxed text-[#2e2a1e]">
              {card.suggested_next_step}
            </div>
          </div>

          {/* Similar past incidents */}
          {hasPastIncidents && (
            <div className="mt-4">
              <div className="eyebrow flex items-center gap-2">
                Similar Past Incidents
                <span
                  style={{
                    fontSize: 9,
                    fontWeight: 700,
                    letterSpacing: "0.8px",
                    color: "#8a8470",
                    textTransform: "none",
                  }}
                >
                  · fixes shown first
                </span>
              </div>
              <div className="mt-2 grid gap-3 md:grid-cols-2">
                {sortedMatches.slice(0, 3).map((m) => (
                  <Match key={m.incident_id} match={m} />
                ))}
              </div>
            </div>
          )}

          {/* Actions */}
          <div className="mt-6 flex flex-wrap gap-3">
            <button
              type="button"
              disabled={busy}
              onClick={handleAck}
              className="btn-primary"
            >
              {busy ? "Sending…" : "I'm on it"}
            </button>

            {/* Show me more — disabled + explains itself if no past incidents */}
            <button
              type="button"
              onClick={handleShowMore}
              disabled={showMoreActive}
              className="btn-secondary"
              title={
                !hasPastIncidents
                  ? "No past incidents for this root cause"
                  : showMoreActive
                  ? "Already showing related memory"
                  : undefined
              }
              style={
                !hasPastIncidents
                  ? { opacity: 0.55, cursor: "not-allowed" }
                  : showMoreActive
                  ? { opacity: 0.65, cursor: "default" }
                  : {}
              }
            >
              {showMoreActive ? "Showing more ✓" : "Show me more"}
            </button>

            <button
              type="button"
              onClick={handleDifferent}
              className="btn-secondary"
            >
              Looks different
            </button>
          </div>

          {showRaw && (
            <div className="lcd-panel mt-4">
              <pre className="overflow-x-auto font-mono text-[10px] leading-relaxed text-[rgba(255,250,218,0.5)]">
                {JSON.stringify(card, null, 2)}
              </pre>
            </div>
          )}
        </div>
      </section>
    </>
  );
}
