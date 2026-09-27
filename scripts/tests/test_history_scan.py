"""Regression checks for the full-history credential scanner.

All inputs are synthetic; the scan itself runs against a fake `git` so no
repository content is read.
"""
import contextlib
import io
import json
import runpy
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "history_scan.py"


class HistoryScanTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scan = runpy.run_path(str(SCRIPT), run_name="history_scan_under_test")

    def rules_for(self, data: bytes) -> list[str]:
        found = []
        for rule, pattern in self.scan["CONTENT_RULES"].items():
            for match in pattern.finditer(data):
                if not self.scan["is_allowed"](rule, match.group(0)):
                    found.append(rule)
                    break
        return found

    def test_known_placeholders_are_allowed_exactly(self):
        for data in (
            b"POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-school_collect_local_only}",
            b"DATABASE_URL: postgres://school_collect:school_collect_ci@127.0.0.1:5432/x",
            b"postgres://school_collect:${POSTGRES_PASSWORD:-school_collect_local_only}@postgres:5432/x",
            b'"postgres://unused:unused@127.0.0.1:1/absent"',
            b"R2_SECRET_ACCESS_KEY: ${R2_SECRET_ACCESS_KEY:-}",
            b"R2_SECRET_ACCESS_KEY: ${R2_SECRET_ACCESS_KEY}",
        ):
            self.assertEqual(self.rules_for(data), [], data)

    def test_a_value_that_only_contains_a_placeholder_is_reported(self):
        self.assertEqual(
            self.rules_for(b"APP_API_KEY=synthetic_school_collect_ci_suffix_987654321"),
            ["generic-assigned-secret"],
        )
        # A real value hidden as the default of an env reference is still reported.
        self.assertEqual(
            self.rules_for(b"R2_SECRET_ACCESS_KEY: ${R2_SECRET_ACCESS_KEY:-real_value_1234567890}"),
            ["generic-assigned-secret"],
        )
        self.assertEqual(
            self.rules_for(b"postgres://school_collect:school_collect_ci_real_value@db:5432/x"),
            ["postgres-url-with-password"],
        )

    def test_in_head_follows_head_content_not_the_path(self):
        blobs = {
            b"oldblob": b"APP_API_KEY=synthetic_value_123456789",
            b"newblob": b"APP_FEATURE=enabled",
        }

        def fake_git(*args):
            if args[0] == "rev-list":
                return b"old_commit\nhead_commit\n"
            if args[0] == "rev-parse":
                return b"head_commit\n"
            if args[0] == "ls-tree":
                blob = b"oldblob" if args[-1] == "old_commit" else b"newblob"
                return b"100644 blob " + blob + b" 50\tconfig.txt\0"
            if args[0] == "cat-file":
                return blobs[args[-1].encode()]
            raise AssertionError(args)

        main = self.scan["main"]
        main.__globals__["git"] = fake_git
        main.__globals__["sys"].argv = ["history_scan.py", "--json"]
        with contextlib.redirect_stdout(io.StringIO()) as output:
            exit_code = main()
        report = json.loads(output.getvalue())
        self.assertEqual(exit_code, 1)
        self.assertEqual(len(report["findings"]), 1)
        # The secret was replaced in HEAD, so it lives in history only.
        self.assertFalse(report["findings"][0]["in_head"])


if __name__ == "__main__":
    unittest.main()
