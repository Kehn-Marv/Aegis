import type { GatewayStatus, HealthState } from "../api";

interface Props {
  status: GatewayStatus | null;
  reachable: boolean;
}

const STATE_COPY: Record<HealthState, { label: string; ledClass: string }> = {
  green: { label: "All quiet", ledClass: "led led-green" },
  orange: { label: "Trending bad", ledClass: "led led-amber" },
  red: { label: "Incident active", ledClass: "led led-red" },
};

const UNREACHABLE = { label: "Unreachable", ledClass: "led led-off" };

export function StatusBanner({ status, reachable }: Props) {
  const visual = !reachable ? UNREACHABLE : STATE_COPY[status?.state ?? "green"];

  return (
    <header className="console-card mx-4 mt-4 flex items-center justify-between !rounded-lg !py-3 !px-5">
      {/* Brand */}
      <div className="flex items-center gap-3">
        <div className="btn-primary flex h-10 w-10 items-center justify-center !rounded-lg !p-0 text-lg">
          ▲
        </div>
        <div className="leading-tight">
          <div className="text-sm font-black uppercase tracking-[3px] text-[#3D3520]">
            Aegis
          </div>
          <div className="mt-0.5 text-[10px] text-[#6A6245]">
            Control Panel · v0.2.0
          </div>
        </div>
      </div>

      {/* Nav links */}
      <nav className="hidden items-center gap-6 md:flex">
        {["Dashboard", "Memory", "Tools"].map((item) => (
          <span
            key={item}
            className="cursor-default text-[10px] font-bold uppercase tracking-[1.4px] text-[#6A6245] transition-colors hover:text-[#3D3520]"
          >
            {item}
          </span>
        ))}
      </nav>

      {/* Status */}
      <div className="flex items-center gap-4">
        {status && (
          <div className="hidden text-right md:block">
            <div className="text-[9px] font-bold uppercase tracking-[1.4px] text-[#6A6245]">
              Memory
            </div>
            <div className="font-mono text-[11px] text-[#3D3520]">
              {status.incidents_remembered} incident
              {status.incidents_remembered === 1 ? "" : "s"}
            </div>
          </div>
        )}
        <div className="badge badge-muted flex items-center gap-2">
          <span className={visual.ledClass} />
          <span className="text-[10px] font-black uppercase tracking-[1px]">
            {visual.label}
          </span>
        </div>
      </div>
    </header>
  );
}
