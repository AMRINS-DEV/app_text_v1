"""§17 Phase 9 'distributed agents': see `dispatcher`'s own doc comment for
what this package proves and how.
"""

from .dispatcher import run_distributed
from .jobs import Job, JobResult

__all__ = ["Job", "JobResult", "run_distributed"]
