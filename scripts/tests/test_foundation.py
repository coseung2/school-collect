from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_branch_policy import allowed
from check_repository import content_issues, path_issues


class RepositoryGuardTests(unittest.TestCase):
    def test_example_and_pnpm_allowed(self):
        for path in ('.env.example', 'infra/.env.example', 'pnpm-lock.yaml'):
            self.assertEqual(path_issues(path), [])

    def test_environment_files_blocked(self):
        for path in ('.env', '.env.local', 'apps/app/.env.production'):
            self.assertIn('tracked-environment-file', path_issues(path))

    def test_generated_state_blocked(self):
        for path in ('tsconfig.tsbuildinfo', 'supabase/.temp/project-ref', 'scripts/x.pyc'):
            self.assertIn('tracked-generated-state', path_issues(path))

    def test_build_output_blocked(self):
        self.assertIn('tracked-build-output', path_issues('server/target/debug/api'))

    def test_other_lockfiles_blocked(self):
        for path in ('package-lock.json', 'yarn.lock', 'apps/app/package-lock.json'):
            self.assertIn('use-pnpm-lockfile', path_issues(path))

    def test_private_key_value_not_returned(self):
        value = ('-----BEGIN ' + 'PRIVATE KEY-----').encode()
        self.assertEqual(content_issues('key.txt', value), [(1, 'private-key')])

    def test_github_token_detected(self):
        value = ('ghp_' + 'A' * 36).encode()
        self.assertEqual(content_issues('config.txt', value), [(1, 'github-token')])

    def test_nonempty_secret_example_blocked(self):
        value = ('DATABASE_' + 'PASSWORD=not-a-real-password').encode()
        self.assertEqual(content_issues('.env.example', value), [(1, 'nonempty-secret-example')])

    def test_storage_secret_example_blocked(self):
        value = ('R2_SECRET_' + 'ACCESS_KEY=not-a-real-secret').encode()
        self.assertEqual(content_issues('infra/.env.local.example', value), [(1, 'nonempty-secret-example')])

    def test_blank_secret_examples_allowed(self):
        self.assertEqual(content_issues('infra/.env.example', b'DATABASE_PASSWORD=\nR2_SECRET_ACCESS_KEY=\n'), [])

    def test_binary_is_not_a_security_claim(self):
        self.assertEqual(content_issues('fixture.bin', b'\x00\xff'), [])


class BranchPolicyTests(unittest.TestCase):
    def test_feature_to_develop(self):
        self.assertTrue(allowed('develop', 'feat/collect-form', True))

    def test_feature_cannot_promote(self):
        self.assertFalse(allowed('main', 'feat/collect-form', True))

    def test_release_and_sync(self):
        self.assertTrue(allowed('main', 'develop', True))
        self.assertTrue(allowed('develop', 'main', True))

    def test_fork_cannot_impersonate_release_or_sync(self):
        self.assertFalse(allowed('main', 'develop', False))
        self.assertFalse(allowed('develop', 'main', False))

    def test_hotfix_release_only(self):
        self.assertTrue(allowed('main', 'hotfix/login', True))
        self.assertFalse(allowed('develop', 'hotfix/login', True))

    def test_invalid_names(self):
        self.assertFalse(allowed('develop', 'feat/', True))
        self.assertFalse(allowed('develop', 'feature-no-prefix', True))
        self.assertFalse(allowed('unknown', 'feat/valid', True))


if __name__ == '__main__':
    unittest.main()
