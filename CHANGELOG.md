# Changelog

## 1.0.0 — unreleased

First release of the Rust-backed parser. Successor to `fastfeedparser` 0.6.x.

### Breaking changes vs `fastfeedparser` 0.6.x

- Package name is now `rssparser` (old `fastfeedparser` remains on PyPI, frozen at 0.6.x).
- `parse()` accepts `bytes` only — URL fetching removed. Callers should pass `httpx.get(url).content` or equivalent.
- Return value is an opaque `rssparser.Feed` pyclass, not a `FastFeedParserDict`. Access fields via attribute (`feed.title`, not `feed["title"]`).
- `feed.entries[i]` is an `rssparser.Entry` pyclass. Same attribute-access pattern.
- Output schema is spec-first:
  - Dropped `_detail` wrapper objects (`title_detail`, `subtitle_detail`, ...).
  - `entry.content` is now `Optional[str]` (previously `list[{"value", "type", ...}]`).
  - Added `authors: list[Person]`, `categories: list[Category]`, `enclosures: list[Enclosure]`, `media: list[MediaContent]`, `links: list[Link]` with structured sub-fields on both `Feed` and `Entry`.
- `include_content=`, `include_tags=`, `include_media=`, `include_enclosures=` kwargs removed. Unused fields don't allocate anyway with the lazy-access design.
- Dates always emitted as UTC ISO-8601 strings (`2024-09-26T00:00:00+00:00`) or the raw string if unparseable. The `dateparser` natural-language tier is dropped.
- Errors raise `rssparser.FeedParseError` (a subclass of `ValueError`) — there is a single exception type, not the loose mixture the old parser used.

### New capabilities

- `rssparser.parse_many(blobs)` — parallel batch parse via Rayon.
- `rssparser.parse_to_json(data)` — direct bytes emit, skips Python object construction.
- `rssparser.parse_many_to_json(blobs)` — batched bytes emit.
- Format auto-dispatch across RSS 2.0 / Atom 1.0 / RDF / JSON Feed based on first byte + root tag.
- Encoding auto-detection via `encoding_rs` (BOM, XML declaration, UTF-8 fallback). Correctly handles feeds that mis-declare their encoding.
- Pre-cleanup of malformed XML ported from `fastfeedparser._fix_malformed_xml_bytes`: double `<?xml?xml>` declaration, double `??>`, unquoted attributes, `utf-16`-declared-but-actually-`utf-8`.

### Performance

~5× faster than `fastfeedparser` 0.6.x and ~140× faster than `feedparser` on the included 200-feed corpus. See `README.md` for numbers.

### Known limitations

- Feeds that are truncated mid-CDATA (e.g. `osm-pl.xml` in the fixtures) are not recoverable without libxml-level leniency and will raise `FeedParseError`.
- Python thread parallelism on single `parse()` calls is serialized by the GIL. Use `parse_many` for real concurrency.
