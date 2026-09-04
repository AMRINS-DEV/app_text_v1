"""LangGraph orchestration (§2, §10). Wires the agent roster into a
stateful, checkpointed graph and publishes fused `Signal`s to NATS
(`signal.{symbol}.{agent_kind}`, §10.3). Phase 5 scope."""

from agents_core import BaseAgent

AGENT_ROSTER: dict[str, type[BaseAgent]] = {}
"""Populated in Phase 5 as each agent package registers itself."""
