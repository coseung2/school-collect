"""Small, dependency-free repository guard; NOT a full secret or history audit."""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
SECRET_PATTERNS = {
    "private-key": re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"),
    "github-token": re.compile(r"\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{40,})\b"),
    "supabase-secret": re.compile(r"\bsb_secret_[A-Za-z0-9_-]{20,}\b"),
}
SENSITIVE_NAME = re.compile(r"(?:PASSWORD|SECRET(?:_ACCESS)?_KEY|SECRET|SERVICE_ROLE_KEY|MASTERKEY|PRIVATE_KEY|ACCESS_TOKEN)$")


def path_issues(name: str) -> list[str]:
    path = PurePosixPath(name)
    issues = []
    if path.name.startswith('.env') and not path.name.endswith('.example'):
        issues.append('tracked-environment-file')
    if any(part in {'node_modules', '.next', 'target', '__pycache__'} for part in path.parts):
        issues.append('tracked-build-output')
    if name.startswith('supabase/.temp/') or name.endswith(('.tsbuildinfo', '.pyc')):
        issues.append('tracked-generated-state')
    if path.name in {'package-lock.json', 'yarn.lock', 'npm-shrinkwrap.json'}:
        issues.append('use-pnpm-lockfile')
    return issues


def content_issues(name: str, content: bytes) -> list[tuple[int, str]]:
    if b'\x00' in content:
        return []  # Binary data requires a separate manual review.
    try:
        text = content.decode('utf-8')
    except UnicodeDecodeError:
        return []
    findings = []
    for number, line in enumerate(text.splitlines(), 1):
        for rule, pattern in SECRET_PATTERNS.items():
            if pattern.search(line):
                findings.append((number, rule))
        if PurePosixPath(name).name.startswith('.env') and PurePosixPath(name).name.endswith('.example'):
            if '=' in line and not line.lstrip().startswith('#'):
                key, value = line.split('=', 1)
                if SENSITIVE_NAME.search(key.strip()) and value.strip().strip('\"\''):
                    findings.append((number, 'nonempty-secret-example'))
    return findings


def main() -> int:
    result = subprocess.run(['git', 'ls-files', '-z'], cwd=ROOT, capture_output=True, check=True)
    names = [item.decode('utf-8') for item in result.stdout.split(b'\0') if item]
    problems = []
    skipped = 0
    for name in names:
        problems.extend(f'{name}: {rule}' for rule in path_issues(name))
        path = ROOT / name
        if path.is_symlink():
            problems.append(f'{name}: symlink-needs-manual-review')
            continue
        if not path.is_file():
            problems.append(f'{name}: tracked-file-missing')
            continue
        if path.stat().st_size > 2_000_000:
            skipped += 1
            continue
        data = path.read_bytes()
        if b'\x00' in data:
            skipped += 1
        problems.extend(f'{name}:{line}: {rule}' for line, rule in content_issues(name, data))
    for problem in problems:
        print(problem, file=sys.stderr)  # Never print a credential value.
    print(f'Repository guard: {len(names)} files; {len(problems)} findings; {skipped} binary/large files skipped.')
    print('This checks selected patterns only. Full history and personal-data review remain separate gates.')
    return int(bool(problems))


if __name__ == '__main__':
    raise SystemExit(main())
