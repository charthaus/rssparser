"""Phase 2 integration tests: well-formed UTF-8 RSS 2.0 fixtures.

Snapshots are spec-first: deleting the .json forces regeneration.
Phase 5 folds in the malformed / UTF-16 RSS 2.0 fixtures.
"""

import json
from pathlib import Path

import pytest

from rssparser import parse

_INTEGRATION_DIR = Path(__file__).parent / "integration"

PHASE2_RSS_FIXTURES = [
    "foolcontrol.xml",
    "fwrarejazzvinylcollector.xml",
    "gamersnexus.xml",
    "postgis.xml",
    "pypy.xml",
    "vintagehomeplans.xml",
    # Deferred to Phase 5 (needs lxml-recover-level forgiveness):
    # "osm-pl.xml",  # truncated mid-CDATA
]


def pytest_generate_tests(metafunc: pytest.Metafunc):
    feeds = [_INTEGRATION_DIR / name for name in PHASE2_RSS_FIXTURES]
    metafunc.parametrize("feed_path", feeds, ids=[p.name for p in feeds])


def test_rss_snapshot(feed_path: Path):
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
