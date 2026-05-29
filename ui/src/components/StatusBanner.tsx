import type { GatewayStatus, HealthState } from "../api";

interface Props {
  status: GatewayStatus | null;
  reachable: boolean;
}

const STATE_COPY: Record<HealthState, { label: string; tone: string; dot: string }> = {
  green: {
    label: "All quiet",
    tone: "bg-emerald-50 text-emerald-700 ring-emerald-600/20",
    dot: "bg-emerald-500",
  },
  orange: {
    label: "Trending bad",
    tone: "bg-amber-50 text-amber-700 ring-amber-600/20",
    dot: "bg-amber-500 animate-pulse",
  },
  red: {
    label: "Incident active",
    tone: "bg-rose-50 text-rose-700 ring-rose-600/20",
    dot: "bg-rose-500 animate-pulse",
  },
};

const UNREACHABLE = {
  label: "Unreachable",
  tone: "bg-neutral-100 text-neutral-600 ring-neutral-300",
  dot: "bg-neutral-400",
};

export function StatusBanner({ status, reachable }: Props) {
  const visual = !reachable ? UNREACHABLE : STATE_COPY[status?.state ?? "green"];

  return (
    <header className="flex items-center justify-between px-6 py-5">
      <div className="flex items-center gap-3.5">
        <div className="flex h-11 w-11 items-center justify-center rounded-[13px] bg-gradient-to-br from-[#0a84ff] to-[#5e5ce6] text-lg font-bold text-white shadow-[0_8px_22px_-8px_rgba(10,132,255,0.7)]">
          ▲
        </div>
        <div className="leading-tight">
          <div className="text-[19px] font-semibold tracking-tight text-neutral-900">
            Aegis
          </div>
          <div className="mt-0.5 text-[13px] text-neutral-500">
            Stop the noise · Find what broke first · Remember every fix
          </div>
        </div>
      </div>
      <div className="flex items-center gap-5">
        <div
          className={`flex items-center gap-2 rounded-full px-3.5 py-1.5 text-[13px] font-semibold shadow-soft ring-1 ${visual.tone}`}
        >
          <span className={`inline-block h-2 w-2 rounded-full ${visual.dot}`} />
          {visual.label}
        </div>
        {status && (
          <div className="hidden text-right sm:block">
            <div className="text-[11px] font-medium uppercase tracking-wider text-neutral-400">
              Memory
            </div>
            <div className="font-mono text-xs text-neutral-600">
              {status.incidents_remembered} incident
              {status.incidents_remembered === 1 ? "" : "s"} remembered
            </div>
          </div>
        )}
      </div>
    </header>
  );
}
