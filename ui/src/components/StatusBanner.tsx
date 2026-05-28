import type { GatewayStatus, HealthState } from "../api";

interface Props {
  status: GatewayStatus | null;
  reachable: boolean;
}

const STATE_COPY: Record<HealthState, { label: string; tone: string; dot: string }> = {
  green: {
    label: "GREEN — all quiet",
    tone: "bg-emerald-500/15 text-emerald-300 ring-emerald-500/30",
    dot: "bg-emerald-400 animate-pulse",
  },
  orange: {
    label: "ORANGE — trending bad",
    tone: "bg-amber-500/15 text-amber-300 ring-amber-500/30",
    dot: "bg-amber-400 animate-pulse",
  },
  red: {
    label: "RED — incident active",
    tone: "bg-rose-500/15 text-rose-300 ring-rose-500/30",
    dot: "bg-rose-400 animate-pulse",
  },
};

const UNREACHABLE = {
  label: "UNREACHABLE",
  tone: "bg-slate-700/30 text-slate-300 ring-slate-700/50",
  dot: "bg-slate-400",
};

export function StatusBanner({ status, reachable }: Props) {
  const visual = !reachable
    ? UNREACHABLE
    : STATE_COPY[status?.state ?? "green"];

  return (
    <header className="flex items-center justify-between border-b border-slate-800/80 px-6 py-4">
      <div className="flex items-center gap-3">
        <div className="flex h-9 w-9 items-center justify-center rounded-md bg-emerald-500/15 ring-1 ring-emerald-500/30">
          <span className="text-emerald-300 text-base">▲</span>
        </div>
        <div className="leading-tight">
          <div className="text-sm font-semibold tracking-tight text-slate-100">
            Aegis
          </div>
          <div className="text-[11px] uppercase tracking-widest text-slate-400">
            Stop the noise · Find what broke first · Remember every fix
          </div>
        </div>
      </div>
      <div className="flex items-center gap-6">
        <div
          className={`flex items-center gap-2 rounded-full px-3 py-1 text-xs font-semibold ring-1 ${visual.tone}`}
        >
          <span className={`inline-block h-2 w-2 rounded-full ${visual.dot}`} />
          {visual.label}
        </div>
        {status && (
          <div className="hidden sm:block text-right">
            <div className="text-[11px] uppercase tracking-widest text-slate-400">
              Memory
            </div>
            <div className="font-mono text-xs text-slate-300">
              {status.incidents_remembered} incident
              {status.incidents_remembered === 1 ? "" : "s"} remembered
            </div>
          </div>
        )}
      </div>
    </header>
  );
}
