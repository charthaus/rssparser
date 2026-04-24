# Plan: rssparser — Rust-backed Python feed parser

> Source PRD: design decisions locked in `/grill-me` session (2026-04-24). Replaces `fastfeedparser` 0.6.x.

## Architectural decisions

Durable decisions that apply across all phases:

- **Package**: new PyPI package `rssparser`. Old `fastfeedparser` 0.6.x frozen, no further releases. Same repo, major version 1.0.0 at first release.
- **Python API surface**:
  - `rssparser.parse(data: bytes) -> rssparser.Feed`
  - `rssparser.parse_many(blobs: list[bytes]) -> list[rssparser.Feed]`
  - `rssparser.parse_to_json(data: bytes) -> bytes`
  - `rssparser.parse_many_to_json(blobs: list[bytes]) -> list[bytes]`
  - `rssparser.FeedParseError(ValueError)` — sole exception type
  - No URL fetching, no toggle kwargs, no encoding hints. Bytes in, feed out.
- **Object model**: `Feed` / `Entry` are `#[pyclass]` with lazy Python string allocation on attribute access. Rust owns the parsed data; `PyString::new` happens on read. Both types expose `.to_dict() -> dict` for full materialization (tests, debugging).
- **Concurrency**: per-call parse releases the GIL via `py.allow_threads`. `parse_many` uses `rayon` across cores. Parse state must be `Send` — owned input buffer, no `Py<T>` in hot path.
- **Crate split**: `rssparser-core` (pure Rust, no PyO3, publishable to crates.io) + `rssparser` (PyO3 bindings, publishable to PyPI). Rust ecosystem gets a feed parser as a free side-effect.
- **Dependencies**:
  - `quick-xml` (lenient config) — XML parsing
  - `encoding_rs` — charset detection / decode
  - `simd-json` — JSON Feed parse + bytes-emit writer
  - `speedate` — ISO-8601 / RFC-2822 date parsing
  - `rayon` — `parse_many` thread pool
  - `pyo3` + `maturin` — Python bindings + build
  - `bumpalo` (probable) — per-feed arena allocation for strings
  - No `libxml`, no `reqwest`, no Python callbacks from Rust.
- **Repo layout**:
  - `src/rssparser/` — Python stub package (`__init__.py` re-exporting Rust symbols)
  - `crates/rssparser-core/` — pure Rust crate
  - `crates/rssparser-py/` — PyO3 bindings crate
  - `tests/integration/` — existing `.xml` / `.json` fixtures, regenerated expected outputs
  - `pyproject.toml` — `[tool.maturin]` build backend
  - `src/fastfeedparser/main.py` — **deleted at Phase 1 cut**; still visible in git history
- **Output shape**: spec-first (RSS 2.0 / Atom 1.0 / RDF / JSON Feed specs), not bug-for-bug parity with `fastfeedparser`. Snapshot `.json` files regenerated, human-reviewed for per-feed diffs.
- **Encoding**: auto-detect via `encoding_rs` + BOM + XML declaration. Malformed inputs get a byte-level pre-cleanup pass (port of `_clean_feed_bytes` / `_fix_malformed_xml_bytes` / `_prepare_xml_bytes` from legacy `main.py`).
- **Dates**: pure Rust, no Python callback. ISO-8601 + RFC-2822 via `speedate`, plus hand-rolled fixups for the `_RE_*` patterns in legacy `main.py:66–75` (Feb29, hour-24, non-colon tz, hour-only tz, 7+ digit fractions). `dateparser`-tier natural-language parsing dropped. Unparseable dates return the raw string unchanged.
- **HTML content**: pass-through. Entity decoding done by XML parser only (XML-spec entities). No HTML sanitization, no tag stripping, no whitespace normalization.
- **Error model**: every recoverable parse problem yields `FeedParseError`. Panics bubble as Python `SystemError` (should never happen; considered bugs).
- **Python versions**: 3.10+.
- **License**: MIT (unchanged).

---

## Phase 1: Tracer bullet

**User stories**: infrastructure / build system.

### What to build

Minimum end-to-end path from `bytes` input to a single Python-accessible field, to prove the entire build + FFI + test chain works.

- Scaffold Cargo workspace with `rssparser-core` and `rssparser-py`.
- `pyproject.toml` switches to `[tool.maturin]` build backend.
- `rssparser.parse(data: bytes) -> Feed` implemented for RSS 2.0 only, extracting `channel/title` only. Every other attribute returns `None` or empty list.
- `Feed.to_dict() -> dict` emits `{"feed": {"title": ...}, "entries": []}`.
- One new pytest that parses a trivial RSS 2.0 byte blob and asserts `.title == "Example"`.
- `pip install -e .` produces a working wheel locally.
- Delete `src/fastfeedparser/main.py` and `src/fastfeedparser/__init__.py`; replace `src/` layout with `src/rssparser/__init__.py` re-exporting Rust symbols.

### Acceptance criteria

- [ ] `pip install -e .` builds the extension on macOS + Linux without errors.
- [ ] `python -c "import rssparser; print(rssparser.parse(b'<rss...>').feed.title)"` prints the title.
- [ ] One pytest passes against a hand-crafted minimal RSS 2.0 blob.
- [ ] Repo still builds green in CI with a minimal `cargo build && pytest` job.
- [ ] Legacy `fastfeedparser` Python module is removed from the working tree.

---

## Phase 2: RSS 2.0 core

**User stories**: RSS 2.0 consumers.

### What to build

Complete RSS 2.0 field extraction for well-formed UTF-8 feeds. No encoding detection, no malformed-feed recovery yet — Phase 5 territory.

- Channel-level: title, link, description, language, copyright, managingEditor, webMaster, pubDate, lastBuildDate, image, icon, logo, generator.
- Item-level: title, link, description, author, pubDate, guid, comments, source, categories, enclosures, media (Media RSS namespace), content (content:encoded), dublin core fields.
- Date parsing via `speedate` + ISO/RFC-2822 fixups (port `_RE_*` from legacy).
- Regenerate expected `.json` for each RSS 2.0 fixture in `tests/integration/`. Human diff-review each one.
- Keep existing `pytest_generate_tests` harness in `test_integration.py`; port it to call `parse(bytes).to_dict()` instead of raw `parse()`.
- `#[pyclass] Feed` / `Entry` expose all fields via `__getattr__` with lazy `PyString` allocation.

### Acceptance criteria

- [ ] All RSS 2.0 fixtures in `tests/integration/` have regenerated, human-reviewed `.json` snapshots and pass.
- [ ] `Feed.to_dict()` output is deterministic (same bytes → same dict) and JSON-serializable.
- [ ] Every RSS 2.0 field documented above is populated for at least one fixture.
- [ ] Benchmark script (unchanged harness) parses the RSS 2.0 subset of `benchmark_data/` without raising.
- [ ] Dates emitted in UTC ISO-8601 for all well-formed `pubDate` values in fixtures.

---

## Phase 3: Atom 1.0 core

**User stories**: Atom 1.0 consumers.

### What to build

Atom 1.0 parser with root-tag-based format dispatch. Still well-formed UTF-8 only.

- Feed-level: title, subtitle, link (with rel/type resolution), id, updated, author, contributor, generator, icon, logo, rights.
- Entry-level: title, link (with rel='alternate' / rel='enclosure' precedence), id, updated, published, author, contributor, summary, content (text/html/xhtml), categories, rights, source.
- Format dispatch: peek first non-whitespace byte (`<` vs `{`), then root-tag local name (`rss`, `feed`, `RDF`, N/A for JSON).
- Regenerate Atom fixture snapshots; diff-review.
- Unified `Feed` / `Entry` shape — same pyclass serves all formats. Format-specific fields that don't apply elsewhere return `None`.

### Acceptance criteria

- [ ] All Atom 1.0 fixtures pass with regenerated snapshots.
- [ ] Format dispatch correctly routes RSS 2.0 vs Atom 1.0 based on root tag.
- [ ] `rel='alternate'` link precedence matches Atom spec.
- [ ] XHTML content in Atom `<content type="xhtml">` extracts the inner XHTML without the wrapper `<div>`.
- [ ] Atom subset of `benchmark_data/` parses without raising.

---

## Phase 4: RDF + JSON Feed core

**User stories**: RDF/RSS 1.0 users, JSON Feed users.

### What to build

RDF/RSS 1.0 and JSON Feed 1.0/1.1 parsers. `simd-json` wired.

- RDF: channel + items linked via `rdf:Seq`. Handle `about=` / `rdf:resource=` attribute-based IDs. Dublin Core fields.
- JSON Feed: `simd-json` parse, walk the tree, map to unified `Feed` / `Entry`. Handle both 1.0 and 1.1.
- Format dispatch finalized: `<` → peek root (`rss` / `feed` / `RDF`), `{` → JSON Feed.
- Regenerate RDF + JSON fixture snapshots; diff-review.
- All 28 existing integration test fixtures now have regenerated `.json` expected outputs and pass.

### Acceptance criteria

- [ ] All 28 fixtures in `tests/integration/` pass with regenerated snapshots.
- [ ] JSON Feed 1.0 and 1.1 both parse correctly (distinguishing version-specific fields).
- [ ] RDF `rdf:Seq` ordering preserved in entry list.
- [ ] Non-feed JSON (e.g. arbitrary `{"foo": 1}`) raises `FeedParseError`, not a generic panic.
- [ ] Non-feed XML (e.g. HTML with `<html>` root) raises `FeedParseError`.

---

## Phase 5: Real-world hardening

**User stories**: production users hitting malformed / oddly-encoded feeds at scale.

### What to build

The largest risk phase. Ports the byte-level recovery layer from legacy `main.py` and wires non-UTF-8 encoding handling. Validates the whole parser against the full `benchmark_data/` corpus (≥180 feeds).

- `encoding_rs` charset detection: XML declaration → BOM → meta-refresh → heuristic.
- Port byte-level fixups: double XML decl, double close tags, unquoted attrs, UTF-16 encoding fix, unclosed `<link>`, feed-namespaced malformed RSS, meta-refresh redirect extraction (cap at 3 hops).
- quick-xml lenient config: `allow_unmatched_ends`, `check_end_names=false`, etc.
- Entity-expansion depth cap (guard against billion-laughs).
- Run full `benchmark_data/` corpus through parser in CI. Zero panics allowed; parse errors expected only on genuinely-unparseable bytes.
- If >2–3 feeds in corpus fail `quick-xml` lenient that `lxml`-recover accepts, trigger the libxml-fallback decision — do not add it speculatively.
- UTF-16 and `malformed_rss_namespaced` fixtures now pass.

### Acceptance criteria

- [ ] All 28 integration fixtures pass, including `utf16_encoded.xml` and `malformed_rss_namespaced.xml`.
- [ ] A CI job parses every file in `benchmark_data/` with zero panics.
- [ ] Count of `FeedParseError` raises on `benchmark_data/` is recorded as a regression baseline.
- [ ] A malicious payload test (deeply nested entities, 10 MB of `&lol;`) terminates quickly with `FeedParseError`, not OOM.
- [ ] Encoding detection correctly handles at least: UTF-8, UTF-8 with BOM, UTF-16 LE/BE with BOM, ISO-8859-1 declared in XML decl, Windows-1252 sniffed.

---

## Phase 6: Parallelism

**User stories**: Kagi-scale ingest — thousands of feeds per batch.

### What to build

Release the GIL during parse bodies and add a batched rayon-parallel API.

- Wrap core parse in `py.allow_threads(|| ...)`. Re-acquire GIL only for `#[pyclass]` construction.
- `parse_many(blobs: list[bytes]) -> list[Feed]` — release GIL once, rayon over blobs, reacquire to build results.
- Benchmark: measure N-core scaling on a batch of 200 feeds from `benchmark_data/`. Expect near-linear up to physical core count.
- Document thread-safety model in docstring: `Feed` / `Entry` are read-only and safe to share across Python threads; parse calls are safe to issue concurrently from Python threads.

### Acceptance criteria

- [ ] `parse_many` benchmark shows ≥0.7 × N-core speedup vs sequential `parse` at N cores (N up to physical core count).
- [ ] Python `ThreadPoolExecutor.map(rssparser.parse, feeds)` demonstrably runs in parallel (CPU utilization > 100 % of one core).
- [ ] No data races under `RUSTFLAGS="-Z sanitizer=thread"` on the `parse_many` path for a nightly-only test job.
- [ ] Total parse time for `benchmark_data/` full corpus via `parse_many` is at least 4× faster than sequential `parse` on an 8-core machine.

---

## Phase 7: Bytes emit

**User stories**: pipelines that write directly to JSONB / Kafka / disk without Python object construction.

### What to build

Second emit path that skips Python object allocation entirely, serializing directly to JSON bytes.

- `parse_to_json(data: bytes) -> bytes` — parse, then serialize via simd-json writer inside the GIL-released block. Return one `PyBytes`.
- `parse_many_to_json(blobs: list[bytes]) -> list[bytes]` — batched variant.
- Contract: `json.loads(parse_to_json(x))` == `parse(x).to_dict()` exactly. Add a property test asserting this on every fixture.
- Benchmark: `parse_to_json` latency must be lower than `parse(x).to_dict()` on every fixture (no Python dict allocation).

### Acceptance criteria

- [ ] `json.loads(parse_to_json(x)) == parse(x).to_dict()` holds for all 28 fixtures.
- [ ] `parse_to_json` median latency is lower than `parse().to_dict()` median latency on every fixture.
- [ ] `parse_many_to_json` scales across cores (same as Phase 6).
- [ ] Output bytes are valid UTF-8 JSON (no invalid escapes, no trailing NULs).

---

## Phase 8: Release

**User stories**: end users installing from PyPI.

### What to build

CI wheel matrix, documentation for migration, PyPI publish.

- `cibuildwheel` matrix: manylinux2014 x86_64 + aarch64, musllinux x86_64 + aarch64, macOS arm64 + x86_64, Windows x86_64. Per-Python-version wheels (no abi3) for 3.10, 3.11, 3.12, 3.13.
- GitHub Actions workflow: on tag `v*`, build all wheels + sdist, upload to PyPI as `rssparser`.
- Publish `rssparser-core` crate to crates.io at the same version.
- `README.md` rewritten: new API, migration-from-fastfeedparser section, benchmark numbers (vs `fastfeedparser` 0.6.x and `feedparser`).
- Release note on `fastfeedparser` 0.6.x PyPI page pointing at `rssparser`. Freeze `fastfeedparser` branch; no new releases.
- `CHANGELOG.md` with full breaking-change list.

### Acceptance criteria

- [ ] Wheels for every platform/Python combo listed above build green on GitHub Actions.
- [ ] A fresh user can `pip install rssparser` on macOS arm64, Linux x86_64, and Windows x86_64 and run the README quick-start without a compiler.
- [ ] `rssparser-core` on crates.io and `rssparser` on PyPI carry matching versions.
- [ ] Benchmark results published in README showing speedup vs `fastfeedparser` 0.6.x on the full `benchmark_data/` corpus.
- [ ] Old `fastfeedparser` PyPI page notes the successor package.
