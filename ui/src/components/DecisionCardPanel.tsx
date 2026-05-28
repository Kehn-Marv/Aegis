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
    <div className="rounded border border-slate-800/80 bg-slate-950/40 p-3">
      <div className="flex items-center justify-between text-xs">
        <div className="font-mono text-emerald-300">{sim}% similar</div>
        <div className="text-slate-500">{ago}</div>
      </div>
      <div className="mt-1 text-xs text-slate-400">
        root cause: <span className="text-slate-200">{match.past_root_cause_service}</span>
        {match.past_resolved_in_minutes != null && (
          <> · fixed in {match.past_resolved_in_minutes} min</>
        )}
      </div>
      {match.past_cause ? (
        <div className="mt-2 text-xs text-slate-300">
          <span className="text-slate-500">cause:</span> {match.past_cause}
        </div>
      ) : (
        <div className="mt-2 text-xs italic text-slate-500">
          No resolution recorded — fix this one and write it down so the next
          on-call has a head start.
        </div>
      )}
      {match.past_fix && (
        <div className="mt-1 text-xs text-slate-300">
          <span className="text-slate-500">fix:</span> {match.past_fix}
        </div>
      )}
    </div>
  );
}

function StateBadge({ state }: { state: DecisionCard["state"] }) {
  const tone = {
    green: "bg-emerald-500/15 text-emerald-300 ring-emerald-500/30",
    orange: "bg-amber-500/15 text-amber-300 ring-amber-500/30",
    red: "bg-rose-500/15 text-rose-300 ring-rose-500/30",
  }[state];
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-widest ring-1 ${tone}`}
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
        <div className="rounded-lg border border-rose-500/30 bg-rose-500/5 p-6 text-sm text-rose-200">
          Gateway unreachable. Make sure <span className="font-mono">aegis-daemon</span>{" "}
          is running on <span className="font-mono">127.0.0.1:7321</span>.
        </div>
      </section>
    );
  }

  if (!card || card.state === "green") {
    return (
      <section className="px-6 pb-6">
        <div className="rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-6">
          <div className="flex items-center gap-3">
            <StateBadge state="green" />
            <div className="text-base font-semibold text-emerald-100">
              All quiet
            </div>
          </div>
          <p className="mt-2 text-sm text-slate-300">
            {card?.headline ??
              "No causal chains, no silent services, dedup is working. Aegis is watching for first-fire patterns."}
          </p>
        </div>
      </section>
    );
  }

  // Orange or red — show the full card.
  return (
    <section className="px-6 pb-6">
      <div
        className={
          card.state === "red"
            ? "rounded-lg border border-rose-500/40 bg-rose-500/5 p-6 shadow-[0_0_60px_-30px_rgba(244,63,94,0.7)]"
            : "rounded-lg border border-amber-500/40 bg-amber-500/5 p-6"
        }
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <StateBadge state={card.state} />
            <div className="text-[11px] uppercase tracking-widest text-slate-400">
              Decision card
            </div>
          </div>
          <button
            type="button"
            onClick={() => setShowRaw((v) => !v)}
            className="text-[11px] font-mono text-slate-500 hover:text-slate-300"
          >
            {showRaw ? "hide raw" : "show raw"}
          </button>
        </div>

        <h2 className="mt-3 text-xl font-semibold text-slate-100">
          {card.root_cause_service ?? "Active incident"}
        </h2>
        <p className="mt-2 text-sm text-slate-200">{card.headline}</p>

        {card.business_impact && (
          <p className="mt-3 rounded bg-slate-900/60 px-3 py-2 text-xs text-slate-300">
            <span className="text-slate-500">why this matters: </span>
            {card.business_impact}
          </p>
        )}

        <div className="mt-4">
          <div className="text-[11px] uppercase tracking-widest text-slate-400">
            Suggested next step
          </div>
          <p className="mt-1 text-sm text-slate-200">
            {card.suggested_next_step}
          </p>
        </div>

        {card.similar_incidents.length > 0 && (
          <div className="mt-5">
            <div className="text-[11px] uppercase tracking-widest text-slate-400">
              Similar past incidents
            </div>
            <div className="mt-2 grid gap-2 md:grid-cols-2">
              {card.similar_incidents.slice(0, 4).map((m) => (
                <Match key={m.incident_id} match={m} />
              ))}
            </div>
          </div>
        )}

        <div className="mt-6 flex flex-wrap gap-2">
          <button
            type="button"
            disabled={busy}
            onClick={onAcknowledge}
            className="rounded-md bg-emerald-500/90 px-4 py-2 text-sm font-semibold text-slate-950 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400"
          >
            I'm on it
          </button>
          <button
            type="button"
            onClick={onShowMore}
            className="rounded-md border border-slate-700 px-4 py-2 text-sm font-medium text-slate-200 hover:border-slate-500 hover:bg-slate-800"
          >
            Show me more past incidents
          </button>
          <button
            type="button"
            onClick={onDifferent}
            className="rounded-md border border-slate-700 px-4 py-2 text-sm font-medium text-slate-200 hover:border-slate-500 hover:bg-slate-800"
          >
            This looks different
          </button>
        </div>

        {showRaw && (
          <pre className="mt-5 overflow-x-auto rounded bg-slate-950/80 p-3 font-mono text-[11px] leading-relaxed text-slate-400">
            {JSON.stringify(card, null, 2)}
          </pre>
        )}
      </div>
    </section>
  );
}
