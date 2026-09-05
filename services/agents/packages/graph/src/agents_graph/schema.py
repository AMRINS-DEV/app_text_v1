"""The §7.1 node/edge schema, as plain typed builders over `Node`/`Edge`.

```cypher
(:Instrument   {symbol, asset_class, base, quote, tick_size, sessions})
(:NewsEvent    {id, ts, headline, source, impact_tier, embedding_id, sentiment})
(:EventType    {name})
(:Pattern      {id, name, family, timeframe, direction_bias})
(:PatternInst  {id, ts_start, ts_end, symbol, confidence, detected_by})
(:MarketRegime {id, label, ts_start, ts_end, vol_bucket, trend_strength})
(:Outcome      {id, horizon_min, move_pips, move_atr, direction, mfe, mae})
(:Trade        {id, ts_open, ts_close, side, r_multiple, pnl, mode})
(:Session      {name, tz, open, close})
(:Concept      {name})
```

Every builder below produces exactly these properties. Node ids follow the
schema's own natural key: the four label types with no `id` property
(Instrument, EventType, Session, Concept) are addressed by their one
identifying field (`symbol`/`name`); the rest carry an explicit `id` the
caller supplies (matching how a real Cypher `MERGE` would key them).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal

NodeLabel = Literal[
    "Instrument",
    "NewsEvent",
    "EventType",
    "Pattern",
    "PatternInst",
    "MarketRegime",
    "Outcome",
    "Trade",
    "Session",
    "Concept",
]

EdgeLabel = Literal[
    "OF_TYPE",
    "MENTIONS",
    "PRECEDED",
    "INSTANCE_OF",
    "ON",
    "RESOLVED_AS",
    "DURING",
    "CO_OCCURRED_WITH",
    "TRIGGERED_BY",
    "EXPRESSES",
    "CORRELATED_WITH",
]


@dataclass(frozen=True)
class Node:
    id: str
    label: NodeLabel
    properties: dict[str, object] = field(default_factory=dict)


@dataclass(frozen=True)
class Edge:
    label: EdgeLabel
    source_id: str
    target_id: str
    properties: dict[str, object] = field(default_factory=dict)


def instrument_node(
    symbol: str, *, asset_class: str, base: str, quote: str, tick_size: float, sessions: list[str]
) -> Node:
    return Node(
        id=symbol,
        label="Instrument",
        properties={
            "symbol": symbol,
            "asset_class": asset_class,
            "base": base,
            "quote": quote,
            "tick_size": tick_size,
            "sessions": sessions,
        },
    )


def news_event_node(
    id: str,
    *,
    ts: int,
    headline: str,
    source: str,
    impact_tier: str,
    embedding_id: str | None,
    sentiment: float,
) -> Node:
    return Node(
        id=id,
        label="NewsEvent",
        properties={
            "id": id,
            "ts": ts,
            "headline": headline,
            "source": source,
            "impact_tier": impact_tier,
            "embedding_id": embedding_id,
            "sentiment": sentiment,
        },
    )


def event_type_node(name: str) -> Node:
    return Node(id=name, label="EventType", properties={"name": name})


def pattern_node(id: str, *, name: str, family: str, timeframe: str, direction_bias: str) -> Node:
    return Node(
        id=id,
        label="Pattern",
        properties={
            "id": id,
            "name": name,
            "family": family,
            "timeframe": timeframe,
            "direction_bias": direction_bias,
        },
    )


def pattern_inst_node(
    id: str, *, ts_start: int, ts_end: int, symbol: str, confidence: float, detected_by: str
) -> Node:
    return Node(
        id=id,
        label="PatternInst",
        properties={
            "id": id,
            "ts_start": ts_start,
            "ts_end": ts_end,
            "symbol": symbol,
            "confidence": confidence,
            "detected_by": detected_by,
        },
    )


def market_regime_node(
    id: str, *, label: str, ts_start: int, ts_end: int, vol_bucket: str, trend_strength: float
) -> Node:
    return Node(
        id=id,
        label="MarketRegime",
        properties={
            "id": id,
            "label": label,
            "ts_start": ts_start,
            "ts_end": ts_end,
            "vol_bucket": vol_bucket,
            "trend_strength": trend_strength,
        },
    )


def outcome_node(
    id: str,
    *,
    horizon_min: int,
    move_pips: float,
    move_atr: float,
    direction: str,
    mfe: float,
    mae: float,
) -> Node:
    return Node(
        id=id,
        label="Outcome",
        properties={
            "id": id,
            "horizon_min": horizon_min,
            "move_pips": move_pips,
            "move_atr": move_atr,
            "direction": direction,
            "mfe": mfe,
            "mae": mae,
        },
    )


def trade_node(
    id: str,
    *,
    ts_open: int,
    ts_close: int | None,
    side: str,
    r_multiple: float,
    pnl: float,
    mode: str,
) -> Node:
    return Node(
        id=id,
        label="Trade",
        properties={
            "id": id,
            "ts_open": ts_open,
            "ts_close": ts_close,
            "side": side,
            "r_multiple": r_multiple,
            "pnl": pnl,
            "mode": mode,
        },
    )


def session_node(name: str, *, tz: str, open: str, close: str) -> Node:
    return Node(
        id=name, label="Session", properties={"name": name, "tz": tz, "open": open, "close": close}
    )


def concept_node(name: str) -> Node:
    return Node(id=name, label="Concept", properties={"name": name})


def of_type_edge(news_event_id: str, event_type_name: str) -> Edge:
    return Edge(label="OF_TYPE", source_id=news_event_id, target_id=event_type_name)


def mentions_edge(news_event_id: str, symbol: str, *, salience: float) -> Edge:
    return Edge(
        label="MENTIONS",
        source_id=news_event_id,
        target_id=symbol,
        properties={"salience": salience},
    )


def preceded_edge(news_event_id: str, outcome_id: str, *, lag_min: int) -> Edge:
    return Edge(
        label="PRECEDED",
        source_id=news_event_id,
        target_id=outcome_id,
        properties={"lag_min": lag_min},
    )


def instance_of_edge(pattern_inst_id: str, pattern_id: str) -> Edge:
    return Edge(label="INSTANCE_OF", source_id=pattern_inst_id, target_id=pattern_id)


def on_edge(pattern_inst_id: str, symbol: str) -> Edge:
    return Edge(label="ON", source_id=pattern_inst_id, target_id=symbol)


def resolved_as_edge(
    source_id: str, outcome_id: str, *, confirmed: bool, r_multiple: float
) -> Edge:
    """`source_id` is a `PatternInst` id per §7.1's own diagram."""
    return Edge(
        label="RESOLVED_AS",
        source_id=source_id,
        target_id=outcome_id,
        properties={"confirmed": confirmed, "r_multiple": r_multiple},
    )


def during_edge(pattern_inst_id: str, market_regime_id: str) -> Edge:
    return Edge(label="DURING", source_id=pattern_inst_id, target_id=market_regime_id)


def co_occurred_with_edge(pattern_inst_a_id: str, pattern_inst_b_id: str, *, lag_bars: int) -> Edge:
    return Edge(
        label="CO_OCCURRED_WITH",
        source_id=pattern_inst_a_id,
        target_id=pattern_inst_b_id,
        properties={"lag_bars": lag_bars},
    )


def triggered_by_edge(trade_id: str, pattern_inst_or_news_event_id: str) -> Edge:
    return Edge(label="TRIGGERED_BY", source_id=trade_id, target_id=pattern_inst_or_news_event_id)


def expresses_edge(pattern_id: str, concept_name: str) -> Edge:
    return Edge(label="EXPRESSES", source_id=pattern_id, target_id=concept_name)


def correlated_with_edge(symbol_a: str, symbol_b: str, *, rho: float, window: str) -> Edge:
    return Edge(
        label="CORRELATED_WITH",
        source_id=symbol_a,
        target_id=symbol_b,
        properties={"rho": rho, "window": window},
    )
