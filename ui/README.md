# Aegis Control Panel UI

A single-page React + TypeScript app built around the **decision card**.
When state is green the page is quiet. When orange or red, the card
takes over with the root cause, similar past fix, business impact, and
three buttons (`I'm on it`, `Show me more past incidents`, `This looks
different`).

## Run

The Aegis daemon needs to be running first:

```powershell
# In the repo root:
cargo run --bin aegis-daemon -- --config configs\aegis.demo.toml
```

Then in a separate terminal:

```powershell
cd ui
npm install            # first time only  -  about a minute
npm run dev            # http://localhost:5173
```

Vite proxies `/api/*` and `/mcp/*` to the daemon at `127.0.0.1:7321`,
so the browser sees same-origin requests and you don't need to touch
CORS.

## What you see, top to bottom

* **Health badge**  -  green / orange / red, mirrors the gateway's
  `state` field. Shows "Memory: N incidents remembered" on the right.
* **Decision card**  -  the hero. Hidden unless state is orange or red.
  Carries:
    * headline ("payment-api broke first. checkout followed 4s later. …")
    * business impact line
    * suggested next step (prefers a past fix when one is recorded)
    * top similar past incidents  -  each with cause + fix when set
    * three buttons (none of which mutate production)
* **KPI tiles**  -  noise stopped %, queue depth, incidents remembered.
* **Incident memory panel**  -  every fingerprint Aegis knows about,
  filterable by resolved status. Click `Resolve` on any unresolved
  entry to enter a 2-line cause + fix  -  that text becomes the past
  fix on the next similar chain.
* **Advanced tools**  -  collapsed by default. Bounded-window
  diagnostic / override / reset for operators who want them. None of
  these reach into production; they only change what Aegis reports.
* **Activity log**  -  every command and decision-ack the UI sent,
  with latency and outcome.

## Build

```powershell
npm run build
```

Outputs to `dist/`. Serve it behind any static file server pointed at
the daemon. For the demo, `npm run dev` is sufficient.

## API surface used

```text
GET  /api/status                       # full snapshot, polled every 2s
GET  /api/incidents?limit=20           # incident memory list, polled every 5s
POST /api/decision/ack                 # "I'm on it"
POST /api/incidents/{id}/resolve       # submit resolution card
POST /api/command                      # advanced tools (diagnostic / override / reset)
```

See [`../docs/decision-card.md`](../docs/decision-card.md) for the
shape of `GET /api/decision` and what each field means.
