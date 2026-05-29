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

const EYEBROW = "text-[11px] font-medium uppercase tracking-wider text-neutral-400";

function Match({ match }: { match: IncidentMatch }) {
  const sim = Math.round(match.similarity * 100);
  const ago = formatAgo(match.past_ts);
  return (
    <div className="rounded-xl border border-neutral-200 bg-neutral-50/80 p-3.5">
      <div className="flex items-center justify-between text-xs">
        <div className="font-mono font-semibold text-emerald-700">{sim}% similar</div>
        <div className="text-neutral-400">{ago}</div>
      </div>
      <div className="mt-1.5 text-xs text-neutral-500">
        root cause: <span className="font-medium text-neutral-800">{match.past_root_cause_service}</span>
        {match.past_resolved_in_minutes != null && (
          <> · fixed in {match.past_resolved_in_minutes} min</>
        )}
      </div>
      {match.past_cause ? (
        <div className="mt-2 text-xs text-neutral-700">
          <span className="text-neutral-400">cause:</span> {match.past_cause}
        </div>
      ) : (
        <div className="mt-2 text-xs italic text-neutral-400">
          No resolution recorded — fix this one and write it down so the next
          on-call has a head start.
        </div>
      )}
      {match.past_fix && (
        <div className="mt-1 text-xs text-neutral-700">
          <span className="text-neutral-400">fix:</span> {match.past_fix}
        </div>
      )}
    </div>
  );
}

function StateBadge({ state }: { state: DecisionCard["state"] }) {
  const tone = {
    green: "bg-emerald-50 text-emerald-700 ring-emerald-600/20",
    orange: "bg-amber-50 text-amber-700 ring-amber-600/20",
    red: "bg-rose-50 text-rose-700 ring-rose-600/20",
  }[state];
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-[10px] font-bold uppercase tracking-widest ring-1 ${tone}`}
    >
      {state}
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
      <section className="px-6 pb-6">
        <div className="rounded-2xl border border-rose-200 bg-rose-50 p-6 text-sm text-rose-700 shadow-soft">
          Gateway unreachable. Make sure <span className="font-mono">aegis-daemon</span>{" "}
          is running on <span className="font-mono">127.0.0.1:7321</span>.
        </div>
      </section>
    );
  }

  if (!card || card.state === "green") {
    return (
      <section className="px-6 pb-6">
        <div className="rounded-2xl bg-white p-6 shadow-card ring-1 ring-emerald-600/15">
          <div className="flex items-center gap-3">
            <StateBadge state="green" />
            <div className="text-base font-semibold text-neutral-900">All quiet</div>
          </div>
          <p className="mt-2 text-sm text-neutral-600">
            {card?.headline ??
              "No causal chains, no silent services, dedup is working. Aegis is watching for first-fire patterns."}
          </p>
        </div>
      </section>
    );
  }

  // Orange or red — show the full card.
  const isRed = card.state === "red";
  return (
    <section className="px-6 pb-6">
      <div
        className={
          isRed
            ? "rounded-2xl bg-white p-6 shadow-hero ring-1 ring-rose-500/30"
            : "rounded-2xl bg-white p-6 shadow-hero-amber ring-1 ring-amber-500/30"
        }
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <StateBadge state={card.state} />
            <div className={EYEBROW}>Decision card</div>
          </div>
          <button
            type="button"
            onClick={() => setShowRaw((v) => !v)}
            className="font-mono text-[11px] text-neutral-400 transition hover:text-neutral-700"
          >
            {showRaw ? "hide raw" : "show raw"}
          </button>
        </div>

        <h2 className="mt-3 text-2xl font-semibold tracking-tight text-neutral-900">
          {card.root_cause_service ?? "Active incident"}
        </h2>
        <p className="mt-2 text-[15px] leading-relaxed text-neutral-700">{card.headline}</p>

        {card.business_impact && (
          <p className="mt-3 rounded-lg bg-neutral-50 px-3.5 py-2.5 text-xs text-neutral-600 ring-1 ring-neutral-200/70">
            <span className="text-neutral-400">why this matters: </span>
            {card.business_impact}
          </p>
        )}

        <div className="mt-5">
          <div className={EYEBROW}>Suggested next step</div>
          <p className="mt-1.5 text-sm leading-relaxed text-neutral-700">
            {card.suggested_next_step}
          </p>
        </div>

        {card.similar_incidents.length > 0 && (
          <div className="mt-5">
            <div className={EYEBROW}>Similar past incidents</div>
            <div className="mt-2 grid gap-2.5 md:grid-cols-2">
              {card.similar_incidents.slice(0, 4).map((m) => (
                <Match key={m.incident_id} match={m} />
              ))}
            </div>
          </div>
        )}

        <div className="mt-6 flex flex-wrap gap-2.5">
          <button
            type="button"
            disabled={busy}
            onClick={onAcknowledge}
            className="rounded-xl bg-[#0071e3] px-4 py-2 text-sm font-semibold text-white shadow-soft transition hover:bg-[#0058b9] disabled:cursor-not-allowed disabled:bg-neutral-300"
          >
            I'm on it
          </button>
          <button
            type="button"
            onClick={onShowMore}
            className="rounded-xl bg-white px-4 py-2 text-sm font-medium text-neutral-700 ring-1 ring-neutral-300 transition hover:bg-neutral-50"
          >
            Show me more past incidents
          </button>
          <button
            type="button"
            onClick={onDifferent}
            className="rounded-xl bg-white px-4 py-2 text-sm font-medium text-neutral-700 ring-1 ring-neutral-300 transition hover:bg-neutral-50"
          >
            This looks different
          </button>
        </div>

        {showRaw && (
          <pre className="mt-5 overflow-x-auto rounded-xl bg-[#1d1d1f] p-4 font-mono text-[11px] leading-relaxed text-neutral-300">
            {JSON.stringify(card, null, 2)}
          </pre>
        )}
      </div>
    </section>
  );
}
