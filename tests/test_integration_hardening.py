"""Phase 5 integration tests: malformed / oddly-encoded real-world feeds.

These require encoding_rs + byte-level pre-cleanup to parse.
"""

import json
from pathlib import Path

import pytest

from rssparser import parse

_INTEGRATION_DIR = Path(__file__).parent / "integration"

PHASE5_FIXTURES = [
    "malformed_rss_namespaced.xml",
    "utf16_encoded.xml",
]


def pytest_generate_tests(metafunc: pytest.Metafunc):
    if "feed_path" not in metafunc.fixturenames:
        return
    feeds = [_INTEGRATION_DIR / name for name in PHASE5_FIXTURES]
    metafunc.parametrize("feed_path", feeds, ids=[p.name for p in feeds])


def test_hardening_snapshot(feed_path: Path):
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


def test_truncated_feed_raises():
    """osm-pl.xml is truncated mid-CDATA; unrecoverable without libxml-level leniency.
    Documented as known-fail; parser raises FeedParseError rather than emitting garbage.
    """
    data = (_INTEGRATION_DIR / "osm-pl.xml").read_bytes()
    with pytest.raises(ValueError):
        parse(data)


def test_html_raises_not_a_feed():
    with pytest.raises(ValueError):
        parse(b"<!DOCTYPE html><html><body>not a feed</body></html>")


def test_non_feed_json_raises():
    with pytest.raises(ValueError):
        parse(b'{"foo": "bar"}')
