import base64
import hashlib
import json
import re
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
EXTENSION = ROOT / "apps" / "extension"
MANIFEST = EXTENSION / "manifest.json"
BRIDGE = ROOT / "apps" / "app" / "src-tauri" / "src" / "bridge.rs"


def extension_id(public_key: str) -> str:
    """Chrome derives an unpacked extension id from the pinned public key."""
    digest = hashlib.sha256(base64.b64decode(public_key)).hexdigest()[:32]
    return "".join(chr(ord("a") + int(nibble, 16)) for nibble in digest)


class ExtensionContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        cls.bridge = BRIDGE.read_text(encoding="utf-8")

    def test_manifest_is_a_keyed_mv3_extension(self):
        self.assertEqual(self.manifest["manifest_version"], 3)
        self.assertIn("key", self.manifest)
        self.assertEqual(self.manifest["version"], "0.2.0")

    def test_pinned_extension_id_matches_the_bridge_allowlist(self):
        match = re.search(
            r'ALLOWED_EXTENSION_ID: &str = "([a-p]{32})"', self.bridge
        )
        self.assertIsNotNone(match, "bridge.rs must pin the extension id")
        self.assertEqual(match.group(1), extension_id(self.manifest["key"]))

    def test_extension_cannot_reach_anything_but_the_local_bridge(self):
        self.assertEqual(self.manifest["host_permissions"], ["http://127.0.0.1:43110/*"])
        self.assertEqual(
            sorted(self.manifest["permissions"]), ["activeTab", "scripting", "storage"]
        )
        serialized = json.dumps(self.manifest)
        self.assertNotIn("<all_urls>", serialized)
        self.assertNotIn("http://*/*", serialized)
        self.assertNotIn("https://*/*", serialized)
        self.assertNotIn("cookies", serialized)

    def test_extension_scripts_parse_as_modules(self):
        """팝업과 페이지 명령이 ES 모듈로 해석되는지 확인합니다."""
        node = shutil.which("node")
        if node is None:
            self.skipTest("node is not available")

        for script in sorted(EXTENSION.glob("*.js")):
            with tempfile.TemporaryDirectory() as directory:
                copy = Path(directory) / f"{script.stem}.mjs"
                shutil.copyfile(script, copy)
                completed = subprocess.run(
                    [node, "--check", str(copy)],
                    capture_output=True,
                    text=True,
                )
            self.assertEqual(
                completed.returncode,
                0,
                f"{script.name}: {completed.stderr.strip()}",
            )


if __name__ == "__main__":
    unittest.main()
