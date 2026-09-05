"""Bridges a §7.2 conditional-reliability result into the shape
`crates/strategy::fuse`'s `FusionInput` expects (probability, weight,
resolved_predictions) — the graph (Python) and the fusion formula (Rust)
have no live bridge in this sandbox, so this is the contract a real
orchestrator would serialize across that bridge, exercised here on the
Python side of it.
"""

from __future__ import annotations

from dataclasses import dataclass

from .queries import ConditionalReliability

# Mirrors crates/strategy::fusion::SAMPLE_SIZE_GATE — kept as a literal
# rather than a cross-language import since there's no shared constants
# module between the two runtimes; a real deployment would generate both
# from one source (e.g. a shared config file) rather than hand-sync two
# literals, which is a known small honesty gap in this sandbox.
SAMPLE_SIZE_GATE = 30


@dataclass(frozen=True)
class FusionPriorInput:
    source_id: str
    probability: float
    weight: float
    resolved_predictions: int


def graph_prior_to_fusion_input(
    reliability: ConditionalReliability, *, source_id: str = "graph-prior"
) -> FusionPriorInput:
    """A graph prior's `probability` is its historical hit rate; its
    `weight` starts at 1.0 once §8.4's sample-size gate clears. A full
    online Brier-tracked weight for the graph-prior source itself — the
    same treatment every other fusion source gets — is future work once
    it has enough of its *own* resolved predictions (as opposed to the
    pattern/news predictions it summarizes) to track."""
    weight = 1.0 if reliability.n >= SAMPLE_SIZE_GATE else 0.0
    return FusionPriorInput(
        source_id=source_id,
        probability=reliability.hit_rate,
        weight=weight,
        resolved_predictions=reliability.n,
    )
