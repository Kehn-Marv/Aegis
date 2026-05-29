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
    good: "text-emerald-600",
    warn: "text-amber-600",
    bad: "text-rose-600",
    neutral: "text-neutral-900",
  }[accent];
  return (
    <div className="rounded-2xl bg-white p-5 shadow-card ring-1 ring-neutral-200/60">
      <div className="text-[11px] font-medium uppercase tracking-wider text-neutral-400">
        {label}
      </div>
      <div className={`mt-3 font-mono text-3xl font-semibold tabular-nums tracking-tight ${accentClass}`}>
        {value}
      </div>
      {sublabel && <div className="mt-1 text-xs text-neutral-500">{sublabel}</div>}
    </div>
  );
}

const NUMFMT = new Intl.NumberFormat("en-US");

export function StatusTiles({ status }: Props) {
  const dedupPct = status ? status.dedup_savings_pct.toFixed(1) + "%" : "—";
  const eventsIn = status ? NUMFMT.format(status.events_in) : "—";
  const eventsOut = status ? NUMFMT.format(status.events_out) : "—";
  const queue = status ? NUMFMT.format(status.queue_depth) : "—";
  const memory = status ? NUMFMT.format(status.incidents_remembered) : "—";

  const dedupAccent: TileProps["accent"] = status
    ? status.dedup_savings_pct > 80
      ? "good"
      : status.dedup_savings_pct > 40
        ? "warn"
        : "neutral"
    : "neutral";

  return (
    <section className="grid grid-cols-1 gap-4 px-6 py-2 md:grid-cols-3">
      <Tile
        label="Noise Stopped"
        value={dedupPct}
        sublabel={status ? `${eventsIn} in → ${eventsOut} out` : "waiting…"}
        accent={dedupAccent}
      />
      <Tile
        label="Queue Depth"
        value={queue}
        sublabel="events waiting for Splunk"
        accent={
          status
            ? status.queue_depth === 0
              ? "good"
              : status.queue_depth < 1_000
                ? "warn"
                : "bad"
            : "neutral"
        }
      />
      <Tile
        label="Incidents Remembered"
        value={memory}
        sublabel="fingerprints in local memory"
        accent={status && status.incidents_remembered > 0 ? "good" : "neutral"}
      />
    </section>
  );
}
