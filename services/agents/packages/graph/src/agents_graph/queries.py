"""§7.2's three read queries, plus confluence discovery, as real traversal/
aggregation logic against `KnowledgeGraph`'s indexed adjacency — the same
questions the design doc's Cypher answers, computed the same way (walk the
real relationships, aggregate the real resolved outcomes), just without a
live FalkorDB to send `GRAPH.QUERY` to in this sandbox.

Every result carries its own `n` so a caller can apply §8.4/§8.7's
sample-size gate (`n >= 30`, `n >= 40` for confluence) before trusting it —
these functions report the number, they don't silently hide an
under-powered result.
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass

from .store import KnowledgeGraph


def _as_float(value: object, default: float = 0.0) -> float:
    return float(value) if isinstance(value, (int, float)) else default


def _as_int(value: object, default: int = 0) -> int:
    return int(value) if isinstance(value, (int, float)) else default


def _percentile_cont(values: list[float], p: float) -> float:
    """Linear-interpolation percentile — the same definition openCypher's
    `percentileCont` uses, so a result computed here and one computed by a
    real FalkorDB against the same data agree."""
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    index = p * (len(ordered) - 1)
    lower = int(index)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = index - lower
    return ordered[lower] + (ordered[upper] - ordered[lower]) * fraction


@dataclass(frozen=True)
class ConditionalReliability:
    n: int
    hit_rate: float
    avg_r: float
    median_r: float


def conditional_reliability(
    graph: KnowledgeGraph,
    *,
    pattern_name: str,
    symbol: str,
    regime_label: str,
    since_ts: int,
) -> ConditionalReliability:
    """§7.2 query #1: does `pattern_name` work on `symbol` in `regime_label`?
    Walks Pattern -[:INSTANCE_OF]- PatternInst -[:ON]-> Instrument,
    -[:DURING]-> MarketRegime, -[:RESOLVED_AS]-> Outcome — every hop a real
    indexed lookup, never a full scan."""
    confirmed_flags: list[float] = []
    r_multiples: list[float] = []

    for pattern in graph.nodes_by_label("Pattern"):
        if pattern.properties.get("name") != pattern_name:
            continue
        for _instance_of_edge, pattern_inst in graph.in_neighbors(pattern.id, "INSTANCE_OF"):
            if pattern_inst.properties.get("symbol") != symbol:
                continue
            ts_start = pattern_inst.properties.get("ts_start")
            if not isinstance(ts_start, int) or ts_start < since_ts:
                continue
            regimes = graph.out_neighbors(pattern_inst.id, "DURING")
            if not any(regime.properties.get("label") == regime_label for _e, regime in regimes):
                continue
            for resolved_edge, _outcome in graph.out_neighbors(pattern_inst.id, "RESOLVED_AS"):
                confirmed_flags.append(1.0 if resolved_edge.properties.get("confirmed") else 0.0)
                r_multiples.append(_as_float(resolved_edge.properties.get("r_multiple")))

    n = len(r_multiples)
    if n == 0:
        return ConditionalReliability(n=0, hit_rate=0.0, avg_r=0.0, median_r=0.0)
    return ConditionalReliability(
        n=n,
        hit_rate=sum(confirmed_flags) / n,
        avg_r=sum(r_multiples) / n,
        median_r=_percentile_cont(r_multiples, 0.5),
    )


@dataclass(frozen=True)
class NewsImpactPeriod:
    quarter: str
    n: int
    avg_impact: float
    direction_hit_rate: float


def _quarter_key(ts_ms: int) -> str:
    """A plain UTC-quarter bucket key (`"2026Q1"`) — `date.truncate` in the
    real Cypher query, reimplemented without a timezone-aware date library
    since this only ever buckets fixed-reference synthetic timestamps."""
    import datetime

    dt = datetime.datetime.fromtimestamp(ts_ms / 1000, tz=datetime.UTC)
    quarter = (dt.month - 1) // 3 + 1
    return f"{dt.year}Q{quarter}"


def news_impact_persistence(
    graph: KnowledgeGraph,
    *,
    event_type_name: str,
    symbol: str,
    horizon_min: int,
    expected_direction: str,
) -> list[NewsImpactPeriod]:
    """§7.2 query #2: is `event_type_name`'s effect on `symbol` stable or
    decaying over time? Bucketed by quarter so a caller can plot the
    stability chart §12.4 calls for."""
    by_quarter: dict[str, list[tuple[float, bool]]] = defaultdict(list)

    for news_event in graph.in_neighbors(event_type_name, "OF_TYPE"):
        _of_type_edge, event_node = news_event
        mentions = {n.id for _e, n in graph.out_neighbors(event_node.id, "MENTIONS")}
        if symbol not in mentions:
            continue
        for _preceded_edge, outcome in graph.out_neighbors(event_node.id, "PRECEDED"):
            if outcome.properties.get("horizon_min") != horizon_min:
                continue
            move_atr = abs(_as_float(outcome.properties.get("move_atr")))
            direction_hit = outcome.properties.get("direction") == expected_direction
            quarter = _quarter_key(_as_int(event_node.properties["ts"]))
            by_quarter[quarter].append((move_atr, direction_hit))

    results = []
    for quarter, samples in sorted(by_quarter.items()):
        n = len(samples)
        avg_impact = sum(s[0] for s in samples) / n
        direction_hit_rate = sum(1.0 for s in samples if s[1]) / n
        results.append(
            NewsImpactPeriod(
                quarter=quarter, n=n, avg_impact=avg_impact, direction_hit_rate=direction_hit_rate
            )
        )
    return results


@dataclass(frozen=True)
class ConfluenceResult:
    pattern_a_name: str
    pattern_b_name: str
    n: int
    combo_r: float


def _pattern_name_of(graph: KnowledgeGraph, pattern_inst_id: str) -> str | None:
    patterns = graph.out_neighbors(pattern_inst_id, "INSTANCE_OF")
    if not patterns:
        return None
    _edge, pattern = patterns[0]
    name = pattern.properties.get("name")
    return name if isinstance(name, str) else None


def _r_multiple_of(graph: KnowledgeGraph, pattern_inst_id: str) -> float | None:
    resolved = graph.out_neighbors(pattern_inst_id, "RESOLVED_AS")
    if not resolved:
        return None
    edge, _outcome = resolved[0]
    r_multiple = edge.properties.get("r_multiple")
    return float(r_multiple) if isinstance(r_multiple, (int, float)) else None


def confluence_discovery(
    graph: KnowledgeGraph, *, max_lag_bars: int = 3, min_n: int = 40, limit: int = 20
) -> list[ConfluenceResult]:
    """§7.2 query #3: which pattern-pair combinations beat their parts?
    Only pairs with `n >= min_n` resolved co-occurrences are returned, and
    only the top `limit` by `combo_r`, matching the design doc's own
    `WHERE n >= 40 ... ORDER BY combo_r DESC LIMIT 20`."""
    combo_r_by_pair: dict[tuple[str, str], list[float]] = defaultdict(list)

    for pattern_inst_a in graph.nodes_by_label("PatternInst"):
        for edge, pattern_inst_b in graph.out_neighbors(pattern_inst_a.id, "CO_OCCURRED_WITH"):
            lag_bars = edge.properties.get("lag_bars", 0)
            if not isinstance(lag_bars, int) or abs(lag_bars) > max_lag_bars:
                continue
            name_a = _pattern_name_of(graph, pattern_inst_a.id)
            name_b = _pattern_name_of(graph, pattern_inst_b.id)
            r_a = _r_multiple_of(graph, pattern_inst_a.id)
            r_b = _r_multiple_of(graph, pattern_inst_b.id)
            if name_a is None or name_b is None or r_a is None or r_b is None:
                continue
            combo_r_by_pair[(name_a, name_b)].append((r_a + r_b) / 2.0)

    results = [
        ConfluenceResult(
            pattern_a_name=pa,
            pattern_b_name=pb,
            n=len(combo_rs),
            combo_r=sum(combo_rs) / len(combo_rs),
        )
        for (pa, pb), combo_rs in combo_r_by_pair.items()
        if len(combo_rs) >= min_n
    ]
    results.sort(key=lambda r: r.combo_r, reverse=True)
    return results[:limit]
