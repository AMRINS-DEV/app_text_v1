"""The function each real, independent OS process (`multiprocessing.Process`)
runs: pull jobs from the shared queue until the poison pill, run the
matching agent, push the result. No state survives between jobs or is ever
visible to another worker — each call to a factory in `registry` builds a
brand-new agent from scratch.
"""

from __future__ import annotations

import asyncio
import multiprocessing as mp
import traceback

from .jobs import Job, JobResult
from .registry import AGENT_FACTORIES, INPUT_BUILDERS

POISON_PILL = None


def worker_main(
    worker_id: int, job_queue: mp.Queue[Job | None], result_queue: mp.Queue[JobResult]
) -> None:
    while True:
        job = job_queue.get()
        if job is POISON_PILL:
            return
        result_queue.put(_process_job(worker_id, job))


def _process_job(worker_id: int, job: Job) -> JobResult:
    try:
        factory = AGENT_FACTORIES[job.agent_kind]
        build_input = INPUT_BUILDERS[job.agent_kind]
    except KeyError:
        return JobResult(
            job_id=job.job_id,
            agent_kind=job.agent_kind,
            worker_id=worker_id,
            output=None,
            error=f"unknown agent_kind: {job.agent_kind!r}",
        )

    try:
        agent = factory()
        agent_input = build_input(job.payload)
        output = asyncio.run(agent.run(agent_input))
    except Exception:  # noqa: BLE001 - a worker crash must become a reported failure, never dropped silently
        return JobResult(
            job_id=job.job_id,
            agent_kind=job.agent_kind,
            worker_id=worker_id,
            output=None,
            error=traceback.format_exc(),
        )

    return JobResult(
        job_id=job.job_id,
        agent_kind=job.agent_kind,
        worker_id=worker_id,
        output=output.model_dump(),
        error=None,
    )
