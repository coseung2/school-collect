"""Validate PR direction, not approvals or server-side branch protection."""
from __future__ import annotations

import json
import os
import re
from pathlib import Path

FEATURE = re.compile(r'^(feat|fix|refactor|chore|docs|test|build|ci|perf)/[a-z0-9][a-z0-9._/-]*$')
HOTFIX = re.compile(r'^hotfix/[a-z0-9][a-z0-9._/-]*$')


def allowed(base: str, head: str, same_repository: bool) -> bool:
    if base == 'main':
        return same_repository and (head == 'develop' or bool(HOTFIX.fullmatch(head)))
    if base == 'develop':
        return bool(FEATURE.fullmatch(head)) or (same_repository and head == 'main')
    return False


def main() -> int:
    event_path = os.environ.get('GITHUB_EVENT_PATH')
    if not event_path:
        raise SystemExit('GITHUB_EVENT_PATH is required; run unit tests for local validation.')
    event = json.loads(Path(event_path).read_text(encoding='utf-8'))
    pr = event.get('pull_request')
    if not pr:
        print('Not a pull request: branch-direction gate does not apply.')
        return 0
    base, head = pr['base'], pr['head']
    same = (base.get('repo') or {}).get('full_name') == (head.get('repo') or {}).get('full_name')
    if not allowed(base['ref'], head['ref'], same):
        raise SystemExit('Disallowed PR direction. See CONTRIBUTING.md. No metadata is executed as shell code.')
    print('PR direction accepted; protected-branch review is still required.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
