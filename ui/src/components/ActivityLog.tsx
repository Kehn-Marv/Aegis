export interface LogEntry {
  id: number;
  ts: number;
  direction: "out" | "in";
  label: string;
  detail: string;
  ok: boolean;
  latency_ms: number;
}

interface Props {
  entries: LogEntry[];
}

function formatTime(ts: number) {
  const d = new Date(ts);
  return (
    d.toTimeString().slice(0, 8) + "." +
    String(d.getMilliseconds()).padStart(3, "0")
  );
}

export function ActivityLog({ entries }: Props) {
  return (
    <section className="px-6 pb-8">
      <div className="rounded-lg border border-slate-800/80 bg-slate-900/40 p-5">
        <div className="flex items-center justify-between">
          <div className="text-[11px] uppercase tracking-widest text-slate-400">
            Activity
          </div>
          <div className="text-[11px] font-mono text-slate-500">
            {entries.length} event{entries.length === 1 ? "" : "s"}
          </div>
        </div>
        <div className="mt-3 max-h-72 overflow-y-auto font-mono text-xs leading-6 text-slate-300">
          {entries.length === 0 && (
            <div className="text-slate-500">
              Waiting for activity — status polls every 2 seconds.
            </div>
          )}
          {entries.map((e) => (
            <div key={e.id} className="flex items-center gap-3 py-0.5">
              <span className="text-slate-500">{formatTime(e.ts)}</span>
              <span
                className={
                  e.direction === "out"
                    ? "text-emerald-400"
                    : e.ok
                      ? "text-slate-400"
                      : "text-rose-400"
                }
              >
                {e.direction === "out" ? "→" : "←"}
              </span>
              <span className="text-slate-200">{e.label}</span>
              <span className="text-slate-500 truncate">{e.detail}</span>
              <span className="ml-auto text-slate-600">
                {e.latency_ms.toFixed(0)}ms
              </span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
