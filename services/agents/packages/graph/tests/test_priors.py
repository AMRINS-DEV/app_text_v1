from agents_graph import ConditionalReliability, graph_prior_to_fusion_input


def test_a_well_powered_prior_gets_weight_one():
    reliability = ConditionalReliability(n=87, hit_rate=0.65, avg_r=1.3, median_r=1.2)
    fusion_input = graph_prior_to_fusion_input(reliability, source_id="graph-prior")
    assert fusion_input.source_id == "graph-prior"
    assert fusion_input.probability == 0.65
    assert fusion_input.weight == 1.0
    assert fusion_input.resolved_predictions == 87


def test_an_under_powered_prior_gets_weight_zero():
    reliability = ConditionalReliability(n=5, hit_rate=0.9, avg_r=3.0, median_r=3.0)
    fusion_input = graph_prior_to_fusion_input(reliability)
    assert fusion_input.weight == 0.0
    assert fusion_input.resolved_predictions == 5


def test_exactly_at_the_gate_boundary_gets_weight_one():
    reliability = ConditionalReliability(n=30, hit_rate=0.5, avg_r=0.0, median_r=0.0)
    fusion_input = graph_prior_to_fusion_input(reliability)
    assert fusion_input.weight == 1.0
