from rssparser._rssparser import Entry, Feed, parse, parse_many

__all__ = ["parse", "parse_many", "Feed", "Entry", "FeedParseError"]
__version__ = "1.0.0a0"

FeedParseError = ValueError
