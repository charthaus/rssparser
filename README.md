# rssparser

A fast Rust-backed RSS / Atom / RDF / JSON Feed parser for Python.

~140x faster than `feedparser` on the 200-feed benchmark corpus (0.22 ms/feed single-threaded, 0.05 ms/feed via batched `parse_many`). Successor to [`fastfeedparser`](https://pypi.org/project/fastfeedparser/); see **Migration** below.

## Install

```bash
pip install rssparser
```

Pre-built wheels ship for Python 3.10–3.14 on manylinux2014 x86_64 / aarch64, musllinux x86_64 / aarch64, macOS arm64 / x86_64, and Windows x86_64.

## Quick start

```python
import rssparser

# Bytes in, Feed out.
feed = rssparser.parse(open("feed.xml", "rb").read())

print(feed.title)
for entry in feed.entries:
    print(entry.title, entry.published, entry.link)

# Full materialization as plain dict (tests, debugging, JSONB inserts).
payload = feed.to_dict()
```

## API

```python
rssparser.parse(data: bytes) -> rssparser.Feed
rssparser.parse_many(blobs: list[bytes]) -> list[rssparser.Feed]
rssparser.parse_to_json(data: bytes) -> bytes
rssparser.parse_many_to_json(blobs: list[bytes]) -> list[bytes]
rssparser.FeedParseError  # alias of ValueError
```

- `parse(data)` — zero-copy view into the Python bytes object; parses while holding the GIL. Blazing fast single-call.
- `parse_many(blobs)` — releases the GIL, parses the batch in parallel via Rayon, returns the list in input order. Use this when you have many feeds.
- `parse_to_json(data)` — parses and emits JSON bytes directly, skipping Python object construction. Ideal for pipelines that pipe straight to JSONB / Kafka / disk.
- `parse_many_to_json(blobs)` — batched JSON emit.

### Feed / Entry shape

```python
feed.title         # Optional[str]
feed.link          # Optional[str]
feed.links         # list[{"href", "rel", "type", "title"}]
feed.description   # Optional[str]
feed.language      # Optional[str]
feed.generator     # Optional[str]
feed.updated       # Optional[str]   # UTC ISO-8601
feed.id            # Optional[str]
feed.icon          # Optional[str]
feed.logo          # Optional[str]
feed.entries       # list[Entry]

entry.title        # Optional[str]
entry.link         # Optional[str]
entry.links        # list[...]
entry.description  # Optional[str]
entry.content      # Optional[str]
entry.published    # Optional[str]   # UTC ISO-8601
entry.updated      # Optional[str]
entry.id           # Optional[str]
# to_dict() also returns: authors, categories, enclosures, media (see full schema in tests/integration/)
```

`feed.to_dict()` and `entry.to_dict()` return nested dicts with the full schema.

### Concurrency

- `parse(bytes)` holds the GIL; Python threads calling `parse` do not run in parallel.
- `parse_many(blobs)` releases the GIL around the whole batch and uses Rayon. Use this for real parallel throughput.
- `Feed` / `Entry` instances are read-only and safe to share across Python threads once constructed.

## Supported feed formats

- RSS 2.0 (including Media RSS, Dublin Core, iTunes, content:encoded, atom:link namespaces)
- Atom 1.0 (including `<content type="xhtml">`, `<rights type="xhtml">`)
- RDF / RSS 1.0 (with Dublin Core)
- JSON Feed 1.0 / 1.1

Format is auto-detected from the first non-whitespace byte (`<` vs `{`) and the XML root tag.

## Input and errors

- `parse` takes `bytes`. No URL fetching — fetch with your HTTP client of choice (`httpx`, `requests`, `aiohttp`) and pass `.content` / `.read()`.
- Input encoding is auto-detected: BOM → UTF-8 / UTF-16; otherwise UTF-8 if valid, else the declared XML encoding via `encoding_rs`.
- Dates are normalized to UTC ISO-8601 (`2024-09-26T00:00:00+00:00`). Unparseable dates are returned verbatim.
- HTML-in-CDATA and XML-escaped HTML are passed through as-is — no sanitization.
- Non-feed input (HTML, arbitrary JSON, truncated XML) raises `rssparser.FeedParseError` (subclass of `ValueError`).

## Benchmark

On an Apple M3 Max (14 cores), parsing the 200-feed / 6600-entry corpus:

|                          | Time    | feeds/sec |   vs feedparser |
|--------------------------|---------|-----------|-----------------|
| `feedparser`             | 6929 ms |        29 |               — |
| `rssparser.parse()`      |   46 ms |      4400 |           152× |
| `rssparser.parse_many()` |   10 ms |     20000 |           690× |

Run `python benchmark.py` for current numbers on your machine.

## Migration from `fastfeedparser`

`rssparser` is the successor project. The API has been redesigned to expose a Rust-owned `Feed` / `Entry` object model. If you're coming from `fastfeedparser` 0.6.x:

| `fastfeedparser`                               | `rssparser`                                 |
|------------------------------------------------|---------------------------------------------|
| `fastfeedparser.parse(url_or_bytes)`           | `rssparser.parse(bytes)` — fetch yourself   |
| `feed.entries[0].title` (dict + attr access)   | `entry.title` (pyclass attr only)           |
| `entry.content[0]["value"]`                    | `entry.content` (string or None)            |
| `entry.description` (HTML)                     | `entry.description` (same)                  |
| `include_content=False` kwarg                  | Not needed — unused fields don't allocate.  |
| `feed.title_detail["value"]`                   | `feed.title` (the `_detail` wrappers are gone) |

Old `fastfeedparser` 0.6.x remains on PyPI and is frozen — no further releases.

## License

MIT.
