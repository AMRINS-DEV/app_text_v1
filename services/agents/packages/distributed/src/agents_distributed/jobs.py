"""Real, distributable units of agent work (§16: "agents are stateless and
horizontally scalable" — this package exists to prove that claim across
actual separate OS processes, not just separate Python objects in one
process). A `Job` carries everything a worker needs to reproduce the exact
computation on its own: the agent kind and its input as a plain, JSON-safe
dict rather than a pickled model instance, so a result never depends on
which process constructed the input or the order jobs were dispatched in —
the literal meaning of "stateless."
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class Job:
    job_id: str
    agent_kind: str
    payload: dict[str, Any]


@dataclass(frozen=True)
class JobResult:
    job_id: str
    agent_kind: str
    worker_id: int
    output: dict[str, Any] | None
    error: str | None

    @property
    def succeeded(self) -> bool:
        return self.error is None
