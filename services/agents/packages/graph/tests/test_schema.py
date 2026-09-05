from agents_graph import (
    instrument_node,
    mentions_edge,
    news_event_node,
    pattern_inst_node,
    resolved_as_edge,
)


def test_instrument_node_is_keyed_by_symbol_with_no_separate_id_property():
    node = instrument_node(
        "EURUSD",
        asset_class="fx",
        base="EUR",
        quote="USD",
        tick_size=0.00001,
        sessions=["london", "ny"],
    )
    assert node.id == "EURUSD"
    assert node.label == "Instrument"
    assert "id" not in node.properties
    assert node.properties["sessions"] == ["london", "ny"]


def test_news_event_node_carries_every_7_1_property():
    node = news_event_node(
        "ev-1",
        ts=1_000,
        headline="Rate decision",
        source="test-feed",
        impact_tier="high",
        embedding_id=None,
        sentiment=-0.4,
    )
    assert node.id == "ev-1"
    assert set(node.properties) == {
        "id",
        "ts",
        "headline",
        "source",
        "impact_tier",
        "embedding_id",
        "sentiment",
    }


def test_mentions_edge_carries_salience_and_points_news_event_to_instrument():
    edge = mentions_edge("ev-1", "EURUSD", salience=0.8)
    assert edge.label == "MENTIONS"
    assert edge.source_id == "ev-1"
    assert edge.target_id == "EURUSD"
    assert edge.properties == {"salience": 0.8}


def test_resolved_as_edge_carries_confirmed_and_r_multiple():
    pattern_inst = pattern_inst_node(
        "pi-1", ts_start=0, ts_end=10, symbol="EURUSD", confidence=0.7, detected_by="pattern-agent"
    )
    edge = resolved_as_edge(pattern_inst.id, "out-1", confirmed=True, r_multiple=2.1)
    assert edge.source_id == "pi-1"
    assert edge.target_id == "out-1"
    assert edge.properties == {"confirmed": True, "r_multiple": 2.1}
