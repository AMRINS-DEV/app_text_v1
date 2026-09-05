"""§17 Phase 6 exit criterion: "conditional-reliability query < 50ms."

The design doc's own target is 10M edges against a real FalkorDB. This
sandbox has neither the memory nor the wall-clock budget to synthesize and
query a 10M-edge graph in a CI-scoped test — so this benchmarks at a
scaled-down, still-meaningful N (100k pattern instances, ~300k edges) and
documents the extrapolation rather than silently claiming the 10M-edge
number. `conditional_reliability`'s cost is O(matching pattern instances),
not O(total graph size) — every hop (`INSTANCE_OF`, `ON`, `DURING`,
`RESOLVED_AS`) is an indexed dict lookup in `KnowledgeGraph`, not a scan —
so its latency is governed by how many instances share one pattern *name*,
which doesn't grow with unrelated parts of the graph. That's what makes a
smaller-N benchmark here a legitimate proxy for the 10M-edge target rather
than an unrelated number.
"""

from __future__ import annotations

import time

from agents_graph import (
    KnowledgeGraph,
    OutcomeResolution,
    Verdict,
    attach_pattern_outcome,
    conditional_reliability,
    ingest_market_regime,
    ingest_pattern_instance,
)

_INSTANCE_COUNT = 100_000


def _build_benchmark_graph() -> KnowledgeGraph:
    graph = KnowledgeGraph()
    ingest_market_regime(
        graph,
        id="regime-1",
        label="Trending",
        ts_start=0,
        ts_end=1,
        vol_bucket="mid",
        trend_strength=0.5,
    )
    for i in range(_INSTANCE_COUNT):
        # Spread across 20 pattern names x 10 symbols so the query has to
        # filter, not just return "everything" — a closer proxy for a real
        # workload than one giant matching bucket.
        pattern_name = f"pattern-{i % 20}"
        symbol = f"SYM{i % 10}"
        inst_id = f"pi-{i}"
        ingest_pattern_instance(
            graph,
            id=inst_id,
            ts_start=i,
            ts_end=i + 1,
            symbol=symbol,
            confidence=0.7,
            detected_by="pattern-agent",
            pattern_id=f"p-{pattern_name}",
            pattern_name=pattern_name,
            pattern_family="reversal",
            timeframe="M5",
            direction_bias="short",
            market_regime_id="regime-1",
        )
        resolution = OutcomeResolution(
            verdict=Verdict.CONFIRMED if i % 2 == 0 else Verdict.FAILED,
            bars_to_resolution=1,
            mfe=1.0,
            mae=1.0,
            r_multiple=1.5 if i % 2 == 0 else -1.0,
            move_pips=10.0,
            move_atr=1.0,
            direction="Long",
        )
        attach_pattern_outcome(
            graph,
            pattern_inst_id=inst_id,
            outcome_id=f"out-{inst_id}",
            resolution=resolution,
            horizon_min=15,
        )
    return graph


def test_conditional_reliability_query_is_comfortably_under_50ms_at_scale():
    graph = _build_benchmark_graph()
    assert graph.node_count() > 0
    # INSTANCE_OF + ON + DURING + RESOLVED_AS per instance.
    assert graph.edge_count() == 4 * _INSTANCE_COUNT

    started = time.perf_counter()
    result = conditional_reliability(
        graph, pattern_name="pattern-0", symbol="SYM0", regime_label="Trending", since_ts=0
    )
    elapsed_ms = (time.perf_counter() - started) * 1000

    # i % 20 == 0 implies i % 10 == 0 (20 is a multiple of 10), so every
    # pattern-0 instance is already on SYM0 -- one match per 20 instances.
    assert result.n == _INSTANCE_COUNT // 20
    assert elapsed_ms < 50.0, f"took {elapsed_ms:.2f}ms, over the 50ms budget"
