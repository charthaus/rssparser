"""Tests for feed-level image/icon/logo extraction."""

from fastfeedparser import parse


def test_rss_feed_image():
    xml = b"""<?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0">
    <channel>
        <title>Test Feed</title>
        <link>https://example.com</link>
        <image>
            <url>https://example.com/logo.png</url>
            <title>Test Feed</title>
            <link>https://example.com</link>
        </image>
        <item>
            <title>Entry 1</title>
        </item>
    </channel>
    </rss>"""
    result = parse(xml)
    assert result["feed"]["image"] == {
        "url": "https://example.com/logo.png",
        "title": "Test Feed",
        "link": "https://example.com",
    }


def test_rss_feed_image_url_only():
    xml = b"""<?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0">
    <channel>
        <title>Test</title>
        <image>
            <url>https://example.com/icon.png</url>
        </image>
        <item><title>Entry</title></item>
    </channel>
    </rss>"""
    result = parse(xml)
    assert result["feed"]["image"]["url"] == "https://example.com/icon.png"
    assert "title" not in result["feed"]["image"]


def test_rss_feed_no_image():
    xml = b"""<?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0">
    <channel>
        <title>Test</title>
        <item><title>Entry</title></item>
    </channel>
    </rss>"""
    result = parse(xml)
    assert "image" not in result["feed"]


def test_rss_feed_image_empty_url():
    xml = b"""<?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0">
    <channel>
        <title>Test</title>
        <image>
            <url></url>
            <title>Test</title>
        </image>
        <item><title>Entry</title></item>
    </channel>
    </rss>"""
    result = parse(xml)
    assert "image" not in result["feed"]


def test_atom_feed_icon_and_logo():
    xml = b"""<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <title>Test Feed</title>
        <icon>https://example.com/favicon.ico</icon>
        <logo>https://example.com/banner.png</logo>
        <entry>
            <title>Entry 1</title>
            <id>urn:entry:1</id>
        </entry>
    </feed>"""
    result = parse(xml)
    assert result["feed"]["icon"] == "https://example.com/favicon.ico"
    assert result["feed"]["logo"] == "https://example.com/banner.png"


def test_atom_feed_icon_only():
    xml = b"""<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <title>Test</title>
        <icon>https://example.com/favicon.ico</icon>
        <entry><title>E</title><id>urn:1</id></entry>
    </feed>"""
    result = parse(xml)
    assert result["feed"]["icon"] == "https://example.com/favicon.ico"
    assert "logo" not in result["feed"]


def test_atom_feed_no_icon():
    xml = b"""<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <title>Test</title>
        <entry><title>E</title><id>urn:1</id></entry>
    </feed>"""
    result = parse(xml)
    assert "icon" not in result["feed"]
    assert "logo" not in result["feed"]


def test_rdf_feed_image():
    xml = b"""<?xml version="1.0" encoding="UTF-8"?>
    <rdf:RDF
        xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
        xmlns="http://purl.org/rss/1.0/">
        <channel>
            <title>Test RDF Feed</title>
            <link>https://example.com</link>
        </channel>
        <image>
            <url>https://example.com/rdf-logo.png</url>
            <title>RDF Feed</title>
            <link>https://example.com</link>
        </image>
        <item>
            <title>Entry 1</title>
        </item>
    </rdf:RDF>"""
    result = parse(xml)
    assert result["feed"]["image"] == {
        "url": "https://example.com/rdf-logo.png",
        "title": "RDF Feed",
        "link": "https://example.com",
    }
