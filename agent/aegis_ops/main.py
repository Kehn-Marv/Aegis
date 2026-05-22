"""AegisOps Agent entrypoint."""

from __future__ import annotations

import asyncio
import logging
import time
from pathlib import Path

import typer

from .actuator import Actuator
from .auditor import Auditor
from .config import AegisOpsCfg
from .gateway_client import GatewayClient
from .models import DecisionRecord
from .observer import Observer
from .policy import PolicyEngine
from .reasoner import Reasoner
from .splunk_client import SplunkClient

log = logging.getLogger("aegis_ops")
app = typer.Typer(help="Autonomous AI agent for Aegis edge gateways.")


@app.command()
def run(
    config: Path = typer.Option(
        Path("configs/aegis-ops.toml"),
        "--config",
        "-c",
        help="Path to the TOML config.",
    ),
    dry_run: bool = typer.Option(
        False,
        "--dry-run",
        help="Skip all actuation and audit writes. Useful for prompt iteration.",
    ),
    once: bool = typer.Option(
        False,
        "--once",
        help="Run a single tick and exit (smoke test).",
    ),
    verbose: bool = typer.Option(False, "--verbose", "-v"),
) -> None:
    """Run the observe → reason → act loop."""
    logging.basicConfig(
        level=logging.DEBUG if verbose else logging.INFO,
        format="%(asctime)s %(levelname)-5s %(name)s %(message)s",
    )
    cfg = AegisOpsCfg.load(config)
    effective_dry_run = dry_run or cfg.agent.dry_run
    asyncio.run(_main_loop(cfg, dry_run=effective_dry_run, once=once))


async def _main_loop(cfg: AegisOpsCfg, *, dry_run: bool, once: bool) -> None:
    splunk = SplunkClient(
        url=cfg.splunk.url,
        token=cfg.splunk.token,
        verify_tls=cfg.splunk.verify_tls,
    )
    observer = Observer(
        splunk=splunk,
        earliest=cfg.splunk.earliest,
        latest=cfg.splunk.latest,
    )
    reasoner = Reasoner(
        splunk=splunk,
        provider=cfg.hosted_model.provider,
        model=cfg.hosted_model.model,
    )
    policy = PolicyEngine(
        mode=cfg.policy.mode,
        min_confidence=cfg.policy.min_confidence,
        cooldown_secs=cfg.policy.cooldown_secs,
    )
    actuator = Actuator(dry_run=dry_run)
    auditor = Auditor(cfg.audit, dry_run=dry_run)
    gateways = {
        gw.name: GatewayClient(gw.url) for gw in cfg.gateways
    }

    log.info(
        "AegisOps starting: %d gateway(s), policy=%s, dry_run=%s, model=%s/%s",
        len(gateways),
        cfg.policy.mode,
        dry_run,
        cfg.hosted_model.provider,
        cfg.hosted_model.model,
    )

    try:
        while True:
            tick_start = time.time()
            await asyncio.gather(
                *[
                    _tick_one(
                        name=name,
                        client=client,
                        observer=observer,
                        reasoner=reasoner,
                        policy=policy,
                        actuator=actuator,
                        auditor=auditor,
                    )
                    for name, client in gateways.items()
                ],
                return_exceptions=True,
            )
            if once:
                return
            elapsed = time.time() - tick_start
            sleep_for = max(0.5, cfg.agent.loop_interval_secs - elapsed)
            await asyncio.sleep(sleep_for)
    finally:
        for c in gateways.values():
            await c.close()
        await splunk.close()
        await auditor.close()


async def _tick_one(
    *,
    name: str,
    client: GatewayClient,
    observer: Observer,
    reasoner: Reasoner,
    policy: PolicyEngine,
    actuator: Actuator,
    auditor: Auditor,
) -> None:
    try:
        observation = await observer.observe(name, client)
    except Exception as exc:
        log.warning("observe failed for %s: %s", name, exc)
        return

    decision, prompt, raw = await reasoner.reason(observation)
    exec_mode = policy.classify(decision)
    result, err = await actuator.act(decision, exec_mode, client)

    log.info(
        "[%s] decision=%s(%s) conf=%.2f exec=%-9s | %s",
        name,
        decision.action,
        f"{decision.duration_secs}s" if decision.duration_secs else "-",
        decision.confidence,
        exec_mode,
        decision.justification[:80],
    )

    await auditor.record(
        DecisionRecord(
            ts=time.time(),
            gateway=name,
            observation=observation,
            decision=decision,
            exec_mode=exec_mode,
            actuator_result=result,
            actuator_error=err,
            prompt=prompt,
            raw_model_response=raw,
        )
    )


if __name__ == "__main__":
    app()
