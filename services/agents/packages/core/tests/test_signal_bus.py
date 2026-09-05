import pytest
from agents_core import AgentOutput, PublishedSignal, SignalBus, SignalRejected


def make_signal(
    *, features_hash: str = "hash-1", published_at_ns: int = 1_000, ttl_ns: int = 500
) -> PublishedSignal:
    return PublishedSignal(
        symbol_id=1,
        agent_kind="news-agent",
        output=AgentOutput(
            direction="Long",
            probability=0.6,
            confidence=0.7,
            expected_r=0.5,
            horizon_ms=60_000,
            regime="Trending",
        ),
        features_hash=features_hash,
        published_at_ns=published_at_ns,
        ttl_ns=ttl_ns,
    )


def test_publish_accepts_a_signal_against_a_known_feature_snapshot_within_ttl():
    bus = SignalBus()
    bus.register_feature_snapshot("hash-1")
    bus.publish(make_signal(), now_ns=1_200)
    assert len(bus.active_signals(now_ns=1_200)) == 1


def test_publish_rejects_an_unknown_features_hash():
    bus = SignalBus()
    # Note: hash-1 is never registered.
    with pytest.raises(SignalRejected, match="features_hash"):
        bus.publish(make_signal(), now_ns=1_200)


def test_publish_rejects_a_signal_past_its_ttl():
    bus = SignalBus()
    bus.register_feature_snapshot("hash-1")
    with pytest.raises(SignalRejected, match="expired"):
        bus.publish(make_signal(published_at_ns=1_000, ttl_ns=100), now_ns=2_000)


def test_publish_rejects_a_probability_outside_the_calibrated_range():
    bus = SignalBus()
    bus.register_feature_snapshot("hash-1")
    with pytest.raises(SignalRejected, match="calibrated range"):
        bus.publish(make_signal(), now_ns=1_200, calibrated_range=(0.7, 0.9))


def test_publish_accepts_a_probability_inside_the_calibrated_range():
    bus = SignalBus()
    bus.register_feature_snapshot("hash-1")
    bus.publish(make_signal(), now_ns=1_200, calibrated_range=(0.5, 0.9))
    assert len(bus.active_signals(now_ns=1_200)) == 1


def test_active_signals_excludes_expired_ones():
    bus = SignalBus()
    bus.register_feature_snapshot("hash-1")
    bus.publish(make_signal(published_at_ns=1_000, ttl_ns=500), now_ns=1_100)
    assert len(bus.active_signals(now_ns=1_100)) == 1
    assert len(bus.active_signals(now_ns=1_600)) == 0


def test_subscribers_are_notified_on_publish():
    bus = SignalBus()
    bus.register_feature_snapshot("hash-1")
    received = []
    bus.subscribe(received.append)

    bus.publish(make_signal(), now_ns=1_200)

    assert len(received) == 1
    assert received[0].agent_kind == "news-agent"


def test_unsubscribe_stops_further_notifications():
    bus = SignalBus()
    bus.register_feature_snapshot("hash-1")
    received = []
    unsubscribe = bus.subscribe(received.append)
    unsubscribe()

    bus.publish(make_signal(), now_ns=1_200)

    assert received == []


def test_a_rejected_signal_never_reaches_subscribers():
    bus = SignalBus()
    received = []
    bus.subscribe(received.append)

    with pytest.raises(SignalRejected):
        bus.publish(make_signal(), now_ns=1_200)  # unknown features_hash

    assert received == []
