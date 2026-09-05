from agents_graph import (
    KnowledgeGraph,
    instance_of_edge,
    instrument_node,
    on_edge,
    pattern_inst_node,
    pattern_node,
)


def _graph_with_two_pattern_instances_on_the_same_pattern() -> tuple[KnowledgeGraph, str, str, str]:
    graph = KnowledgeGraph()
    graph.upsert_node(
        instrument_node(
            "EURUSD", asset_class="fx", base="EUR", quote="USD", tick_size=1e-5, sessions=[]
        )
    )
    graph.upsert_node(
        pattern_node(
            "p-1", name="double_top", family="reversal", timeframe="M5", direction_bias="short"
        )
    )
    pi_1 = pattern_inst_node(
        "pi-1", ts_start=0, ts_end=10, symbol="EURUSD", confidence=0.7, detected_by="pattern-agent"
    )
    pi_2 = pattern_inst_node(
        "pi-2", ts_start=20, ts_end=30, symbol="EURUSD", confidence=0.6, detected_by="pattern-agent"
    )
    graph.upsert_node(pi_1)
    graph.upsert_node(pi_2)
    graph.upsert_edge(instance_of_edge("pi-1", "p-1"))
    graph.upsert_edge(instance_of_edge("pi-2", "p-1"))
    graph.upsert_edge(on_edge("pi-1", "EURUSD"))
    graph.upsert_edge(on_edge("pi-2", "EURUSD"))
    return graph, "p-1", "pi-1", "pi-2"


def test_upsert_node_is_idempotent_by_id():
    graph = KnowledgeGraph()
    graph.upsert_node(
        instrument_node(
            "EURUSD", asset_class="fx", base="EUR", quote="USD", tick_size=1e-5, sessions=[]
        )
    )
    graph.upsert_node(
        instrument_node(
            "EURUSD", asset_class="fx", base="EUR", quote="USD", tick_size=2e-5, sessions=["london"]
        )
    )
    assert graph.node_count() == 1
    node = graph.get_node("EURUSD")
    assert node is not None
    assert node.properties["tick_size"] == 2e-5


def test_upsert_edge_is_idempotent_by_label_source_target():
    graph, pattern_id, pi_1, pi_2 = _graph_with_two_pattern_instances_on_the_same_pattern()
    assert graph.edge_count() == 4  # 2x INSTANCE_OF + 2x ON
    graph.upsert_edge(instance_of_edge(pi_1, pattern_id))  # re-upsert, not a new edge
    assert graph.edge_count() == 4


def test_in_neighbors_finds_all_pattern_instances_of_a_pattern():
    graph, pattern_id, pi_1, pi_2 = _graph_with_two_pattern_instances_on_the_same_pattern()
    instances = graph.in_neighbors(pattern_id, "INSTANCE_OF")
    found_ids = {node.id for _edge, node in instances}
    assert found_ids == {pi_1, pi_2}


def test_out_neighbors_finds_the_pattern_a_pattern_instance_belongs_to():
    graph, pattern_id, pi_1, _pi_2 = _graph_with_two_pattern_instances_on_the_same_pattern()
    patterns = graph.out_neighbors(pi_1, "INSTANCE_OF")
    assert [node.id for _edge, node in patterns] == [pattern_id]


def test_nodes_by_label_only_returns_that_label():
    graph, _pattern_id, _pi_1, _pi_2 = _graph_with_two_pattern_instances_on_the_same_pattern()
    assert {n.id for n in graph.nodes_by_label("PatternInst")} == {"pi-1", "pi-2"}
    assert {n.id for n in graph.nodes_by_label("Pattern")} == {"p-1"}


def test_missing_node_lookup_returns_none():
    assert KnowledgeGraph().get_node("does-not-exist") is None


def test_neighbor_lookup_on_a_node_with_no_such_edges_is_empty():
    graph, pattern_id, _pi_1, _pi_2 = _graph_with_two_pattern_instances_on_the_same_pattern()
    assert graph.out_neighbors(pattern_id, "RESOLVED_AS") == []
