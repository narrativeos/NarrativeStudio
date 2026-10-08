#!/usr/bin/env python3
"""Generate the desensitized TraceView test fixture from a real TraceView project.

The integration tests (crates/studio-app/src-tauri/tests/integration.rs) used to
read a machine-local TraceView output file, which made `cargo test --workspace`
impossible to run on CI. This script extracts a small window of blocks from a
real `semantic_result.json` and writes a repo-committed fixture that is:

  * structurally faithful - every block keeps the keys the Rust parser reads
    (block_ids, source_block_ids, content, type, title, tokens, entities), so
    token/entity/span counts match the real data;
  * desensitized - a deterministic length-preserving substitution over CJK
    characters, applied identically to content, title, tokens[].text,
    entities[].text and entities[].normalized. Because the mapping is 1:1 per
    character, all span offsets, token counts and entity counts stay valid;
    only the surface wording becomes meaningless.

Usage:
    python3 scripts/gen_fixture.py [--source PATH] [--window N] [--start I]
"""

from __future__ import annotations

import argparse
import collections
import json
import os
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_SOURCE = os.path.expanduser("~/.TraceView/ovgj/semantic/semantic_result.json")
FIXTURE_PATH = os.path.join(
    REPO_ROOT, "crates", "studio-app", "src-tauri", "tests", "fixtures",
    "traceview_semantic_sample.json",
)

# Keys the Rust parser (crates/studio-import/src/semantic.rs) actually reads.
# Everything else (dep_tokens, dep_edges, relations, location_relations,
# term_definitions) is dropped: it is the bulk of the file (15 MB / 693 blocks)
# and unused by the import layer.
BLOCK_KEYS = (
    "block_ids",
    "source_block_ids",
    "content",
    "section_path",
    "type",
    "title",
    "tokens",
    "entities",
    "noun_signals",
)

TOKEN_KEYS = ("text", "pos", "confidence", "span", "source")
ENTITY_KEYS = ("id", "text", "normalized", "category", "source", "confidence",
               "span", "keep", "filter", "filter_reason")

# Deterministic 1:1 CJK substitution pool: the first 2048 codepoints of the CJK
def scramble_char(ch: str) -> str:
    """Map one CJK char to another CJK char. Deterministic and length-preserving."""
    code = ord(ch)
    if 0x4E00 <= code <= 0x9FFF:
        # Knuth multiplicative hash - stable across runs and machines.
        idx = (code * 2654435761) % POOL_SIZE
        return POOL[idx]
    return ch


def scramble(text):
    """Scramble all CJK chars, leaving ASCII/punctuation/digits alone."""
    if not isinstance(text, str):
        return text
    return "".join(scramble_char(c) for c in text)


def pick_window(blocks: list, size: int) -> int:
    """Pick the start index of the most informative contiguous window.

    Preference: windows with title blocks, PERSON/LOCATION entities, a caption
    block and a decent amount of text - so the fixture exercises the structural
    (T1) and entity paths of the analysis layer, not just word counts.
    """
    best_start, best_score = 0, -1
    for start in range(0, max(1, len(blocks) - size + 1)):
        window = blocks[start : start + size]
        titles = sum(1 for b in window if b.get("type") == "title")
        captions = sum(1 for b in window if b.get("type") == "caption")
        persons = locations = chars = 0
        for b in window:
            chars += len(b.get("content", ""))
            for e in b.get("entities", []):
                cat = e.get("category", "")
                if cat == "PERSON":
                    persons += 1
                elif cat == "LOCATION":
                    locations += 1
        score = (
            min(titles, 3) * 60
            + min(persons, 6) * 90
            + min(locations, 6) * 45
            + min(captions, 2) * 25
            + min(chars, 1500)
        )
        if score > best_score:
            best_start, best_score = start, score
    return best_start


def trim_block(block: dict) -> dict:
    """Keep only parser-relevant keys, then scramble every text-bearing field."""
    out = {k: block[k] for k in BLOCK_KEYS if k in block}

    out["content"] = scramble(out.get("content", ""))
    if isinstance(out.get("title"), str):
        out["title"] = scramble(out["title"])
    if isinstance(out.get("section_path"), str):
        out["section_path"] = scramble(out["section_path"])

    tokens = []
    for tok in out.get("tokens", []):
        t = {k: tok[k] for k in TOKEN_KEYS if k in tok}
        t["text"] = scramble(t.get("text", ""))
        tokens.append(t)
    out["tokens"] = tokens

    entities = []
    for ent in out.get("entities", []):
        e = {k: ent[k] for k in ENTITY_KEYS if k in ent}
        e["text"] = scramble(e.get("text", ""))
        if "normalized" in e:
            e["normalized"] = scramble(e["normalized"])
        entities.append(e)
    out["entities"] = entities

    if "noun_signals" in out:
        signals = []
        for sig in out["noun_signals"]:
            s = dict(sig)
            s["text"] = scramble(s.get("text", ""))
            signals.append(s)
        out["noun_signals"] = signals

    return out

# Unified Ideographs block (U+4E00..U+4BFF), all assigned and common enough.
POOL = [chr(0x4E00 + i) for i in range(2048)]
POOL_SIZE = len(POOL)

def main() -> int:
    ap = argparse.ArgumentParser(description="Generate the TraceView test fixture.")
    ap.add_argument("--source", default=DEFAULT_SOURCE, help="real semantic_result.json")
    ap.add_argument("--window", type=int, default=12, help="number of blocks to keep")
    ap.add_argument("--start", type=int, default=None, help="explicit window start index")
    ap.add_argument("--out", default=FIXTURE_PATH, help="fixture output path")
    args = ap.parse_args()

    if not os.path.exists(args.source):
        print(f"source not found: {args.source}", file=sys.stderr)
        return 1

    with open(args.source, encoding="utf-8") as fh:
        data = json.load(fh)

    blocks = data.get("blocks", [])
    start = args.start if args.start is not None else pick_window(blocks, args.window)
    window = blocks[start : start + args.window]

    fixture = {
        "version": data.get("version", "1.0"),
        "timestamp": "2026-01-01T00:00:00Z",
        "source": data.get("source", "hanlp_v2"),
        "blocks": [trim_block(b) for b in window],
    }

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(fixture, fh, ensure_ascii=False, indent=1)
        fh.write("\n")

    types = collections.Counter(b.get("type", "?") for b in fixture["blocks"])
    n_tok = sum(len(b["tokens"]) for b in fixture["blocks"])
    n_ent = sum(len(b["entities"]) for b in fixture["blocks"])
    n_chars = sum(len(b["content"]) for b in fixture["blocks"])
    cats = collections.Counter(
        e["category"] for b in fixture["blocks"] for e in b["entities"]
    )
    print(f"source : {args.source} ({len(blocks)} blocks)")
    print(f"window : blocks [{start}..{start + len(window) - 1}] -> {len(window)} kept")
    print(f"types  : {dict(types)}")
    print(f"tokens : {n_tok}  entities: {n_ent}  chars: {n_chars}")
    print(f"entity categories: {dict(cats)}")
    print(f"written: {args.out} ({os.path.getsize(args.out)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

