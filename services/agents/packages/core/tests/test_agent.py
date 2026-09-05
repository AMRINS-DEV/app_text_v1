import pytest
from agents_core import AgentOutput
from pydantic import ValidationError


def test_agent_output_rejects_out_of_range_probability():
    with pytest.raises(ValidationError):
        AgentOutput(
            direction="Long",
            probability=1.5,
            confidence=0.5,
            expected_r=0.4,
            horizon_ms=60_000,
            regime="Trending",
        )


def test_agent_output_accepts_valid_payload():
    out = AgentOutput(
        direction="Long",
        probability=0.58,
        confidence=0.7,
        expected_r=0.4,
        horizon_ms=60_000,
        regime="Trending",
    )
    assert out.probability == 0.58
