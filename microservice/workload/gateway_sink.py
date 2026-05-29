"""Streams raw log lines to the Aegis edge gateway over TCP.

This is what replaces the manual `log_spammer.py` invocations: the workload
opens one long-lived TCP connection to the gateway's ingest port and writes
newline-delimited log lines as it generates them. If the gateway is not up
yet (or restarts), the sink reconnects on its own and never crashes the app.
"""

from __future__ import annotations

import asyncio
import logging

log = logging.getLogger("workload.gateway")


class GatewaySink:
    def __init__(self, host: str, port: int, *, enabled: bool = True, max_buffer: int = 10_000) -> None:
        self._host = host
        self._port = port
        self._enabled = enabled
        self._queue: asyncio.Queue[str] = asyncio.Queue(maxsize=max_buffer)
        self._task: asyncio.Task | None = None
        self._connected = False
        self._sent = 0
        self._dropped = 0

    @property
    def connected(self) -> bool:
        return self._connected

    @property
    def stats(self) -> dict:
        return {
            "enabled": self._enabled,
            "connected": self._connected,
            "target": f"tcp://{self._host}:{self._port}",
            "lines_sent": self._sent,
            "lines_dropped": self._dropped,
            "buffered": self._queue.qsize(),
        }

    def emit(self, line: str) -> None:
        """Non-blocking enqueue. Drops oldest-style (counts) if the buffer is
        full so a stalled gateway can never apply backpressure to the sim."""
        if not self._enabled:
            return
        try:
            self._queue.put_nowait(line)
        except asyncio.QueueFull:
            self._dropped += 1

    async def start(self) -> None:
        if self._enabled and self._task is None:
            self._task = asyncio.create_task(self._run(), name="gateway-sink")

    async def stop(self) -> None:
        if self._task is not None:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass
            self._task = None

    async def _run(self) -> None:
        backoff = 1.0
        while True:
            try:
                reader, writer = await asyncio.open_connection(self._host, self._port)
                self._connected = True
                backoff = 1.0
                log.info("connected to aegis gateway at tcp://%s:%s", self._host, self._port)
                try:
                    while True:
                        line = await self._queue.get()
                        writer.write((line + "\n").encode("utf-8"))
                        await writer.drain()
                        self._sent += 1
                finally:
                    self._connected = False
                    writer.close()
                    try:
                        await writer.wait_closed()
                    except Exception:  # noqa: BLE001 - best-effort close
                        pass
            except asyncio.CancelledError:
                raise
            except (OSError, ConnectionError) as exc:
                self._connected = False
                log.warning("aegis gateway unreachable (%s); retrying in %.0fs", exc, backoff)
                await asyncio.sleep(backoff)
                backoff = min(backoff * 2, 15.0)
