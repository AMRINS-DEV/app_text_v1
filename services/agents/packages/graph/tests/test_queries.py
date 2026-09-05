from agents_graph import (
    KnowledgeGraph,
    OutcomeResolution,
    Verdict,
    attach_news_outcome,
    attach_pattern_outcome,
    conditional_reliability,
    confluence_discovery,
    ingest_market_regime,
    ingest_news_event,
    ingest_pattern_instance,
    instrument_node,
    link_co_occurring_patterns,
    news_impact_persistence,
    resolve_fixed_horizon_move,
)


def _seed_pattern_instance(
    graph: KnowledgeGraph,
    *,
    inst_id: str,
    ts_start: int,
    confirmed: bool,
    r_multiple: float,
    symbol: str = "EURUSD",
    regime_label: str = "Trending",
) -> None:
    ingest_market_regime(
        graph,
        id=f"regime-{ts_start}",
        label=regime_label,
        ts_start=ts_start,
        ts_end=ts_start + 100,
        vol_bucket="mid",
        trend_strength=0.5,
    )
    ingest_pattern_instance(
        graph,
        id=inst_id,
        ts_start=ts_start,
        ts_end=ts_start + 10,
        symbol=symbol,
        confidence=0.7,
        detected_by="pattern-agent",
        pattern_id="p-double-top",
        pattern_name="double_top",
        pattern_family="reversal",
        timeframe="M5",
        direction_bias="short",
        market_regime_id=f"regime-{ts_start}",
    )
    # Built directly rather than via resolve_pattern_outcome — this test is
    # about the *query* reading RESOLVED_AS edges correctly, not about the
    # resolver's own bar-walking logic (that's test_outcomes.py's job).
    resolution = OutcomeResolution(
        verdict=Verdict.CONFIRMED if confirmed else Verdict.FAILED,
        bars_to_resolution=1,
        mfe=abs(r_multiple) * 2.0,
        mae=abs(r_multiple),
        r_multiple=r_multiple,
        move_pips=r_multiple * 10.0,
        move_atr=r_multiple,
        direction="Long",
    )
    attach_pattern_outcome(
        graph,
        pattern_inst_id=inst_id,
        outcome_id=f"out-{inst_id}",
        resolution=resolution,
        horizon_min=15,
    )


def test_conditional_reliability_aggregates_only_matching_instances():
    graph = KnowledgeGraph()
    # Two matching instances: one confirmed (r=2.0), one failed (r=-1.0).
    _seed_pattern_instance(graph, inst_id="pi-1", ts_start=1_000, confirmed=True, r_multiple=2.0)
    _seed_pattern_instance(graph, inst_id="pi-2", ts_start=2_000, confirmed=False, r_multiple=-1.0)
    # A non-matching instance: wrong symbol.
    _seed_pattern_instance(
        graph, inst_id="pi-3", ts_start=3_000, confirmed=True, r_multiple=3.0, symbol="GBPUSD"
    )
    # A non-matching instance: wrong regime.
    _seed_pattern_instance(
        graph,
        inst_id="pi-4",
        ts_start=4_000,
        confirmed=True,
        r_multiple=3.0,
        regime_label="Ranging",
    )
    # A non-matching instance: too old (before since_ts).
    _seed_pattern_instance(graph, inst_id="pi-5", ts_start=500, confirmed=True, r_multiple=3.0)

    result = conditional_reliability(
        graph, pattern_name="double_top", symbol="EURUSD", regime_label="Trending", since_ts=900
    )

    assert result.n == 2
    assert result.hit_rate == 0.5
    assert result.avg_r == 0.5  # (2.0 + -1.0) / 2
    assert result.median_r == 0.5  # only two values -> midpoint


def test_conditional_reliability_with_no_matches_reports_n_zero():
    graph = KnowledgeGraph()
    result = conditional_reliability(
        graph,
        pattern_name="head_and_shoulders",
        symbol="EURUSD",
        regime_label="Trending",
        since_ts=0,
    )
    assert result.n == 0
    assert result.hit_rate == 0.0


def test_news_impact_persistence_buckets_by_quarter_and_filters_horizon():
    graph = KnowledgeGraph()
    graph.upsert_node(
        instrument_node(
            "EURUSD", asset_class="fx", base="EUR", quote="USD", tick_size=1e-5, sessions=[]
        )
    )
    ingest_news_event(
        graph,
        id="ev-q1",
        ts=1_700_000_000_000,  # 2023-11-14 (Q4)
        headline="x",
        source="test",
        impact_tier="high",
        sentiment=0.0,
        event_type="rate_decision",
        instruments=["EURUSD"],
    )
    move = resolve_fixed_horizon_move(
        price_at_event=1.10,
        price_at_horizon=1.11,
        expected_direction="Long",
        pip_size=0.0001,
        atr=0.002,
    )
    attach_news_outcome(
        graph, news_event_id="ev-q1", outcome_id="out-q1", move=move, horizon_min=15, lag_min=15
    )
    # A second outcome at a *different* horizon on the same event — must
    # not be counted when we ask for horizon_min=15.
    attach_news_outcome(
        graph, news_event_id="ev-q1", outcome_id="out-q1-60m", move=move, horizon_min=60, lag_min=60
    )

    periods = news_impact_persistence(
        graph,
        event_type_name="rate_decision",
        symbol="EURUSD",
        horizon_min=15,
        expected_direction="Long",
    )

    assert len(periods) == 1
    assert periods[0].n == 1
    assert periods[0].direction_hit_rate == 1.0
    assert periods[0].quarter == "2023Q4"


def test_confluence_discovery_requires_the_minimum_sample_size():
    graph = KnowledgeGraph()
    for i in range(39):
        _seed_pattern_instance(graph, inst_id=f"a-{i}", ts_start=i, confirmed=True, r_multiple=1.5)
        _seed_pattern_instance(graph, inst_id=f"b-{i}", ts_start=i, confirmed=True, r_multiple=2.5)
        link_co_occurring_patterns(graph, f"a-{i}", f"b-{i}", lag_bars=1)

    # Only 39 co-occurrences -- below the n>=40 gate.
    assert confluence_discovery(graph, min_n=40) == []

    # A 40th pushes it over the gate.
    _seed_pattern_instance(graph, inst_id="a-39", ts_start=39, confirmed=True, r_multiple=1.5)
    _seed_pattern_instance(graph, inst_id="b-39", ts_start=39, confirmed=True, r_multiple=2.5)
    link_co_occurring_patterns(graph, "a-39", "b-39", lag_bars=1)

    results = confluence_discovery(graph, min_n=40)
    assert len(results) == 1
    assert results[0].n == 40
    assert results[0].combo_r == 2.0  # (1.5 + 2.5) / 2


def test_confluence_discovery_excludes_pairs_outside_the_lag_window():
    graph = KnowledgeGraph()
    for i in range(40):
        _seed_pattern_instance(graph, inst_id=f"a-{i}", ts_start=i, confirmed=True, r_multiple=1.0)
        _seed_pattern_instance(graph, inst_id=f"b-{i}", ts_start=i, confirmed=True, r_multiple=1.0)
        link_co_occurring_patterns(graph, f"a-{i}", f"b-{i}", lag_bars=10)  # outside max_lag_bars=3

    assert confluence_discovery(graph, max_lag_bars=3, min_n=40) == []
