# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 TPT Solutions

import os
import unittest

from proto_compat import ParseError, compare, parse

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))

BASE = """
message A {
  uint32 id = 1;
  string name = 2;
  oneof action {
    B b = 3;
    uint32 c = 4;
  }
}
message B {}
enum E {
  E_ZERO = 0;
  E_ONE = 1;
}
"""


class ParseTests(unittest.TestCase):
    def test_reads_fields_oneofs_and_enums(self):
        p = parse(BASE)
        self.assertEqual(p["message A"][1], ("id", "uint32"))
        self.assertEqual(p["message A"][3], ("b", "B"))
        self.assertEqual(p["message A"][4], ("c", "uint32"))
        self.assertEqual(p["enum E"], {0: "E_ZERO", 1: "E_ONE"})
        self.assertIn("message B", p)

    def test_refuses_constructs_it_cannot_check(self):
        with self.assertRaises(ParseError):
            parse("message A {\n  map<string, uint32> m = 1;\n}\n")
        with self.assertRaises(ParseError):
            parse("message A {\n  reserved 5;\n}\n")

    def test_refuses_a_repeated_field_number(self):
        with self.assertRaises(ParseError):
            parse("message A {\n  uint32 a = 1;\n  uint32 b = 1;\n}\n")


class CompareTests(unittest.TestCase):
    def check(self, current_text):
        return compare(parse(BASE), parse(current_text))

    def test_identical_is_compatible(self):
        self.assertEqual(self.check(BASE), [])

    def test_additions_are_compatible(self):
        grown = BASE.replace("uint32 c = 4;", "uint32 c = 4;\n    uint32 d = 5;")
        grown += "message C {}\n"
        self.assertEqual(self.check(grown), [])

    def test_removed_field_is_breaking(self):
        cut = BASE.replace("  string name = 2;\n", "")
        self.assertTrue(any("2 (name) removed" in p for p in self.check(cut)))

    def test_renamed_field_is_breaking(self):
        renamed = BASE.replace("string name = 2;", "string label = 2;")
        self.assertTrue(any("renamed name -> label" in p for p in self.check(renamed)))

    def test_retyped_field_is_breaking(self):
        retyped = BASE.replace("string name = 2;", "uint32 name = 2;")
        self.assertTrue(any("type string -> uint32" in p for p in self.check(retyped)))

    def test_removed_message_is_breaking(self):
        cut = BASE.split("message B")[0]
        self.assertTrue(any("removed message B" in p for p in self.check(cut)))

    def test_enum_renumbering_is_breaking(self):
        moved = BASE.replace("E_ONE = 1;", "E_ONE = 2;")
        self.assertTrue(self.check(moved))

    def test_repo_baseline_matches_current_proto(self):
        # The committed baseline is the snapshot the repo keeps compatible. If
        # this fails, the proto changed. Update the baseline on purpose, in the
        # same commit, once the change is agreed to be allowed.
        with open(os.path.join(REPO, "proto", "baseline", "tpt_hands.proto"), encoding="utf-8") as f:
            base = parse(f.read())
        with open(os.path.join(REPO, "proto", "tpt_hands.proto"), encoding="utf-8") as f:
            cur = parse(f.read())
        self.assertEqual(compare(base, cur), [])


if __name__ == "__main__":
    unittest.main()
