from agents_graph import (
    FixedHorizonMove,
    KnowledgeGraph,
    OutcomeResolution,
    Verdict,
    attach_news_outcome,
    attach_pattern_outcome,
    ingest_market_regime,
    ingest_news_event,
    ingest_pattern_instance,
    ingest_trade,
    instrument_node,
    link_co_occurring_patterns,
    pattern_inst_node,
)


def _seed_instrument(graph: KnowledgeGraph, symbol: str) -> None:
    graph.upsert_node(
        instrument_node(
            symbol, asset_class="fx", base=symbol[:3], quote=symbol[3:], tick_size=1e-5, sessions=[]
        )
    )


def test_ingest_news_event_links_event_type_and_every_mentioned_instrument():
    graph = KnowledgeGraph()
    _seed_instrument(graph, "EURUSD")
    _seed_instrument(graph, "GBPUSD")
    ingest_news_event(
        graph,
        id="ev-1",
        ts=1_000,
        headline="Rate decision",
        source="test-feed",
        impact_tier="high",
        sentiment=-0.4,
        event_type="rate_decision",
        instruments=["EURUSD", "GBPUSD"],
        salience=0.9,
    )
    assert graph.get_node("ev-1") is not None
    assert graph.get_node("rate_decision") is not None
    assert [n.id for _e, n in graph.out_neighbors("ev-1", "OF_TYPE")] == ["rate_decision"]
    mentioned = {n.id for _e, n in graph.out_neighbors("ev-1", "MENTIONS")}
    assert mentioned == {"EURUSD", "GBPUSD"}


def test_ingest_news_event_is_idempotent():
    graph = KnowledgeGraph()
    for _ in range(2):
        ingest_news_event(
            graph,
            id="ev-1",
            ts=1_000,
            headline="Rate decision",
            source="test-feed",
            impact_tier="high",
            sentiment=-0.4,
            event_type="rate_decision",
            instruments=["EURUSD"],
        )
    # NewsEvent + EventType (MENTIONS targets a symbol, no Instrument node)
    assert graph.node_count() == 2
    assert graph.edge_count() == 2  # OF_TYPE + MENTIONS, not duplicated


def test_ingest_pattern_instance_links_pattern_instrument_and_regime():
    graph = KnowledgeGraph()
    _seed_instrument(graph, "EURUSD")
    ingest_market_regime(
        graph,
        id="regime-1",
        label="Trending",
        ts_start=0,
        ts_end=100,
        vol_bucket="mid",
        trend_strength=0.8,
    )
    ingest_pattern_instance(
        graph,
        id="pi-1",
        ts_start=10,
        ts_end=20,
        symbol="EURUSD",
        confidence=0.7,
        detected_by="pattern-agent",
        pattern_id="p-double-top",
        pattern_name="double_top",
        pattern_family="reversal",
        timeframe="M5",
        direction_bias="short",
        market_regime_id="regime-1",
    )
    assert [n.id for _e, n in graph.out_neighbors("pi-1", "INSTANCE_OF")] == ["p-double-top"]
    assert [n.id for _e, n in graph.out_neighbors("pi-1", "ON")] == ["EURUSD"]
    assert [n.id for _e, n in graph.out_neighbors("pi-1", "DURING")] == ["regime-1"]


def test_ingest_trade_links_triggered_by():
    graph = KnowledgeGraph()
    graph.upsert_node(
        pattern_inst_node(
            "pi-1",
            ts_start=0,
            ts_end=10,
            symbol="EURUSD",
            confidence=0.7,
            detected_by="pattern-agent",
        )
    )
    ingest_trade(
        graph,
        id="trade-1",
        ts_open=100,
        ts_close=200,
        side="short",
        r_multiple=2.1,
        pnl=210.0,
        mode="paper",
        triggered_by_id="pi-1",
    )
    assert [n.id for _e, n in graph.out_neighbors("trade-1", "TRIGGERED_BY")] == ["pi-1"]


def test_link_co_occurring_patterns_is_directional():
    graph = KnowledgeGraph()
    graph.upsert_node(
        pattern_inst_node(
            "pi-1",
            ts_start=0,
            ts_end=10,
            symbol="EURUSD",
            confidence=0.7,
            detected_by="pattern-agent",
        )
    )
    graph.upsert_node(
        pattern_inst_node(
            "pi-2",
            ts_start=5,
            ts_end=15,
            symbol="EURUSD",
            confidence=0.6,
            detected_by="pattern-agent",
        )
    )
    link_co_occurring_patterns(graph, "pi-1", "pi-2", lag_bars=3)
    assert [n.id for _e, n in graph.out_neighbors("pi-1", "CO_OCCURRED_WITH")] == ["pi-2"]
    assert graph.out_neighbors("pi-2", "CO_OCCURRED_WITH") == []


def test_attach_pattern_outcome_writes_confirmed_flag_and_r_multiple():
    graph = KnowledgeGraph()
    resolution = OutcomeResolution(
        verdict=Verdict.CONFIRMED,
        bars_to_resolution=5,
        mfe=4.5,
        mae=0.5,
        r_multiple=2.0,
        move_pips=40.0,
        move_atr=2.0,
        direction="Long",
    )
    attach_pattern_outcome(
        graph, pattern_inst_id="pi-1", outcome_id="out-1", resolution=resolution, horizon_min=25
    )
    outcome = graph.get_node("out-1")
    assert outcome is not None
    assert outcome.properties["horizon_min"] == 25
    assert outcome.properties["move_atr"] == 2.0
    resolved_edges = graph.out_neighbors("pi-1", "RESOLVED_AS")
    assert len(resolved_edges) == 1
    edge, node = resolved_edges[0]
    assert node.id == "out-1"
    assert edge.properties == {"confirmed": True, "r_multiple": 2.0}


def test_attach_news_outcome_writes_preceded_edge_with_lag():
    graph = KnowledgeGraph()
    move = FixedHorizonMove(move_pips=50.0, move_atr=2.5, direction="Long", direction_hit=True)
    attach_news_outcome(
        graph, news_event_id="ev-1", outcome_id="out-news-1", move=move, horizon_min=15, lag_min=15
    )
    edges = graph.out_neighbors("ev-1", "PRECEDED")
    assert len(edges) == 1
    edge, node = edges[0]
    assert node.id == "out-news-1"
    assert edge.properties == {"lag_min": 15}
