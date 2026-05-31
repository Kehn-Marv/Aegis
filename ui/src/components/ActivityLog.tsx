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
    <section className="px-4 pb-4">
      <div className="console-card">
        <div className="flex items-center justify-between">
          <div className="eyebrow">Activity</div>
          <div className="font-mono text-[10px] text-[#6A6245]">
            {entries.length} event{entries.length === 1 ? "" : "s"}
          </div>
        </div>
        <div className="lcd-panel mt-3 max-h-72 overflow-y-auto font-mono text-xs leading-6">
          {entries.length === 0 && (
            <div className="text-[rgba(255,250,218,0.35)]">
              Waiting for activity — status polls every 2 seconds.
            </div>
          )}
          {entries.map((e) => (
            <div key={e.id} className="flex items-center gap-3 py-0.5">
              <span className="text-[rgba(255,250,218,0.3)]">{formatTime(e.ts)}</span>
              <span
                style={{
                  color:
                    e.direction === "out"
                      ? "#E87C14"
                      : e.ok
                        ? "#44C464"
                        : "#D43020",
                }}
              >
                {e.direction === "out" ? "→" : "←"}
              </span>
              <span className="text-[#FFFADA]">{e.label}</span>
              <span className="truncate text-[rgba(255,250,218,0.4)]">{e.detail}</span>
              <span className="ml-auto text-[rgba(255,250,218,0.2)]">
                {e.latency_ms.toFixed(0)}ms
              </span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
