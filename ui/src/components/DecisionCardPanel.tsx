import { useState } from "react";
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
        <div className="font-mono font-bold text-[#22C45A]">{sim}% similar</div>
        <div className="text-[#6A6245]">{ago}</div>
      </div>
      <div className="mt-1.5 text-xs text-[#6A6245]">
        root cause:{" "}
        <span className="font-bold text-[#3D3520]">{match.past_root_cause_service}</span>
        {match.past_resolved_in_minutes != null && (
          <> · fixed in {match.past_resolved_in_minutes} min</>
        )}
      </div>
      {match.past_cause ? (
        <div className="mt-2 text-xs text-[#3D3520]">
          <span className="text-[#6A6245]">cause:</span> {match.past_cause}
        </div>
      ) : (
        <div className="mt-2 text-xs italic text-[#6A6245]">
          No resolution recorded — fix this one and write it down so the next
          on-call has a head start.
        </div>
      )}
      {match.past_fix && (
        <div className="mt-1 text-xs text-[#3D3520]">
          <span className="text-[#6A6245]">fix:</span> {match.past_fix}
        </div>
      )}
    </div>
  );
}

function StateBadge({ state }: { state: DecisionCard["state"] }) {
  const config = {
    green: { ledClass: "led led-green", label: "GREEN", badgeClass: "badge badge-green" },
    orange: { ledClass: "led led-amber", label: "ORANGE", badgeClass: "badge badge-amber" },
    red: { ledClass: "led led-red", label: "RED", badgeClass: "badge badge-red" },
  }[state];
  return (
    <span className={config.badgeClass}>
      <span className={config.ledClass} />
      {config.label}
    </span>
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
  const card: DecisionCard | null = status?.decision ?? null;

  if (!reachable) {
    return (
      <section className="px-4 pb-4 pt-4">
        <div className="console-card" style={{ borderColor: "rgba(212,48,32,0.3)" }}>
          <div className="lcd-panel flex items-center gap-3">
            <span className="led led-red" />
            <span className="text-sm" style={{ color: "#D43020" }}>
              Gateway unreachable. Make sure{" "}
              <span className="font-mono text-[#E87C14]">aegis-daemon</span>{" "}
              is running on{" "}
              <span className="font-mono text-[#E87C14]">127.0.0.1:7321</span>.
            </span>
          </div>
        </div>
      </section>
    );
  }

  if (!card || card.state === "green") {
    return (
      <section className="px-4 pb-4 pt-4">
        <div className="console-card">
          <div className="mb-3 flex items-center gap-3">
            <StateBadge state="green" />
            <span className="eyebrow">System Status</span>
          </div>
          <div className="lcd-panel">
            <div className="flex items-center gap-2">
              <span className="led led-green" />
              <span className="text-sm font-bold text-[#44C464]">SYS.RDY</span>
              <span className="ml-auto font-mono text-[10px] text-[#6A6245]">v0.2.0</span>
            </div>
            <p className="mt-3 text-sm leading-relaxed text-[rgba(255,250,218,0.7)]">
              {card?.headline ??
                "No causal chains, no silent services, dedup is working. Aegis is watching for first-fire patterns."}
            </p>
          </div>
        </div>
      </section>
    );
  }

  const isRed = card.state === "red";
  return (
    <section className="px-4 pb-4 pt-4">
      <div
        className="console-card"
        style={
          isRed
            ? { borderColor: "rgba(212,48,32,0.3)" }
            : { borderColor: "rgba(212,192,32,0.3)" }
        }
      >
        {/* Header */}
        <div className="mb-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <StateBadge state={card.state} />
            <span className="eyebrow">Decision Card</span>
          </div>
          <button
            type="button"
            onClick={() => setShowRaw((v) => !v)}
            className="font-mono text-[10px] text-[#6A6245] transition hover:text-[#3D3520]"
          >
            {showRaw ? "HIDE RAW" : "SHOW RAW"}
          </button>
        </div>

        {/* Main LCD display — this stays dark */}
        <div
          className="lcd-panel"
          style={
            isRed
              ? { boxShadow: "rgba(220,48,48,0.25) 0px 0px 20px 0px inset, rgba(0,0,0,0.55) 0px 4px 12px 0px inset" }
              : { boxShadow: "rgba(212,192,32,0.15) 0px 0px 20px 0px inset, rgba(0,0,0,0.55) 0px 4px 12px 0px inset" }
          }
        >
          <div className="flex items-center gap-2">
            <span
              className="text-[10px] font-bold uppercase tracking-[1.4px]"
              style={{ color: isRed ? "#D43020" : "#D4C020" }}
            >
              {isRed ? "ALERT" : "WARNING"}
            </span>
            <span className={isRed ? "led led-red" : "led led-amber"} />
          </div>
          <h2 className="mt-2 text-2xl font-black uppercase tracking-[3px] text-[#E87C14]">
            {card.root_cause_service ?? "Active incident"}
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-[rgba(255,250,218,0.8)]">
            {card.headline}
          </p>

          {card.business_impact && (
            <div className="mt-4 rounded px-3 py-2 text-xs text-[rgba(255,250,218,0.6)]"
              style={{ backgroundColor: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,250,218,0.1)" }}
            >
              <span className="text-[rgba(255,250,218,0.35)]">why this matters: </span>
              {card.business_impact}
            </div>
          )}
        </div>

        {/* Suggested next step — light section */}
        <div className="mt-4">
          <div className="eyebrow">Suggested Next Step</div>
          <div className="module-card mt-2 text-sm leading-relaxed text-[#3D3520]">
            {card.suggested_next_step}
          </div>
        </div>

        {/* Similar past incidents */}
        {card.similar_incidents.length > 0 && (
          <div className="mt-4">
            <div className="eyebrow">Similar Past Incidents</div>
            <div className="mt-2 grid gap-3 md:grid-cols-2">
              {card.similar_incidents.slice(0, 4).map((m) => (
                <Match key={m.incident_id} match={m} />
              ))}
            </div>
          </div>
        )}

        {/* Action buttons — all visible now */}
        <div className="mt-6 flex flex-wrap gap-3">
          <button
            type="button"
            disabled={busy}
            onClick={onAcknowledge}
            className="btn-primary"
          >
            I'm on it
          </button>
          <button type="button" onClick={onShowMore} className="btn-secondary">
            Show me more
          </button>
          <button type="button" onClick={onDifferent} className="btn-secondary">
            Looks different
          </button>
        </div>

        {/* Raw JSON */}
        {showRaw && (
          <div className="lcd-panel mt-4">
            <pre className="overflow-x-auto font-mono text-[10px] leading-relaxed text-[rgba(255,250,218,0.6)]">
              {JSON.stringify(card, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </section>
  );
}
