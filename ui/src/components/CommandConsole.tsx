import { useState } from "react";

interface Props {
  onSend: (command: string, seconds?: number) => Promise<void>;
  busy: boolean;
}

const COMMANDS = [
  {
    id: "status",
    label: "Status",
    description: "Read the gateway snapshot",
    durationLabel: null,
  },
  {
    id: "reset",
    label: "Reset",
    description: "Clear counters and the priority queue",
    durationLabel: null,
  },
  {
    id: "diagnostic",
    label: "Diagnostic",
    description: "Enable verbose edge tracing for N seconds",
    durationLabel: "seconds",
  },
  {
    id: "override",
    label: "Override",
    description: "Bypass dedup, stream raw lines to HEC for N seconds",
    durationLabel: "seconds",
  },
] as const;

type CommandId = (typeof COMMANDS)[number]["id"];

export function CommandConsole({ onSend, busy }: Props) {
  const [command, setCommand] = useState<CommandId>("status");
  const [seconds, setSeconds] = useState<number>(30);
  const cmd = COMMANDS.find((c) => c.id === command)!;
  const needsDuration = cmd.durationLabel !== null;

  const handle = async (e: React.FormEvent) => {
    e.preventDefault();
    await onSend(command, needsDuration ? seconds : undefined);
  };

  return (
    <section className="px-6 pb-6">
      <div className="rounded-lg border border-slate-800/80 bg-slate-900/40 p-5">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-widest text-slate-400">
              Remote MCP Command
            </div>
            <div className="mt-1 text-sm text-slate-300">{cmd.description}</div>
          </div>
          <div className="text-[11px] font-mono text-slate-500">
            POST /api/command
          </div>
        </div>

        <form onSubmit={handle} className="mt-5 flex flex-wrap items-end gap-3">
          <label className="flex flex-col gap-1">
            <span className="text-[11px] uppercase tracking-widest text-slate-500">
              Command
            </span>
            <select
              value={command}
              onChange={(e) => setCommand(e.target.value as CommandId)}
              className="rounded-md border border-slate-800 bg-slate-950 px-3 py-2 font-mono text-sm text-slate-100 focus:border-emerald-500/60 focus:outline-none"
            >
              {COMMANDS.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.label}
                </option>
              ))}
            </select>
          </label>

          {needsDuration && (
            <label className="flex flex-col gap-1">
              <span className="text-[11px] uppercase tracking-widest text-slate-500">
                Duration ({cmd.durationLabel})
              </span>
              <input
                type="number"
                min={1}
                max={3600}
                value={seconds}
                onChange={(e) => setSeconds(Number(e.target.value) || 30)}
                className="w-28 rounded-md border border-slate-800 bg-slate-950 px-3 py-2 font-mono text-sm text-slate-100 focus:border-emerald-500/60 focus:outline-none"
              />
            </label>
          )}

          <button
            type="submit"
            disabled={busy}
            className="rounded-md bg-emerald-500/90 px-4 py-2 text-sm font-medium text-slate-950 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400"
          >
            {busy ? "Sending…" : "Send to Aegis Gateway"}
          </button>
        </form>
      </div>
    </section>
  );
}
