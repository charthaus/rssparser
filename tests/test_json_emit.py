"""Phase 7: bytes-emit path. JSON output must round-trip through json.loads to
exactly the same dict shape as Feed.to_dict()."""

import json
from pathlib import Path

import pytest

from rssparser import parse, parse_many_to_json, parse_to_json

_INTEGRATION_DIR = Path(__file__).parent / "integration"

ALL_FIXTURES = sorted(
    list(_INTEGRATION_DIR.glob("*.xml")) + [_INTEGRATION_DIR / "json_feed_sample.json"]
)
# Skip the truncated-CDATA fixture (documented in Phase 5 as genuinely unparseable).
SKIP = {"osm-pl.xml"}
FIXTURES = [p for p in ALL_FIXTURES if p.name not in SKIP]


@pytest.mark.parametrize("feed_path", FIXTURES, ids=[p.name for p in FIXTURES])
def test_parse_to_json_matches_to_dict(feed_path: Path):
    data = feed_path.read_bytes()
    from_object = parse(data).to_dict()
    from_bytes = json.loads(parse_to_json(data))
    assert from_bytes == from_object


def test_parse_to_json_returns_bytes():
    data = b"<?xml version='1.0'?><rss version='2.0'><channel><title>X</title></channel></rss>"
    result = parse_to_json(data)
    assert isinstance(result, bytes)
    assert json.loads(result)["feed"]["title"] == "X"


def test_parse_many_to_json_matches_sequential():
    blobs = [p.read_bytes() for p in FIXTURES]
    sequential = [json.loads(parse_to_json(b)) for b in blobs]
    batch = [json.loads(b) for b in parse_many_to_json(blobs)]
    assert sequential == batch


def test_parse_many_to_json_preserves_order():
    blobs = [p.read_bytes() for p in FIXTURES[:3]]
    results = [json.loads(b) for b in parse_many_to_json(blobs)]
    expected_titles = [parse(b).title for b in blobs]
    actual_titles = [r["feed"]["title"] for r in results]
    assert actual_titles == expected_titles
