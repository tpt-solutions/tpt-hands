# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 TPT Solutions

"""Breaking-change check for proto/tpt_hands.proto.

Compares a baseline snapshot with the current file and fails on any change that
would break an existing peer. The rules are in docs/PROTO_COMPATIBILITY.md.

Usage: python proto_compat.py BASELINE CURRENT

Stdlib only. It reads the subset of proto3 this repo uses: messages, enums,
oneofs, and scalar, message and enum fields. It refuses other constructs (map
fields, reserved ranges, extend, service), so the check cannot pass by ignoring
something it does not understand.
"""

import re
import sys

BLOCK_OPEN = re.compile(r"^\s*(message|enum|oneof)\s+(\w+)\s*\{(\s*\})?\s*$")
FIELD = re.compile(
    r"^\s*(repeated\s+|optional\s+)?([\w.]+)\s+(\w+)\s*=\s*(\d+)\s*(\[[^\]]*\])?\s*;\s*$"
)
ENUM_VALUE = re.compile(r"^\s*(\w+)\s*=\s*(-?\d+)\s*;\s*$")
UNSUPPORTED = re.compile(r"\b(map\s*<|reserved|extend|service|rpc)\b")
HEADER_KEYWORDS = ("syntax", "package", "import", "option")


class ParseError(Exception):
    pass


def parse(text):
    """Returns {"message X": {number: (name, type)}, "enum E": {number: name}}."""
    out = {}
    stack = []  # (kind, key) for each open block. A oneof is transparent.
    for lineno, raw in enumerate(text.splitlines(), 1):
        line = raw.split("//", 1)[0].rstrip()
        if not line.strip():
            continue
        if UNSUPPORTED.search(line):
            raise ParseError(f"line {lineno}: unsupported construct: {line.strip()}")
        if line.strip() == "}":
            if not stack:
                raise ParseError(f"line {lineno}: unbalanced closing brace")
            stack.pop()
            continue
        m = BLOCK_OPEN.match(line)
        if m:
            kind, name, inline_empty = m.group(1), m.group(2), m.group(3)
            if kind == "oneof":
                if not stack or stack[-1][0] != "message":
                    raise ParseError(f"line {lineno}: oneof outside a message")
                if not inline_empty:
                    stack.append(("oneof", None))
                continue
            key = f"{kind} {name}"
            if key in out:
                raise ParseError(f"line {lineno}: duplicate {key}")
            out[key] = {}
            if not inline_empty:
                stack.append((kind, key))
            continue
        owner = next((k for kind, k in reversed(stack) if kind in ("message", "enum")), None)
        if owner is None:
            if line.strip().startswith(HEADER_KEYWORDS):
                continue
            raise ParseError(f"line {lineno}: unexpected line outside a block: {line.strip()}")
        if owner.startswith("enum "):
            e = ENUM_VALUE.match(line)
            if not e:
                raise ParseError(f"line {lineno}: cannot read enum value: {line.strip()}")
            number = int(e.group(2))
            if number in out[owner]:
                raise ParseError(f"line {lineno}: enum value {number} used twice in {owner}")
            out[owner][number] = e.group(1)
            continue
        f = FIELD.match(line)
        if not f:
            raise ParseError(f"line {lineno}: cannot read field: {line.strip()}")
        number = int(f.group(4))
        if number in out[owner]:
            raise ParseError(f"line {lineno}: field number {number} used twice in {owner}")
        label = (f.group(1) or "").strip()
        out[owner][number] = (f.group(3), f"{label} {f.group(2)}".strip())
    if stack:
        raise ParseError("unclosed block at end of file")
    return out


def compare(baseline, current):
    """Returns the breaking changes. Additions are not breaking."""
    problems = []
    for owner, fields in baseline.items():
        if owner not in current:
            problems.append(f"removed {owner}")
            continue
        now = current[owner]
        for number, value in fields.items():
            if number not in now:
                label = value if owner.startswith("enum ") else value[0]
                problems.append(f"{owner}: {number} ({label}) removed")
                continue
            if owner.startswith("enum "):
                if now[number] != value:
                    problems.append(f"{owner}: value {number} renamed {value} -> {now[number]}")
                continue
            old_name, old_type = value
            new_name, new_type = now[number]
            if old_name != new_name:
                problems.append(f"{owner}: field {number} renamed {old_name} -> {new_name}")
            if old_type != new_type:
                problems.append(
                    f"{owner}: field {number} ({old_name}) type {old_type} -> {new_type}"
                )
    return problems


def main(argv):
    if len(argv) != 3:
        print(__doc__.strip())
        return 2
    with open(argv[1], encoding="utf-8") as f:
        baseline = parse(f.read())
    with open(argv[2], encoding="utf-8") as f:
        current = parse(f.read())
    problems = compare(baseline, current)
    if problems:
        print("breaking changes against the baseline:")
        for p in problems:
            print("  -", p)
        return 1
    added = sorted(set(current) - set(baseline))
    print("compatible" + (f"; new: {', '.join(added)}" if added else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
