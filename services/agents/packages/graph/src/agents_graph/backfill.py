"""A backfill script for historical news and patterns (Prompt 7). There is
no real historical news/pattern archive to backfill from in this
sandbox, so this generates deterministic synthetic history — same
discipline as `agents_mcp.synthetic_data`: a fixed reference instant,
never `time.time()`, and a seeded `random.Random` rather than the global
`random` module, so the exact same graph comes out of two calls with the
same arguments.
"""

from __future__ import annotations

import random

from .ingest import (
    attach_news_outcome,
    attach_pattern_outcome,
    ingest_market_regime,
    ingest_news_event,
    ingest_pattern_instance,
    link_co_occurring_patterns,
)
from .outcomes import OutcomeResolution, Verdict, resolve_fixed_horizon_move
from .schema import instrument_node
from .store import KnowledgeGraph

# 2026-01-01T00:00:00Z — the same fixed reference instant convention as
# agents_mcp.synthetic_data, kept independently here since agents-graph
# has no dependency on agents-mcp (the dependency runs the other way).
_FIXED_REFERENCE_MS = 1_767_225_600_000
_DAY_MS = 24 * 60 * 60 * 1000

_SYMBOLS = ("EURUSD", "GBPUSD", "USDJPY")
_PATTERNS = (("double_top", "reversal", "short"), ("double_bottom", "reversal", "long"))
_EVENT_TYPES = ("rate_decision", "nfp", "cpi")
_REGIMES = ("Trending", "Ranging", "Expansion", "HighVolChoppy")


def build_backfilled_graph(
    *, pattern_count: int = 300, news_count: int = 150, seed: int = 7
) -> KnowledgeGraph:
    """`pattern_count` resolved pattern instances and `news_count` resolved
    news events, spread across `_SYMBOLS`/regimes/event types so the §7.2
    queries have something real, if synthetic, to aggregate over.
    `double_top` is deliberately seeded with a higher hit rate than
    `double_bottom` so `conditional_reliability` has a genuine difference
    to discover rather than uniform noise."""
    rng = random.Random(seed)
    graph = KnowledgeGraph()

    for symbol in _SYMBOLS:
        graph.upsert_node(
            instrument_node(
                symbol,
                asset_class="fx",
                base=symbol[:3],
                quote=symbol[3:],
                tick_size=1e-5,
                sessions=["london", "ny"],
            )
        )

    _backfill_patterns(graph, rng, pattern_count)
    _backfill_news(graph, rng, news_count)
    return graph


def _backfill_patterns(graph: KnowledgeGraph, rng: random.Random, pattern_count: int) -> None:
    previous_inst_id: str | None = None
    for i in range(pattern_count):
        ts_start = _FIXED_REFERENCE_MS - (pattern_count - i) * _DAY_MS
        symbol = rng.choice(_SYMBOLS)
        pattern_name, family, direction_bias = rng.choice(_PATTERNS)
        regime_label = rng.choice(_REGIMES)
        regime_id = f"backfill-regime-{i}"
        ingest_market_regime(
            graph,
            id=regime_id,
            label=regime_label,
            ts_start=ts_start,
            ts_end=ts_start + _DAY_MS,
            vol_bucket=rng.choice(("low", "mid", "high")),
            trend_strength=rng.random(),
        )
        inst_id = f"backfill-pi-{i}"
        ingest_pattern_instance(
            graph,
            id=inst_id,
            ts_start=ts_start,
            ts_end=ts_start + 3_600_000,
            symbol=symbol,
            confidence=rng.uniform(0.5, 0.95),
            detected_by="pattern-agent",
            pattern_id=f"backfill-p-{pattern_name}",
            pattern_name=pattern_name,
            pattern_family=family,
            timeframe="M5",
            direction_bias=direction_bias,
            market_regime_id=regime_id,
        )
        hit_rate = 0.62 if pattern_name == "double_top" else 0.45
        confirmed = rng.random() < hit_rate
        r_multiple = rng.uniform(1.2, 2.5) if confirmed else -1.0
        resolution = OutcomeResolution(
            verdict=Verdict.CONFIRMED if confirmed else Verdict.FAILED,
            bars_to_resolution=rng.randint(1, 30),
            mfe=abs(r_multiple),
            mae=abs(r_multiple) / 2,
            r_multiple=r_multiple,
            move_pips=r_multiple * 10,
            move_atr=r_multiple,
            direction="Long",
        )
        attach_pattern_outcome(
            graph,
            pattern_inst_id=inst_id,
            outcome_id=f"backfill-out-{inst_id}",
            resolution=resolution,
            horizon_min=15,
        )
        if previous_inst_id is not None and rng.random() < 0.1:
            link_co_occurring_patterns(graph, previous_inst_id, inst_id, lag_bars=rng.randint(0, 3))
        previous_inst_id = inst_id


def _backfill_news(graph: KnowledgeGraph, rng: random.Random, news_count: int) -> None:
    price_at_event = 1.1000
    for i in range(news_count):
        ts = _FIXED_REFERENCE_MS - (news_count - i) * _DAY_MS
        symbol = rng.choice(_SYMBOLS)
        event_type = rng.choice(_EVENT_TYPES)
        ev_id = f"backfill-ev-{i}"
        ingest_news_event(
            graph,
            id=ev_id,
            ts=ts,
            headline=f"{event_type} #{i}",
            source="backfill",
            impact_tier=rng.choice(("low", "medium", "high")),
            sentiment=rng.uniform(-1.0, 1.0),
            event_type=event_type,
            instruments=[symbol],
        )
        expected_direction = rng.choice(("Long", "Short"))
        matches_expectation = rng.random() < 0.55
        sign = 1 if expected_direction == "Long" else -1
        if not matches_expectation:
            sign = -sign
        price_at_horizon = price_at_event + sign * rng.uniform(0.0005, 0.0080)
        move = resolve_fixed_horizon_move(
            price_at_event=price_at_event,
            price_at_horizon=price_at_horizon,
            expected_direction=expected_direction,
            pip_size=0.0001,
            atr=0.0020,
        )
        attach_news_outcome(
            graph,
            news_event_id=ev_id,
            outcome_id=f"backfill-out-{ev_id}",
            move=move,
            horizon_min=15,
            lag_min=15,
        )
