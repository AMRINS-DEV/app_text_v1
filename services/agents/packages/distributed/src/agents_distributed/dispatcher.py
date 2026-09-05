"""Spawns `n_workers` real OS processes and distributes `jobs` across them
through a shared queue, collecting every result — the actual, executable
proof of §16's "agents are stateless and horizontally scalable," not an
assertion.

Uses the `spawn` start method explicitly, not `fork`: `spawn` is the
portable, safe default on macOS/Windows and is increasingly recommended on
Linux too (fork-after-threads has its own well-known hazards), and it also
forces every job's payload through `Job`'s plain-dict shape rather than
letting a worker accidentally inherit copy-on-write memory a fork would
share. If the dispatch is correct under `spawn`, it is genuinely free of
shared state between workers — not merely "a bug fork happens to hide."
"""

from __future__ import annotations

import multiprocessing as mp

from .jobs import Job, JobResult
from .worker import worker_main


def run_distributed(jobs: list[Job], n_workers: int) -> list[JobResult]:
    if n_workers < 1:
        msg = "n_workers must be >= 1"
        raise ValueError(msg)
    if not jobs:
        return []

    ctx = mp.get_context("spawn")
    job_queue: mp.Queue[Job | None] = ctx.Queue()
    result_queue: mp.Queue[JobResult] = ctx.Queue()

    for job in jobs:
        job_queue.put(job)
    for _ in range(n_workers):
        job_queue.put(None)  # one poison pill per worker, so every worker terminates on its own

    workers = [
        ctx.Process(target=worker_main, args=(i, job_queue, result_queue), daemon=True)
        for i in range(n_workers)
    ]
    for w in workers:
        w.start()

    try:
        results = [result_queue.get(timeout=60) for _ in jobs]
    finally:
        for w in workers:
            w.join(timeout=10)

    return results
