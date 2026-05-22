import type { GatewayStatus } from "../api";

interface Props {
  status: GatewayStatus | null;
}

interface TileProps {
  label: string;
  value: string;
  sublabel?: string;
  accent?: "good" | "warn" | "bad" | "neutral";
}

function Tile({ label, value, sublabel, accent = "neutral" }: TileProps) {
  const accentClass = {
    good: "text-emerald-300",
    warn: "text-amber-300",
    bad: "text-rose-300",
    neutral: "text-slate-100",
  }[accent];
  return (
    <div className="rounded-lg border border-slate-800/80 bg-slate-900/40 p-5 backdrop-blur">
      <div className="text-[11px] uppercase tracking-widest text-slate-400">
        {label}
      </div>
      <div
        className={`mt-3 font-mono text-3xl font-semibold tabular-nums ${accentClass}`}
      >
        {value}
      </div>
      {sublabel && (
        <div className="mt-1 text-xs text-slate-500">{sublabel}</div>
      )}
    </div>
  );
}

const NUMFMT = new Intl.NumberFormat("en-US");

export function StatusTiles({ status }: Props) {
  const dedupPct = status ? status.dedup_savings_pct.toFixed(2) + "%" : "—";
  const eventsIn = status ? NUMFMT.format(status.events_in) : "—";
  const eventsOut = status ? NUMFMT.format(status.events_out) : "—";
  const queue = status ? NUMFMT.format(status.queue_depth) : "—";
  const sigs = status ? NUMFMT.format(status.unique_signatures) : "—";

  const dedupAccent: TileProps["accent"] = status
    ? status.dedup_savings_pct > 80
      ? "good"
      : status.dedup_savings_pct > 40
        ? "warn"
        : "neutral"
    : "neutral";
  const queueAccent: TileProps["accent"] = status
    ? status.queue_depth === 0
      ? "good"
      : status.queue_depth < 1_000
        ? "warn"
        : "bad"
    : "neutral";

  return (
    <section className="grid grid-cols-1 gap-4 px-6 py-6 md:grid-cols-3">
      <Tile
        label="Dedup Savings"
        value={dedupPct}
        sublabel={status ? `${eventsIn} in → ${eventsOut} out` : "waiting…"}
        accent={dedupAccent}
      />
      <Tile
        label="Queue Depth"
        value={queue}
        sublabel="buffered events waiting for HEC"
        accent={queueAccent}
      />
      <Tile
        label="Unique Signatures"
        value={sigs}
        sublabel={
          status
            ? `${status.override_active ? "override active · " : ""}${
                status.diagnostic_active ? "diagnostic active" : ""
              }` || "open in current dedup window"
            : "—"
        }
      />
    </section>
  );
}
