"""§10.1: "Vetoes are logged and measured — if the critic's vetoed trades
would have been profitable, its weight is reduced automatically." The
same shape as `crates/risk::quick_profit`'s shadow A/B tracker on the Rust
side (§9.4): log every veto, wait for it to resolve (would that trade have
won or lost had it not been vetoed — determined externally, e.g. by
paper-tracking the signal anyway), then decide whether the critic is net
harmful once enough vetoes have resolved.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class VetoRecord:
    signal_id: str
    vetoed_at_ns: int
    resolved: bool = False
    would_have_been_profitable: bool | None = None


@dataclass
class CriticOutcomeTracker:
    min_resolved_for_judgment: int = 100
    _records: dict[str, VetoRecord] = field(default_factory=dict)

    def record_veto(self, signal_id: str, vetoed_at_ns: int) -> None:
        self._records[signal_id] = VetoRecord(signal_id=signal_id, vetoed_at_ns=vetoed_at_ns)

    def resolve(self, signal_id: str, *, would_have_been_profitable: bool) -> None:
        record = self._records.get(signal_id)
        if record is None:
            msg = f"no veto was recorded for signal_id {signal_id!r}"
            raise KeyError(msg)
        record.resolved = True
        record.would_have_been_profitable = would_have_been_profitable

    def resolved_vetoes(self) -> list[VetoRecord]:
        return [r for r in self._records.values() if r.resolved]

    def should_reduce_weight(self) -> bool:
        """True once at least `min_resolved_for_judgment` vetoes have
        resolved *and* a majority of the vetoed trades would actually have
        been profitable — i.e. the critic has been net harmful, not just
        occasionally wrong."""
        resolved = self.resolved_vetoes()
        if len(resolved) < self.min_resolved_for_judgment:
            return False
        profitable = sum(1 for r in resolved if r.would_have_been_profitable)
        return profitable > len(resolved) / 2
