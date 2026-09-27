"""C-04 full-history scan: locate credential-shaped content in every commit.

Walks every blob reachable from any ref (all branches, tags, stashes) and
reports where a credential-shaped pattern or a sensitive path appears. The
report names the rule, path, and first/last commit that contained it; it never
prints the matched value. The goal is to size a history-rewrite decision and
list which external credentials must be revoked, not to prove the history is
clean: pattern scanning cannot find every secret.

Usage: python scripts/history_scan.py [--json]
Exit code 0 when nothing matched, 1 when findings exist.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
MAX_BLOB_BYTES = 2_000_000

CONTENT_RULES = {
    "private-key": re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH |DSA |ENCRYPTED )?PRIVATE KEY-----"),
    "github-token": re.compile(rb"\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{40,})\b"),
    "supabase-secret": re.compile(rb"\bsb_secret_[A-Za-z0-9_-]{20,}\b"),
    "jwt": re.compile(rb"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b"),
    "aws-access-key": re.compile(rb"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b"),
    "postgres-url-with-password": re.compile(rb"postgres(?:ql)?://[^:@/\s'\"]+:[^@/\s'\"]{6,}@[^\s'\"]+"),
    "infisical-token": re.compile(rb"\bst\.[A-Za-z0-9-]{20,}\.[A-Za-z0-9]{20,}\.[A-Za-z0-9]{20,}\b"),
    "generic-assigned-secret": re.compile(
        rb"(?im)^[ \t]*[A-Z0-9_]*(?:PASSWORD|SECRET|SERVICE_ROLE_KEY|PRIVATE_KEY|ACCESS_TOKEN|API_KEY)[A-Z0-9_]*"
        rb"[ \t]*[=:][ \t]*['\"]?[^\s'\"#]{12,}"
    ),
}

# Secret values that are known local/CI placeholders committed on purpose
# (developer Docker stack, CI service containers, unit tests). A match is
# allowed only when its whole secret value equals one of these; a real value
# that merely contains a placeholder is still reported.
ALLOWED_SECRET_VALUES = frozenset({
    b"school_collect_ci",
    b"school_collect_local_only",
    b"school_collect",
    b"unused",
})

_ENV_DEFAULT = re.compile(rb"^\$\{[A-Z0-9_]+:-(?P<default>[^}]*)\}$")
_URL_PASSWORD = re.compile(rb"://[^:@/\s'\"]+:(?P<password>[^@/\s'\"]+)@")
_ASSIGNED_VALUE = re.compile(rb"[=:][ \t]*['\"]?(?P<value>[^\s'\"#]+)")


def secret_value(rule: str, matched: bytes) -> bytes | None:
    """The part of a match that would be the secret, with `${VAR:-x}` reduced to x."""
    if rule == "postgres-url-with-password":
        found = _URL_PASSWORD.search(matched)
    elif rule == "generic-assigned-secret":
        found = _ASSIGNED_VALUE.search(matched)
    else:
        return None  # keys and tokens have no placeholder form
    if found is None:
        return None
    value = found.group(found.lastgroup)
    default = _ENV_DEFAULT.match(value)
    return default.group("default") if default else value


def is_allowed(rule: str, matched: bytes) -> bool:
    value = secret_value(rule, matched)
    # `${VAR}` or `${VAR:-}` only names where a value comes from at run time; an
    # empty default carries no secret.
    if value is not None and (value == b"" or re.fullmatch(rb"\$\{[A-Z0-9_]+\}", value)):
        return True
    return value in ALLOWED_SECRET_VALUES


def sensitive_path(name: str) -> str | None:
    path = PurePosixPath(name)
    lowered = path.name.lower()
    if lowered.startswith(".env") and not lowered.endswith(".example"):
        return "environment-file"
    if name.startswith("supabase/.temp/") or "/.temp/" in name:
        return "cli-state"
    if lowered.endswith((".pem", ".p12", ".pfx", ".key")):
        return "key-material-file"
    if name.startswith("_agent_") or "/_agent_" in name:
        return "work-document-tree"
    return None


@dataclass
class Finding:
    rule: str
    path: str
    commits: list[str] = field(default_factory=list)


def git(*args: str) -> bytes:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, check=True).stdout


def main() -> int:
    as_json = "--json" in sys.argv[1:]
    commits = git("rev-list", "--all", "--reverse").decode().split()
    findings: dict[tuple[str, str], Finding] = {}
    blob_rules: dict[str, list[str]] = {}

    for commit in commits:
        listing = git("ls-tree", "-r", "-z", "--long", commit).split(b"\0")
        for entry in listing:
            if not entry:
                continue
            meta, _, raw_path = entry.partition(b"\t")
            parts = meta.split()
            if len(parts) < 4 or parts[1] != b"blob":
                continue
            blob, size = parts[2].decode(), parts[3].decode()
            name = raw_path.decode("utf-8", "replace")
            rules: list[str] = []
            path_rule = sensitive_path(name)
            if path_rule:
                rules.append(path_rule)
            if size.isdigit() and int(size) <= MAX_BLOB_BYTES:
                if blob not in blob_rules:
                    data = git("cat-file", "blob", blob)
                    matched = []
                    if b"\x00" not in data:
                        for rule, pattern in CONTENT_RULES.items():
                            for match in pattern.finditer(data):
                                if not is_allowed(rule, match.group(0)):
                                    matched.append(rule)
                                    break
                    blob_rules[blob] = matched
                rules.extend(blob_rules[blob])
            for rule in rules:
                key = (rule, name)
                finding = findings.setdefault(key, Finding(rule, name))
                if not finding.commits or finding.commits[-1] != commit:
                    finding.commits.append(commit)

    # "In HEAD" means HEAD's own content still triggers the rule, not merely that
    # a file with that name exists there.
    head = git("rev-parse", "HEAD").decode().strip()
    rows = []
    for finding in sorted(findings.values(), key=lambda f: (f.rule, f.path)):
        rows.append({
            "rule": finding.rule,
            "path": finding.path,
            "commits": len(finding.commits),
            "first": finding.commits[0][:10],
            "last": finding.commits[-1][:10],
            "in_head": head in finding.commits,
        })

    if as_json:
        print(json.dumps({"commits_scanned": len(commits), "findings": rows}, ensure_ascii=False, indent=2))
    else:
        print(f"History scan: {len(commits)} commits, {len(rows)} findings (values are never printed).")
        for row in rows:
            where = "HEAD" if row["in_head"] else "history only"
            print(f"- {row['rule']}: {row['path']} [{where}; {row['commits']} commits, {row['first']}..{row['last']}]")
        print("Pattern scanning cannot prove the absence of secrets; revoke any credential that was ever committed.")
    return int(bool(rows))


if __name__ == "__main__":
    raise SystemExit(main())
