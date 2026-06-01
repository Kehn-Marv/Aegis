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
    <header className="console-card flex items-center justify-between gap-4 !rounded-xl !py-4 !px-6">
      {/* Brand — single clean wordmark, owns its space */}
      <a
        href="#top"
        onClick={(e) => {
          e.preventDefault();
          window.scrollTo({ top: 0, behavior: "smooth" });
        }}
        className="group flex items-center no-underline"
      >
        <span className="text-[24px] font-black uppercase leading-none tracking-[7px] text-[#2e2a1e] transition-[letter-spacing] duration-300 group-hover:tracking-[8px]">
          Aegis
        </span>
      </a>

      {/* Status */}
      <div className="flex items-center gap-3">
        <div className="badge badge-muted flex items-center gap-2">
          <span className={visual.ledClass} />
          <span>{visual.label}</span>
        </div>
      </div>
    </header>
  );
}
