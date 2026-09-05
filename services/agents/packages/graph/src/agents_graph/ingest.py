"""Ingest pipelines (Prompt 7): turn agent-detected events into the §7.1
graph. Every function here is idempotent — the `KnowledgeGraph` upserts it
calls are all upsert-by-id, so re-running the same ingest call (e.g. a
retried bar-close event) never duplicates a node or edge.

Deliberately independent of `agents_news`/`agents_pattern`/`agents_regime`:
these take plain scalars rather than importing those packages' Pydantic
models, so `agents-graph` doesn't have to depend on the whole agent
roster just to record what it produced. The orchestrator (which already
depends on every agent package) is where a real call site adapts an
`AgentOutput`/`PatternInstance`/`NewsEventOutput` into these parameters.
"""

from __future__ import annotations

from .outcomes import FixedHorizonMove, OutcomeResolution, Verdict
from .schema import (
    co_occurred_with_edge,
    during_edge,
    event_type_node,
    instance_of_edge,
    market_regime_node,
    mentions_edge,
    news_event_node,
    of_type_edge,
    on_edge,
    outcome_node,
    pattern_inst_node,
    pattern_node,
    preceded_edge,
    resolved_as_edge,
    trade_node,
    triggered_by_edge,
)
from .store import KnowledgeGraph


def ingest_news_event(
    graph: KnowledgeGraph,
    *,
    id: str,
    ts: int,
    headline: str,
    source: str,
    impact_tier: str,
    sentiment: float,
    event_type: str,
    instruments: list[str],
    embedding_id: str | None = None,
    salience: float = 1.0,
) -> None:
    graph.upsert_node(
        news_event_node(
            id,
            ts=ts,
            headline=headline,
            source=source,
            impact_tier=impact_tier,
            embedding_id=embedding_id,
            sentiment=sentiment,
        )
    )
    graph.upsert_node(event_type_node(event_type))
    graph.upsert_edge(of_type_edge(id, event_type))
    for symbol in instruments:
        graph.upsert_edge(mentions_edge(id, symbol, salience=salience))


def ingest_pattern_instance(
    graph: KnowledgeGraph,
    *,
    id: str,
    ts_start: int,
    ts_end: int,
    symbol: str,
    confidence: float,
    detected_by: str,
    pattern_id: str,
    pattern_name: str,
    pattern_family: str,
    timeframe: str,
    direction_bias: str,
    market_regime_id: str | None = None,
) -> None:
    graph.upsert_node(
        pattern_node(
            pattern_id,
            name=pattern_name,
            family=pattern_family,
            timeframe=timeframe,
            direction_bias=direction_bias,
        )
    )
    graph.upsert_node(
        pattern_inst_node(
            id,
            ts_start=ts_start,
            ts_end=ts_end,
            symbol=symbol,
            confidence=confidence,
            detected_by=detected_by,
        )
    )
    graph.upsert_edge(instance_of_edge(id, pattern_id))
    graph.upsert_edge(on_edge(id, symbol))
    if market_regime_id is not None:
        graph.upsert_edge(during_edge(id, market_regime_id))


def ingest_market_regime(
    graph: KnowledgeGraph,
    *,
    id: str,
    label: str,
    ts_start: int,
    ts_end: int,
    vol_bucket: str,
    trend_strength: float,
) -> None:
    graph.upsert_node(
        market_regime_node(
            id,
            label=label,
            ts_start=ts_start,
            ts_end=ts_end,
            vol_bucket=vol_bucket,
            trend_strength=trend_strength,
        )
    )


def ingest_trade(
    graph: KnowledgeGraph,
    *,
    id: str,
    ts_open: int,
    ts_close: int | None,
    side: str,
    r_multiple: float,
    pnl: float,
    mode: str,
    triggered_by_id: str,
) -> None:
    graph.upsert_node(
        trade_node(
            id,
            ts_open=ts_open,
            ts_close=ts_close,
            side=side,
            r_multiple=r_multiple,
            pnl=pnl,
            mode=mode,
        )
    )
    graph.upsert_edge(triggered_by_edge(id, triggered_by_id))


def link_co_occurring_patterns(
    graph: KnowledgeGraph, pattern_inst_a_id: str, pattern_inst_b_id: str, *, lag_bars: int
) -> None:
    graph.upsert_edge(
        co_occurred_with_edge(pattern_inst_a_id, pattern_inst_b_id, lag_bars=lag_bars)
    )


def attach_pattern_outcome(
    graph: KnowledgeGraph,
    *,
    pattern_inst_id: str,
    outcome_id: str,
    resolution: OutcomeResolution,
    horizon_min: int,
) -> None:
    """The pattern-verification half of automatic outcome resolution:
    writes the Outcome node §12.3 renders as CONFIRMED/FAILED/TIMEOUT plus
    the RESOLVED_AS edge §7.2's conditional-reliability query reads."""
    graph.upsert_node(
        outcome_node(
            outcome_id,
            horizon_min=horizon_min,
            move_pips=resolution.move_pips,
            move_atr=resolution.move_atr,
            direction=resolution.direction,
            mfe=resolution.mfe,
            mae=resolution.mae,
        )
    )
    graph.upsert_edge(
        resolved_as_edge(
            pattern_inst_id,
            outcome_id,
            confirmed=resolution.verdict == Verdict.CONFIRMED,
            r_multiple=resolution.r_multiple,
        )
    )


def attach_news_outcome(
    graph: KnowledgeGraph,
    *,
    news_event_id: str,
    outcome_id: str,
    move: FixedHorizonMove,
    horizon_min: int,
    lag_min: int,
) -> None:
    """The news-impact half of automatic outcome resolution: writes the
    Outcome node and the PRECEDED edge §7.2 query #2 reads to answer "does
    this event type always move this pair?"."""
    graph.upsert_node(
        outcome_node(
            outcome_id,
            horizon_min=horizon_min,
            move_pips=move.move_pips,
            move_atr=move.move_atr,
            direction=move.direction,
            mfe=0.0,
            mae=0.0,
        )
    )
    graph.upsert_edge(preceded_edge(news_event_id, outcome_id, lag_min=lag_min))
