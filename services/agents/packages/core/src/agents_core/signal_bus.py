"""§10.3's agent -> core contract: agents publish, "the core" (this bus,
standing in for the real one on the Rust side reached over NATS) validates
before anything downstream ever sees the signal. There is no NATS here —
this is the in-process equivalent, same "real logic, mock transport" split
as `TopicBus` in the gateway (TypeScript) — but the four checks below are
the real gate, not a stub.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .agent import AgentOutput

Unsubscribe = Callable[[], None]


@dataclass(frozen=True)
class PublishedSignal:
    symbol_id: int
    agent_kind: str
    output: AgentOutput
    features_hash: str
    published_at_ns: int
    ttl_ns: int


class SignalRejected(Exception):
    """§10.3: the core "never treats [a signal] as a decision" — a signal
    that fails any of these checks never reaches fusion at all."""


@dataclass
class SignalBus:
    """§10.3's four core checks, in order: (1) schema + TTL, (2) known
    `features_hash`, (3) probability within the agent's calibrated range,
    (4) — feeding an accepted signal into fusion — is `crates/strategy::fusion`
    on the Rust side, out of this bus's scope; `subscribe` is the hand-off
    point a real bridge would forward accepted signals across."""

    _known_feature_hashes: set[str] = field(default_factory=set)
    _subscribers: list[Callable[[PublishedSignal], None]] = field(default_factory=list)
    _published: list[PublishedSignal] = field(default_factory=list)

    def register_feature_snapshot(self, features_hash: str) -> None:
        """Called once per bar close with the hash of the feature vector
        that was actually computed — the only hashes `publish` will accept
        signals against."""
        self._known_feature_hashes.add(features_hash)

    def publish(
        self,
        signal: PublishedSignal,
        *,
        now_ns: int,
        calibrated_range: tuple[float, float] | None = None,
    ) -> None:
        # AgentOutput's own Pydantic validation already enforces "schema";
        # TTL is this bus's job.
        if now_ns > signal.published_at_ns + signal.ttl_ns:
            msg = f"signal for {signal.agent_kind}/{signal.symbol_id} has expired (TTL)"
            raise SignalRejected(msg)

        if signal.features_hash not in self._known_feature_hashes:
            raise SignalRejected(
                f"unknown features_hash {signal.features_hash!r} — stale or hallucinated context"
            )

        if calibrated_range is not None:
            low, high = calibrated_range
            if not (low <= signal.output.probability <= high):
                prob = signal.output.probability
                raise SignalRejected(f"probability {prob} outside calibrated range [{low}, {high}]")

        self._published.append(signal)
        for subscriber in self._subscribers:
            subscriber(signal)

    def subscribe(self, callback: Callable[[PublishedSignal], None]) -> Unsubscribe:
        self._subscribers.append(callback)

        def unsubscribe() -> None:
            self._subscribers.remove(callback)

        return unsubscribe

    def active_signals(self, *, now_ns: int) -> list[PublishedSignal]:
        return [s for s in self._published if now_ns <= s.published_at_ns + s.ttl_ns]
