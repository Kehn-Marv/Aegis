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
  return d.toTimeString().slice(0, 8) + "." + String(d.getMilliseconds()).padStart(3, "0");
}

export function ActivityLog({ entries }: Props) {
  return (
    <section>
      <div className="console-card">
        <div className="flex items-center justify-between mb-3">
          <div className="eyebrow">Activity</div>
          <div className="font-mono text-[10px] text-[#8a8470]">
            {entries.length} event{entries.length === 1 ? "" : "s"}
          </div>
        </div>
        <div className="lcd-panel max-h-72 overflow-y-auto font-mono text-xs leading-6">
          {entries.length === 0 && (
            <div className="text-[rgba(255,250,218,0.3)]">
              Waiting for activity…
            </div>
          )}
          {entries.map((e) => (
            <div key={e.id} className="flex items-center gap-3 py-0.5">
              <span className="text-[rgba(255,250,218,0.25)]">{formatTime(e.ts)}</span>
              <span
                style={{
                  color: e.direction === "out" ? "#e07818"
                    : e.ok ? "#fffada" : "#c83020",
                }}
              >
                {e.direction === "out" ? "→" : "←"}
              </span>
              <span className="text-[#fffada]">{e.label}</span>
              <span className="truncate text-[rgba(255,250,218,0.35)]">{e.detail}</span>
              <span className="ml-auto text-[rgba(255,250,218,0.18)]">
                {e.latency_ms.toFixed(0)}ms
              </span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
