"""Pydantic-validated structured outputs with retry-and-repair (§10.2): "a
malformed response never reaches the core."
"""

from __future__ import annotations

import json
from collections.abc import Awaitable, Callable
from typing import TypeVar

from pydantic import BaseModel, ValidationError

T = TypeVar("T", bound=BaseModel)

RepairFn = Callable[[str, str], Awaitable[str]]


class StructuredOutputError(Exception):
    """Raised when every repair attempt still fails to parse or validate."""


async def parse_structured(
    raw_text: str,
    schema: type[T],
    *,
    repair: RepairFn | None = None,
    max_repairs: int = 1,
) -> T:
    """Parses `raw_text` as JSON and validates it against `schema`. On
    failure, if `repair` is given, calls `repair(bad_text, error_message)`
    to get a corrected text (typically another LLM call asking the model to
    fix its own output against the error) and retries, up to `max_repairs`
    times."""
    attempt_text = raw_text
    last_error: Exception | None = None
    for attempt in range(max_repairs + 1):
        try:
            data = json.loads(attempt_text)
            return schema.model_validate(data)
        except (json.JSONDecodeError, ValidationError) as exc:
            last_error = exc
            if repair is None or attempt >= max_repairs:
                break
            attempt_text = await repair(attempt_text, str(exc))
    raise StructuredOutputError(
        f"failed to parse {schema.__name__} after {max_repairs} repair attempt(s)"
    ) from last_error
