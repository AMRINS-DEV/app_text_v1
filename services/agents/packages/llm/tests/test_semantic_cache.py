from agents_llm import CompletionResponse
from agents_llm.semantic_cache import SemanticCache


def response(text: str) -> CompletionResponse:
    return CompletionResponse(text=text, input_tokens=1, output_tokens=1, model="test")


def test_exact_repeat_is_a_cache_hit():
    cache = SemanticCache()
    cache.store("what is the impact of a 50bps rate cut on EURUSD", response("bullish EUR"))
    assert cache.lookup("what is the impact of a 50bps rate cut on EURUSD") is not None


def test_completely_unrelated_prompt_is_a_miss():
    cache = SemanticCache()
    cache.store("what is the impact of a 50bps rate cut on EURUSD", response("bullish EUR"))
    assert cache.lookup("describe the double-top pattern on the daily chart") is None


def test_a_near_duplicate_prompt_still_hits():
    cache = SemanticCache()
    cache.store(
        "What is the market impact of US CPI on USDJPY?",
        response("USDJPY likely weakens"),
    )
    # Same underlying question, close paraphrase — high word overlap.
    hit = cache.lookup("What's the likely market impact of US CPI on USDJPY?")
    assert hit is not None
    assert hit.text == "USDJPY likely weakens"


def test_hit_rate_across_a_realistic_mixed_workload_meets_the_40pct_target():
    # A bag-of-words cache (no real embedding model — see this module's
    # doc comment) genuinely catches close paraphrases of an already-cached
    # question, but not a heavily-reworded one; this workload mixes both,
    # like a real multi-agent system would produce (several agents asking
    # about the same breaking event in slightly different wording, plus
    # genuinely novel one-off questions).
    cache = SemanticCache()
    near_duplicate_templates = [
        "What is the market impact of {event} on {pair}?",
        "What's the likely market impact of {event} on {pair}?",
        "Please summarize the market impact of {event} on {pair}.",
    ]
    events = ["US CPI", "ECB rate decision", "NFP release", "FOMC minutes"]
    pairs = ["EURUSD", "GBPUSD", "USDJPY", "XAUUSD"]

    for event in events:
        for pair in pairs:
            cache.store(
                near_duplicate_templates[0].format(event=event, pair=pair),
                response(f"{event}/{pair} analysis"),
            )

    # Near-duplicate re-phrasings of already-cached questions — should hit.
    for event in events:
        for pair in pairs:
            for template in near_duplicate_templates:
                cache.lookup(template.format(event=event, pair=pair))

    # Genuinely novel one-off questions sharing no cached content — should miss.
    for i in range(len(events) * len(pairs)):
        cache.lookup(f"describe pattern instance #{i} on the 4h chart with no prior context")

    assert cache.hit_rate() >= 0.40, cache.stats()
