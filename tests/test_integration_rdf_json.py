"""Phase 4 integration tests: RDF/RSS 1.0 + JSON Feed fixtures."""

import json
from pathlib import Path

import pytest

from rssparser import parse

_INTEGRATION_DIR = Path(__file__).parent / "integration"

PHASE4_FIXTURES = [
    ("davidbau.xml", ".json"),
    ("json_feed_sample.json", ".expected.json"),
]


def pytest_generate_tests(metafunc: pytest.Metafunc):
    feeds = [(_INTEGRATION_DIR / name, expected_suffix) for name, expected_suffix in PHASE4_FIXTURES]
    metafunc.parametrize(
        ("feed_path", "expected_suffix"),
        feeds,
        ids=[p.name for p, _ in feeds],
    )


def test_snapshot(feed_path: Path, expected_suffix: str):
    data = feed_path.read_bytes()
    parsed = parse(data).to_dict()

    expected_path = feed_path.with_suffix(expected_suffix)
    if not expected_path.exists():
        expected_path.write_text(
            json.dumps(parsed, ensure_ascii=False, indent=2, sort_keys=True)
        )
        pytest.skip(f"generated snapshot {expected_path.name}; rerun to validate")

    expected = json.loads(expected_path.read_text())
    assert parsed == expected
