import rssparser


SIMPLE_RSS = b"""<?xml version="1.0"?>
<rss version="2.0">
    <channel>
        <title>Example</title>
    </channel>
</rss>"""


def test_parse_returns_feed():
    feed = rssparser.parse(SIMPLE_RSS)
    assert isinstance(feed, rssparser.Feed)


def test_title_extracted():
    feed = rssparser.parse(SIMPLE_RSS)
    assert feed.title == "Example"


def test_entries_empty_when_no_items():
    feed = rssparser.parse(SIMPLE_RSS)
    assert list(feed.entries) == []


def test_to_dict_has_expected_feed_keys():
    feed = rssparser.parse(SIMPLE_RSS).to_dict()
    assert feed["feed"]["title"] == "Example"
    assert feed["entries"] == []
    assert set(feed.keys()) == {"feed", "entries"}
    expected_feed_keys = {
        "title",
        "link",
        "links",
        "description",
        "language",
        "generator",
        "updated",
        "id",
        "image",
        "icon",
        "logo",
        "authors",
        "categories",
    }
    assert set(feed["feed"].keys()) == expected_feed_keys


def test_parse_non_feed_raises():
    import pytest

    with pytest.raises(ValueError):
        rssparser.parse(b"<html><body>not a feed</body></html>")

    with pytest.raises(ValueError):
        rssparser.parse(b'{"foo": 1}')
