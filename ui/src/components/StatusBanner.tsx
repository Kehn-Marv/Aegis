import type { GatewayStatus } from "../api";

interface Props {
  status: GatewayStatus | null;
  reachable: boolean;
}

export function StatusBanner({ status, reachable }: Props) {
  const online = reachable && (status?.online ?? false);
  const stateColor = !reachable
    ? "bg-rose-500/15 text-rose-300 ring-rose-500/30"
    : online
      ? "bg-emerald-500/15 text-emerald-300 ring-emerald-500/30"
      : "bg-amber-500/15 text-amber-300 ring-amber-500/30";
  const stateLabel = !reachable ? "UNREACHABLE" : online ? "ONLINE" : "OFFLINE";

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
            Edge Telemetry Gateway
          </div>
        </div>
      </div>
      <div className="flex items-center gap-6">
        <div className="text-right">
          <div className="text-[11px] uppercase tracking-widest text-slate-400">
            MCP &amp; API
          </div>
          <div className="font-mono text-xs text-slate-300">
            http://127.0.0.1:7321
          </div>
        </div>
        <div
          className={`flex items-center gap-2 rounded-full px-3 py-1 text-xs font-medium ring-1 ${stateColor}`}
        >
          <span
            className={`inline-block h-2 w-2 rounded-full ${
              online
                ? "bg-emerald-400 animate-pulse"
                : reachable
                  ? "bg-amber-400"
                  : "bg-rose-400"
            }`}
          />
          {stateLabel}
        </div>
      </div>
    </header>
  );
}
