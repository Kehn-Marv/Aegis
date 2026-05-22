# Aegis Control Panel UI

Single-page React + TypeScript app that talks to the Aegis daemon's REST
API (`/api/status`, `/api/command`) and shows live gateway state.

## Run

The Aegis daemon needs to be running first (the UI consumes it):

```powershell
cd ..\
cargo run --bin aegis-daemon
```

Then in a separate terminal:

```powershell
cd ui
npm install
npm run dev
```

Open <http://localhost:5173>. Vite proxies `/api/*` and `/mcp/*` through
to the daemon at `127.0.0.1:7321`, so the browser sees same-origin
requests and you don't need to touch CORS.

## What it shows

* **Dedup Savings / Queue Depth / Unique Signatures** — three live KPI tiles
  polled every 2 seconds from `/api/status`.
* **Network Panel** — Online/Offline toggle that flips the gateway's
  reported uplink (useful for demoing the priority-queue drain behaviour).
* **Remote MCP Command** — dropdown + duration input that posts to
  `/api/command`. The same commands the MCP server exposes — but as
  plain REST so a browser doesn't need to speak MCP.
* **Activity Log** — every command sent shows up here with latency and
  outcome. Status polls are silent.

## Build

```powershell
npm run build
```

Outputs to `dist/`. In production you'd serve this from any static
server pointed at the daemon. For the hackathon demo, `npm run dev` is
sufficient.
