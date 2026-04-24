"""Phase 3 integration tests: Atom 1.0 fixtures."""

import json
from pathlib import Path

import pytest

from rssparser import parse

_INTEGRATION_DIR = Path(__file__).parent / "integration"

PHASE3_ATOM_FIXTURES = [
    "kagi.xml",
    "osm.xml",
    "stackoverflow.xml",
]


def pytest_generate_tests(metafunc: pytest.Metafunc):
    feeds = [_INTEGRATION_DIR / name for name in PHASE3_ATOM_FIXTURES]
    metafunc.parametrize("feed_path", feeds, ids=[p.name for p in feeds])


def test_atom_snapshot(feed_path: Path):
    data = feed_path.read_bytes()
    parsed = parse(data).to_dict()

    expected_path = feed_path.with_suffix(".json")
    if not expected_path.exists():
        expected_path.write_text(
            json.dumps(parsed, ensure_ascii=False, indent=2, sort_keys=True)
        )
        pytest.skip(f"generated snapshot {expected_path.name}; rerun to validate")

    expected = json.loads(expected_path.read_text())
    assert parsed == expected
