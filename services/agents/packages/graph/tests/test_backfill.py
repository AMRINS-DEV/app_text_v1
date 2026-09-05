from agents_graph import build_backfilled_graph


def test_same_seed_produces_an_identical_graph():
    graph_a = build_backfilled_graph(pattern_count=50, news_count=20, seed=7)
    graph_b = build_backfilled_graph(pattern_count=50, news_count=20, seed=7)
    assert graph_a.node_count() == graph_b.node_count()
    assert graph_a.edge_count() == graph_b.edge_count()


def test_different_seeds_produce_different_content():
    graph_a = build_backfilled_graph(pattern_count=50, news_count=20, seed=7)
    graph_b = build_backfilled_graph(pattern_count=50, news_count=20, seed=8)
    # Same shape (deterministic counts), different content.
    assert graph_a.node_count() == graph_b.node_count()
    patterns_a = {n.id: n.properties for n in graph_a.nodes_by_label("PatternInst")}
    patterns_b = {n.id: n.properties for n in graph_b.nodes_by_label("PatternInst")}
    assert patterns_a != patterns_b


def test_backfilled_graph_has_the_requested_counts():
    graph = build_backfilled_graph(pattern_count=50, news_count=20, seed=1)
    assert len(list(graph.nodes_by_label("PatternInst"))) == 50
    assert len(list(graph.nodes_by_label("NewsEvent"))) == 20
    assert len(list(graph.nodes_by_label("Instrument"))) == 3


def _hit_rate_by_pattern_name(graph) -> dict[str, float]:  # type: ignore[no-untyped-def]
    confirmed_counts: dict[str, int] = {}
    total_counts: dict[str, int] = {}
    for pattern in graph.nodes_by_label("Pattern"):
        name = pattern.properties["name"]
        for _e, pattern_inst in graph.in_neighbors(pattern.id, "INSTANCE_OF"):
            for edge, _outcome in graph.out_neighbors(pattern_inst.id, "RESOLVED_AS"):
                total_counts[name] = total_counts.get(name, 0) + 1
                if edge.properties.get("confirmed"):
                    confirmed_counts[name] = confirmed_counts.get(name, 0) + 1
    return {name: confirmed_counts.get(name, 0) / total for name, total in total_counts.items()}


def test_double_top_has_a_higher_seeded_hit_rate_than_double_bottom():
    graph = build_backfilled_graph(pattern_count=2000, news_count=10, seed=42)
    hit_rates = _hit_rate_by_pattern_name(graph)
    # Seeded at 0.62 vs 0.45 (see backfill.py) — at ~1000 instances each,
    # the gap should be unmistakable, not just noise.
    assert hit_rates["double_top"] > hit_rates["double_bottom"] + 0.10
