import pytest
from agents_llm import LlmRouter


def test_route_returns_configured_primary():
    router = LlmRouter()
    assert router.route("news_triage") == "deepseek"
    assert router.route("critic") == "claude"


def test_unknown_task_class_raises():
    router = LlmRouter()
    with pytest.raises(ValueError):
        router.route("not_a_real_task_class")
