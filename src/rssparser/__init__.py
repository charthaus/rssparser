from rssparser._rssparser import (
    Entry,
    Feed,
    parse,
    parse_many,
    parse_many_to_json,
    parse_to_json,
)

__all__ = [
    "parse",
    "parse_many",
    "parse_to_json",
    "parse_many_to_json",
    "Feed",
    "Entry",
    "FeedParseError",
]
__version__ = "1.0.0a0"

FeedParseError = ValueError
