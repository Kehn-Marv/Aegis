import type { GatewayStatus } from "../api";

interface Props {
  status: GatewayStatus | null;
  onSetOnline: (online: boolean) => Promise<void>;
  busy: boolean;
}

export function NetworkPanel({ status, onSetOnline, busy }: Props) {
  const online = status?.online ?? false;
  return (
    <section className="px-6 pb-6">
      <div className="rounded-lg border border-slate-800/80 bg-slate-900/40 p-5">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-widest text-slate-400">
              Network Status
            </div>
            <div className="mt-1 text-sm text-slate-300">
              Toggle the gateway's reported uplink. The HEC drain task halts
              while offline and resumes anomaly-first when you go back online.
            </div>
          </div>
          <div className="flex gap-2 rounded-md border border-slate-800 p-1">
            <button
              type="button"
              disabled={busy || online}
              onClick={() => onSetOnline(true)}
              className={`px-4 py-1.5 text-sm font-medium transition rounded ${
                online
                  ? "bg-emerald-500/20 text-emerald-300 ring-1 ring-emerald-500/30"
                  : "text-slate-300 hover:bg-slate-800"
              }`}
            >
              Online
            </button>
            <button
              type="button"
              disabled={busy || !online}
              onClick={() => onSetOnline(false)}
              className={`px-4 py-1.5 text-sm font-medium transition rounded ${
                !online
                  ? "bg-amber-500/20 text-amber-300 ring-1 ring-amber-500/30"
                  : "text-slate-300 hover:bg-slate-800"
              }`}
            >
              Offline
            </button>
          </div>
        </div>
      </div>
    </section>
  );
}
