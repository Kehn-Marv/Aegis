import type { GatewayStatus } from "../api";

interface Props {
  status: GatewayStatus | null;
}

interface TileProps {
  label: string;
  value: string;
  sublabel?: string;
}

function Tile({ label, value, sublabel }: TileProps) {
  return (
    <div className="console-card flex flex-col gap-3">
      <div className="flex items-center gap-2">
        <span className="text-[9px] font-bold uppercase tracking-[1.4px] text-[#5a5440]">
          {label}
        </span>
      </div>
      <div className="lcd-panel scan-sweep flex items-baseline">
        <span className="font-mono text-[30px] font-black tabular-nums leading-none tracking-tight text-[#e07818]">
          {value}
        </span>
      </div>
      {sublabel && (
        <div className="text-[10px] font-medium text-[#8a8470]">{sublabel}</div>
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

  return (
    <section className="grid grid-cols-1 gap-4 md:grid-cols-3">
      <Tile
        label="Noise Stopped"
        value={dedupPct}
        sublabel={status ? `${eventsIn} in → ${eventsOut} out` : "waiting…"}
      />
      <Tile
        label="Queue Depth"
        value={queue}
        sublabel="events waiting for Splunk"
      />
      <Tile
        label="Incidents Remembered"
        value={memory}
        sublabel="fingerprints in local memory"
      />
    </section>
  );
}
