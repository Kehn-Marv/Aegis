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
  const accentColor = {
    good: "#44C464",
    warn: "#D4C020",
    bad: "#D43020",
    neutral: "#E87C14",
  }[accent];

  return (
    <div className="console-card flex flex-col gap-3">
      <div className="text-[9px] font-black uppercase tracking-[1.4px] text-[#6A6245]">
        {label}
      </div>
      <div className="lcd-panel flex items-baseline gap-2">
        <span
          className="font-mono text-3xl font-black tabular-nums tracking-tight"
          style={{ color: accentColor }}
        >
          {value}
        </span>
      </div>
      {sublabel && (
        <div className="text-[10px] text-[#6A6245]">{sublabel}</div>
      )}
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
    ? status.dedup_savings_pct > 80 ? "good"
      : status.dedup_savings_pct > 40 ? "warn"
      : "neutral"
    : "neutral";

  return (
    <section className="grid grid-cols-1 gap-4 px-4 py-2 md:grid-cols-3">
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
        accent={status ? (status.queue_depth === 0 ? "good" : status.queue_depth < 1_000 ? "warn" : "bad") : "neutral"}
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
