from agents_models import Barrier, triple_barrier


def test_tp_hit_first_labels_win():
    prices = [100.0, 101.0, 103.0, 100.0]  # atr=1, tp_mult=2.2 -> upper=102.2
    result = triple_barrier(prices, t0=0, atr=1.0, tp_mult=2.2, sl_mult=1.5, max_bars=10)
    assert result.label == Barrier.WIN
    assert result.bars_to_resolution == 1


def test_sl_hit_first_labels_loss():
    prices = [100.0, 99.0, 98.0, 90.0]  # atr=1, sl_mult=1.5 -> lower=98.5
    result = triple_barrier(prices, t0=0, atr=1.0, tp_mult=2.2, sl_mult=1.5, max_bars=10)
    assert result.label == Barrier.LOSS


def test_neither_barrier_hit_labels_timeout():
    prices = [100.0, 100.1, 99.9, 100.05]
    result = triple_barrier(prices, t0=0, atr=1.0, tp_mult=2.2, sl_mult=1.5, max_bars=3)
    assert result.label == Barrier.TIMEOUT
