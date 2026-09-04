"""Foundation for every agent (§10). `BaseAgent`'s shape and the
calibrated-signal contract every agent must emit; §10.3's signal bus
(TTL + features_hash + calibrated-range gating) and §10.4's guardrails
(numeric cross-check, prompt-injection isolation, model-version-in-
calibration-key) are real as of Phase 5. Memory and a full eval harness
remain later scope.
"""

from .agent import AgentInput, AgentOutput, BaseAgent
from .guardrails import ModelVersionWeight, calibration_key, implausible_levels, wrap_untrusted_text
from .signal_bus import PublishedSignal, SignalBus, SignalRejected

__all__ = [
    "AgentInput",
    "AgentOutput",
    "BaseAgent",
    "ModelVersionWeight",
    "calibration_key",
    "implausible_levels",
    "wrap_untrusted_text",
    "PublishedSignal",
    "SignalBus",
    "SignalRejected",
]
