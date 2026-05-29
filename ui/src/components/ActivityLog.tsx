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
    d.toTimeString().slice(0, 8) + "." + String(d.getMilliseconds()).padStart(3, "0")
  );
}

export function ActivityLog({ entries }: Props) {
  return (
    <section className="px-6 pb-6">
      <div className="rounded-2xl bg-white p-5 shadow-card ring-1 ring-neutral-200/60">
        <div className="flex items-center justify-between">
          <div className="text-[11px] font-medium uppercase tracking-wider text-neutral-400">
            Activity
          </div>
          <div className="font-mono text-[11px] text-neutral-400">
            {entries.length} event{entries.length === 1 ? "" : "s"}
          </div>
        </div>
        <div className="mt-3 max-h-72 overflow-y-auto font-mono text-xs leading-6 text-neutral-600">
          {entries.length === 0 && (
            <div className="text-neutral-400">
              Waiting for activity — status polls every 2 seconds.
            </div>
          )}
          {entries.map((e) => (
            <div key={e.id} className="flex items-center gap-3 py-0.5">
              <span className="text-neutral-400">{formatTime(e.ts)}</span>
              <span
                className={
                  e.direction === "out"
                    ? "text-[#0071e3]"
                    : e.ok
                      ? "text-neutral-400"
                      : "text-rose-500"
                }
              >
                {e.direction === "out" ? "→" : "←"}
              </span>
              <span className="text-neutral-800">{e.label}</span>
              <span className="truncate text-neutral-400">{e.detail}</span>
              <span className="ml-auto text-neutral-300">{e.latency_ms.toFixed(0)}ms</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
