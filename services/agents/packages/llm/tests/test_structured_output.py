import pytest
from agents_llm.structured_output import StructuredOutputError, parse_structured
from pydantic import BaseModel


class Answer(BaseModel):
    direction: str
    confidence: float


@pytest.mark.anyio
async def test_parses_well_formed_json_on_the_first_try():
    result = await parse_structured('{"direction": "Long", "confidence": 0.8}', Answer)
    assert result == Answer(direction="Long", confidence=0.8)


@pytest.mark.anyio
async def test_repairs_malformed_json_once_and_succeeds():
    calls = []

    async def repair(bad_text: str, error: str) -> str:
        calls.append((bad_text, error))
        return '{"direction": "Short", "confidence": 0.6}'

    result = await parse_structured("not json at all", Answer, repair=repair)

    assert result == Answer(direction="Short", confidence=0.6)
    assert len(calls) == 1


@pytest.mark.anyio
async def test_repairs_a_schema_validation_failure_not_just_bad_json():
    async def repair(bad_text: str, error: str) -> str:
        return '{"direction": "Long", "confidence": 0.9}'

    # Missing required field the first time -> ValidationError, not JSONDecodeError.
    result = await parse_structured('{"direction": "Long"}', Answer, repair=repair)
    assert result.confidence == 0.9


@pytest.mark.anyio
async def test_gives_up_after_max_repairs_and_raises():
    async def repair(bad_text: str, error: str) -> str:
        return "still not json"

    with pytest.raises(StructuredOutputError):
        await parse_structured("not json", Answer, repair=repair, max_repairs=2)


@pytest.mark.anyio
async def test_with_no_repair_callback_raises_immediately_on_bad_input():
    with pytest.raises(StructuredOutputError):
        await parse_structured("not json", Answer)


@pytest.fixture
def anyio_backend():
    return "asyncio"
