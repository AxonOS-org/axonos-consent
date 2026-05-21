#!/usr/bin/env python3
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]

required = [
    "README.md",
    "CHANGELOG.md",
    "docs/EVIDENCE_BOUNDARY.md",
    ".github/workflows/ci.yml",
]

for rel in required:
    path = ROOT / rel
    if not path.is_file():
        print(f"ERROR: missing {rel}", file=sys.stderr)
        sys.exit(1)

bad_tokens = [
    "info@axonos.org",
    "FDA approved",
    "CE marked",
    "clinical deployment ready",
    "patient-ready",
    "medical-device approval",
    "regulatory approval",
]

for path in ROOT.rglob("*"):
    if path.is_file() and path.suffix in {".md", ".toml", ".yml", ".yaml", ".rs", ".py"}:
        text = path.read_text(encoding="utf-8", errors="replace")
        for token in bad_tokens:
            if token.lower() in text.lower():
                print(f"ERROR: {path.relative_to(ROOT)} contains risky token: {token}", file=sys.stderr)
                sys.exit(1)

print("axonos-consent repository surface: PASS")
