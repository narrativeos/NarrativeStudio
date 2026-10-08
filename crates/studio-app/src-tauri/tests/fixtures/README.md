# Test Fixtures

`traceview_semantic_sample.json` is a **desensitized subset** of a real TraceView
`semantic_result.json`, committed so that `cargo test --workspace` runs on any
machine and on CI (the tests previously read `~/.TraceView/...`, which only
exists on the developer's laptop).

| Property | Value |
|---|---|
| Blocks | 12 (a contiguous window of the source document) |
| Tokens | 886 |
| Entities | 108 — PERSON 8, LOCATION 61, DATE 19, NUMBER 12, ORGANIZATION 6, FACILITY 1, MATERIAL 1 |
| Characters | 1879 |
| Size | ~167 KB |

## Desensitization

A deterministic 1:1 substitution maps every CJK character to another CJK
character (pool: U+4E00..U+4BFF, selected by a fixed multiplicative hash),
applied identically to `content`, `title`, `tokens[].text`, `entities[].text`
and `entities[].normalized`. ASCII, digits and punctuation are untouched.

Because the mapping is one character in, one character out, every `span` offset,
token count and entity count in the fixture matches the original document
exactly — only the wording is meaningless. Entity categories, POS tags and
confidence values are preserved verbatim, so the fixture exercises the same
code paths as the real data.

Fields the import layer does not read (`dep_tokens`, `dep_edges`, `relations`,
`location_relations`, `term_definitions`) are dropped: they account for most of
the original 15 MB file and are unused.

## Regenerating

```bash
python3 scripts/gen_fixture.py                       # auto-picks the richest window
python3 scripts/gen_fixture.py --start 360 --window 12
python3 scripts/gen_fixture.py --source /path/to/semantic_result.json
```

The generator only runs where a real TraceView project is available; the output
is committed, so CI never needs the source data.
