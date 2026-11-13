#!/usr/bin/env python3
"""Check the Vietnamese table in `crates/viet-core/src/unicode.rs` against Unicode.

The engine ships a static table instead of a Unicode library (decision D1 in
`docs/SPEC-engine-core.md`), so that table is the only thing standing between the user
and a wrong letter. Python's `unicodedata` is the oracle.

Every row must be one base letter plus an optional quality mark plus an optional tone,
in that order, and every form must already be precomposed (NFC). `đ`/`Đ` is not in the
table on purpose: it has no canonical decomposition, so it is matched by hand in
`base_letter` and `letter_mark`.

Run from the repository root. Exits non-zero on the first mismatch.
"""

import re
import sys
import unicodedata
from pathlib import Path

TABLE_PATH = Path("crates/viet-core/src/unicode.rs")

# The row order is `[plain, acute, grave, hook above, tilde, under dot]`, which is
# `Tone::index` in the engine.
TONE_MARKS = [None, "\u0301", "\u0300", "\u0309", "\u0303", "\u0323"]

QUALITY_MARKS = {
    "None": None,
    "Breve": "\u0306",
    "Circumflex": "\u0302",
    "Horn": "\u031b",
}

ROW_PATTERN = re.compile(
    r"VowelForms\s*\{\s*"
    r"base:\s*'(?P<base>.)',\s*"
    r"quality:\s*(?:None|Some\(LetterMark::(?P<quality>\w+)\)),\s*"
    r"forms:\s*\[(?P<forms>[^\]]*)\],?\s*"
    r"\}",
    re.DOTALL,
)


def read_rows(source: str) -> list[tuple[str, str, list[str]]]:
    rows = []
    for match in ROW_PATTERN.finditer(source):
        forms = re.findall(r"'(.)'", match.group("forms"))
        rows.append((match.group("base"), match.group("quality") or "None", forms))
    return rows


def check_row(base: str, quality: str, forms: list[str]) -> list[str]:
    problems = []
    if len(forms) != len(TONE_MARKS):
        return [f"base {base!r} has {len(forms)} forms, expected {len(TONE_MARKS)}"]

    quality_mark = QUALITY_MARKS[quality]
    for (index, tone_mark), form in zip(enumerate(TONE_MARKS), forms):
        expected = base
        if quality_mark is not None:
            expected += quality_mark
        if tone_mark is not None:
            expected += tone_mark
        # Canonical ordering sorts marks by combining class, so the dot below (220)
        # comes before the circumflex or breve (230) in `ặ`. Normalizing the expected
        # string puts it in the same order instead of hard-coding one.
        expected = unicodedata.normalize("NFD", expected)

        if unicodedata.normalize("NFD", form) != expected:
            decomposition = " ".join(
                f"U+{ord(character):04X}" for character in unicodedata.normalize("NFD", form)
            )
            problems.append(
                f"{form!r} decomposes to {decomposition}, expected U+{ord(base):04X}"
                + (f" + U+{ord(quality_mark):04X}" if quality_mark else "")
                + (f" + U+{ord(tone_mark):04X}" if tone_mark else "")
            )
        if unicodedata.normalize("NFC", form) != form:
            problems.append(f"{form!r} is not precomposed (NFC)")

    if len(set(forms)) != len(forms):
        problems.append(f"base {base!r} lists a form twice")
    return problems


def main() -> int:
    source = TABLE_PATH.read_text(encoding="utf-8")
    rows = read_rows(source)
    if not rows:
        print(f"error: no rows found in {TABLE_PATH}", file=sys.stderr)
        return 1

    problems = []
    seen: dict[str, str] = {}
    for base, quality, forms in rows:
        problems.extend(check_row(base, quality, forms))
        for form in forms:
            if form in seen:
                problems.append(f"{form!r} is listed twice in the table")
            seen[form] = base

    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1

    print(f"{len(rows)} rows, {len(seen)} letters checked against Unicode canonical decomposition")
    return 0


if __name__ == "__main__":
    sys.exit(main())
