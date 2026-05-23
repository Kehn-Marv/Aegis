# Aegis — Architecture

> Top-level architecture diagram for judges and reviewers, satisfying the
> Splunk Agentic Ops Hackathon submission requirement that this file live
> at the root of the repository. See [`docs/architecture.md`](docs/architecture.md)
> for the detailed walkthrough.

Aegis is a **local-first, MCP-controllable observability gateway** that
sits between applications and Splunk. It deduplicates repetitive error
loops into lightweight metrics, summarizes routine traffic, buffers
everything offline with anomaly-first priority, and exposes both a real
REST control plane and an MCP server so external AI agents can inspect
and command the running edge.

```mermaid
flowchart LR
    subgraph Edge["Local Edge Environment"]
        APP["Microservices<br/>raw logs &amp; metrics"]
        subgraph Aegis["Aegis Gateway (Rust)"]
            ING["Ingest<br/>TCP &middot; UDP"]
            CORE["Processing Core<br/>signature hash &middot; dedup<br/>&middot; cache &middot; counters"]
            QUEUE[("Priority Queue<br/>SQLite, anomaly-first")]
            SINK["HEC Sink<br/>batched, retry,<br/>backoff"]
            SELFM["Self-metrics<br/>emitter"]
            MCPSRV["MCP HTTP Server<br/>+ REST control API"]
        end
        SIDECAR["Python AI Sidecar<br/>MiniLM embeddings<br/>KMeans clustering<br/>hybrid classifier"]
        UI["Control Panel UI<br/>React + Vite"]
    end

    subgraph Splunk["Central Splunk Environment"]
        HEC[/"HTTP Event Collector"/]
        INDEX[("Indexed Events<br/>aegis:raw &middot; aegis:metric<br/>aegis:selfmetric &middot; aegis:agent")]
        DASH["Dashboard Studio<br/>FinOps + AI panels"]
        SMCP["Splunk MCP Server<br/>SPL search &middot; KOs"]
        SAIA["AI Assistant 2.0"]
        HOSTED["Splunk Hosted Models<br/>gpt-oss &middot; Foundation-Sec"]
    end

    AGENT["External AI Agent<br/>Cursor / Claude Desktop"]
    OPS["AegisOps Agent<br/>observe &rarr; reason &rarr; act"]
    OLLAMA["Local Ollama<br/>qwen2.5:3b<br/>(default LLM, JSON-schema enforced)"]

    APP -->|raw stream| ING
    ING --> CORE
    CORE <-->|classify per signature| SIDECAR
    CORE -->|enqueue| QUEUE
    QUEUE -->|priority drain| SINK
    SINK -->|metrics + raw + summaries| HEC
    SELFM -->|self perf telemetry| HEC
    HEC --> INDEX
    INDEX --> DASH

    UI <-->|/api/status<br/>/api/command| MCPSRV
    AGENT <-->|MCP| SMCP
    AGENT <-->|MCP| MCPSRV
    OPS -->|REST /api/*| MCPSRV
    OPS -->|reason: default| OLLAMA
    OPS -.->|reason: hibernated<br/>&#124; ai SPL| HOSTED
    OPS -->|audit HEC| HEC
    OPS -->|observational SPL| INDEX
    MCPSRV -.->|live Arc&lt;Control&gt;| CORE
    MCPSRV -.->|reset / clear| QUEUE
    SMCP --> INDEX
    SAIA <--> HOSTED
    SIDECAR -.->|&#124; ai classify<br/>(hibernated)| HOSTED

    classDef edge fill:#0b3d2e,stroke:#3ddc97,color:#fff
    classDef splunk fill:#1a1a3e,stroke:#7c5cff,color:#fff
    classDef agent fill:#3d2a0b,stroke:#ffb86b,color:#fff
    class APP,ING,CORE,QUEUE,SINK,SELFM,MCPSRV,SIDECAR,UI,OLLAMA edge
    class HEC,INDEX,DASH,SMCP,SAIA,HOSTED splunk
    class AGENT,OPS agent
```

## Three planes, one process

| Plane | What it does | Implementation |
|-------|--------------|----------------|
| **Data** | Receives raw logs, hashes signatures, collapses duplicates, summarizes routine traffic, buffers offline, drains to HEC anomaly-first | `aegis-core` (Rust) — async tokio pipeline with mpsc channels and SQLite-backed priority queue |
| **AI** | Classifies each new signature once and attaches the verdict to the eventual collapsed metric event | `sidecar/` (Python FastAPI) — `sentence-transformers/all-MiniLM-L6-v2` + sklearn KMeans + Splunk `| ai` hosted-model adapter (OpenAI-compat fallback) |
| **Control** | Exposes the same `Arc<Control>` and `Queue` handles to (a) MCP clients over streamable-HTTP, (b) a browser UI over REST, and (c) the AegisOps autonomous agent | `aegis-mcp` (Rust) + `agent/` (Python) |

All three planes run in a **single daemon process** sharing the same Arc-backed
state. When an external AI agent calls `aegis.override(seconds=30)` via MCP,
the very next iteration of the dedup loop reads `control.override_active() == true`
and switches to raw passthrough — a real agentic loop, not a fake.

## Splunk integration touchpoints

| Splunk capability | How Aegis uses it | Targeted prize |
|-------------------|-------------------|----------------|
| **HTTP Event Collector (HEC)** | Primary egress path. Four sourcetypes: `aegis:raw`, `aegis:metric`, `aegis:selfmetric`, `aegis:diagnostic` | Best of Observability |
| **MCP Server (Splunkbase, v1.1.3)** | Aegis ships a *complementary* MCP server. The demo orchestrates Cursor / Claude Desktop talking to *both* MCP servers in one chat session | Best Use of Splunk MCP Server |
| **Hosted Models** (`gpt-oss-20b`, `Foundation-Sec-1.1-8B`) | Sidecar classifies via SPL `| ai`; AegisOps agent reasons via the same transport (`transports.SplunkAITransport`). Currently **hibernated** because the 14-day Splunk Cloud trial does not provision SLIM API — see [`docs/splunk-blocker.md`](docs/splunk-blocker.md). Default transport is local Ollama running the same `gpt-oss:20b` model id. Both paths are tested; flipping the config restores the Splunk transport with no code change | Best Use of Splunk Hosted Models |
| **AI Assistant 2.0** | No programmatic API today. Documented pairing: operator asks SAIA to explain `sourcetype=aegis:agent` audit events the autonomous agent produced | Best of Observability |
| **Dashboard Studio** | [`dashboards/aegis.json`](dashboards/aegis.json) ships 9 panels covering dedup savings, top suppressed signatures, classifier verdict, classifier-strategy breakdown, and first-occurrence rate | Best of Observability |

## Data flows

* **Raw ingest** — TCP/UDP listeners feed an `mpsc<IngestLine>`.
* **Dedup** — A single async task owns the open-signature `HashMap`. First
  occurrence of any signature emits a raw event immediately; subsequent
  occurrences within the window bump a counter. On window close the
  counter becomes one collapsed metric event.
* **AI enrichment** — Per *new* signature (not per line — important for
  perf), the dedup task spawns a background task that calls the sidecar's
  `/classify`. The result lands in a classification cache keyed by
  signature and is attached to the eventual collapsed event.
* **Queue** — Every emitted event is enqueued in SQLite with a priority
  (`HIGH` for first-occurrences and override-mode raws, `MEDIUM` for
  collapsed metrics). A separate drain task pulls priority-ordered
  batches, POSTs to HEC, and marks the gateway offline on failure.
* **Self-metrics** — A timer snapshots `Control` every 15 s and emits a
  dedicated `aegis:selfmetric` event so the dashboard can show the
  gateway's own performance in real time.

## Control flows

* **Browser UI** → `GET /api/status` every 2 s, `POST /api/command` on user
  action.
* **MCP client (Cursor, Claude Desktop)** → `POST /mcp` with JSON-RPC 2.0
  over streamable HTTP. Five tools: `status`, `reset`, `diagnostic`,
  `override`, `replay_raw`.
* **AegisOps Agent** → polls each gateway's REST API, runs observational
  SPL against Splunk (optional), calls its configured **LLM transport**
  (`ollama` default, `splunk_ai` hibernated), actuates via
  `POST /api/command`, audits to `sourcetype=aegis:agent` (optional).
* Both control planes mutate the **same `Arc<Control>`** the data plane
  reads on its hot path — verified end-to-end during development (the
  README walks through the smoke test).

## File map

```
.
|-- ARCHITECTURE.md         this file (Devpost-required root architecture)
|-- README.md               quick-start with two setup paths (demo & live)
|-- LICENSE                 MIT
|-- Cargo.toml              Rust workspace manifest
|-- Cargo.lock              Reproducible Rust dependency lock
|-- rust-toolchain.toml     Pinned Rust toolchain (stable)
|-- gateway/                Rust workspace (data plane + control plane)
|   |-- aegis-core/         ingest, dedup, queue, HEC client, sidecar client
|   |-- aegis-mcp/          MCP HTTP server + REST control API
|   `-- aegis-daemon/       binary that wires core + mcp together
|-- sidecar/                Python AI sidecar (FastAPI)
|   |-- pyproject.toml      Python dependencies and entry point
|   `-- aegis_sidecar/      embeddings, clustering, classifier, splunk_ai adapter
|-- agent/                  AegisOps autonomous agent (observe → reason → act)
|   |-- pyproject.toml      Agent dependencies and CLI entry point
|   `-- aegis_ops/          observer, reasoner, transports (Ollama + Splunk |ai), policy, actuator, auditor
|-- ui/                     React 19 + Vite 7 + Tailwind v4 control panel
|   |-- package.json        Node dependencies + scripts
|   `-- src/                components + REST client
|-- dashboards/             Splunk Dashboard Studio JSON + install guide
|-- demo/                   log_spammer.py traffic generator + canned MCP/REST fixtures
|-- configs/
|   |-- aegis.example.toml  Production template (copy to aegis.toml, fill in HEC token)
|   |-- aegis.demo.toml     No-Splunk demo config (3s dedup window, stderr sink)
|   |-- aegis.us-east.*     Multi-edge templates (demo + live)
|   `-- aegis.eu-west.*
`-- docs/                   architecture deep-dive, MCP integration, FinOps math, SAIA notes
```
