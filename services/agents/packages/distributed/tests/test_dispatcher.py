from __future__ import annotations

import asyncio

import pytest
from agents_distributed import Job, run_distributed
from agents_distributed.registry import AGENT_FACTORIES, INPUT_BUILDERS


def _job(job_id: str, feature: tuple[float, float]) -> Job:
    return Job(
        job_id=job_id,
        agent_kind="regime-agent",
        payload={"symbol_id": 1, "as_of_ns": 0, "feature_history": [list(feature)]},
    )


# A spread of distinct (return, volatility) pairs so different jobs are not
# trivially guaranteed to classify the same way.
_SAMPLE_FEATURES: list[tuple[float, float]] = [
    (0.0005, 0.008),
    (-0.02, 0.05),
    (0.0, 0.001),
    (0.01, 0.03),
    (-0.001, 0.009),
    (0.03, 0.06),
]


def test_a_single_job_matches_a_plain_in_process_call_to_the_same_agent():
    """The distributed path must not change *what* gets computed, only
    *where* -- direct parity check against calling the agent the normal,
    undistributed way."""
    job = _job("j0", _SAMPLE_FEATURES[0])

    [result] = run_distributed([job], n_workers=1)
    assert result.succeeded

    agent = AGENT_FACTORIES["regime-agent"]()
    agent_input = INPUT_BUILDERS["regime-agent"](job.payload)
    direct_output = asyncio.run(agent.run(agent_input)).model_dump()

    assert result.output == direct_output


def test_results_are_identical_regardless_of_worker_count():
    jobs = [_job(f"job-{i}", feature) for i, feature in enumerate(_SAMPLE_FEATURES)]

    with_one_worker = run_distributed(jobs, n_workers=1)
    with_four_workers = run_distributed(jobs, n_workers=4)

    as_map_one = {r.job_id: r.output for r in with_one_worker}
    as_map_four = {r.job_id: r.output for r in with_four_workers}
    assert as_map_one == as_map_four
    assert all(r.succeeded for r in with_one_worker)
    assert all(r.succeeded for r in with_four_workers)


def test_work_is_genuinely_spread_across_more_than_one_process():
    jobs = [_job(f"job-{i}", feature) for i, feature in enumerate(_SAMPLE_FEATURES)]
    results = run_distributed(jobs, n_workers=3)

    distinct_worker_ids = {r.worker_id for r in results}
    assert len(distinct_worker_ids) > 1, "expected work to spread across multiple worker processes"


def test_every_job_id_appears_exactly_once_in_the_results():
    jobs = [_job(f"job-{i}", feature) for i, feature in enumerate(_SAMPLE_FEATURES)]
    results = run_distributed(jobs, n_workers=3)
    result_ids = sorted(r.job_id for r in results)
    assert result_ids == sorted(j.job_id for j in jobs)


def test_an_unknown_agent_kind_reports_a_failed_result_not_a_crash():
    bad_job = Job(job_id="bad", agent_kind="does-not-exist", payload={})
    [result] = run_distributed([bad_job], n_workers=1)
    assert not result.succeeded
    assert "unknown agent_kind" in result.error


def test_a_malformed_payload_reports_a_failed_result_not_a_crash():
    bad_job = Job(
        job_id="bad-payload", agent_kind="regime-agent", payload={"missing": "required fields"}
    )
    [result] = run_distributed([bad_job], n_workers=1)
    assert not result.succeeded
    assert result.error is not None


def test_n_workers_below_one_is_rejected():
    with pytest.raises(ValueError, match="n_workers"):
        run_distributed([_job("j0", _SAMPLE_FEATURES[0])], n_workers=0)


def test_an_empty_job_list_returns_immediately_with_no_results():
    assert run_distributed([], n_workers=4) == []
