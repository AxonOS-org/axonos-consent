#!/usr/bin/env python3
"""Repository hygiene checks for axonos-consent."""

from __future__ import annotations

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]

JOBS = [
    "repository-contract",
    "line-counts",
    "cargo-manifest",
    "readme-surface",
    "lib-surface",
    "ci-workflow",
    "ci-job-count",
    "contact-policy",
    "clean-public-surface",
    "overclaim-guard",
    "standard-mapping",
    "security-policy",
    "changelog",
    "license-files",
    "source-tree",
    "path-hygiene",
    "rustdoc-hygiene",
]

PRIMARY = [
    "README.md",
    "Cargo.toml",
    "src/lib.rs",
    "CHANGELOG.md",
    "SECURITY.md",
    ".github/workflows/ci.yml",
]

def fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    sys.exit(1)

def read(rel: str) -> str:
    p = ROOT / rel
    if not p.is_file():
        fail(f"missing required file: {rel}")
    return p.read_text(encoding="utf-8", errors="replace")

def require_file(rel: str) -> None:
    if not (ROOT / rel).is_file():
        fail(f"missing required file: {rel}")

def require_dir(rel: str) -> None:
    if not (ROOT / rel).is_dir():
        fail(f"missing required directory: {rel}")

def line_count(rel: str) -> int:
    return len(read(rel).splitlines())

def check_repository_contract() -> None:
    for rel in [
        "README.md",
        "Cargo.toml",
        "src/lib.rs",
        "CHANGELOG.md",
        "SECURITY.md",
        "LICENSE-APACHE",
        "LICENSE-MIT",
        ".github/workflows/ci.yml",
        "tools/verify_consent_repository.py",
    ]:
        require_file(rel)
    print("repository contract: PASS")

def check_line_counts() -> None:
    thresholds = {
        "README.md": 100,
        "Cargo.toml": 40,
        "src/lib.rs": 65,
        "CHANGELOG.md": 15,
        "SECURITY.md": 20,
        ".github/workflows/ci.yml": 200,
        "tools/verify_consent_repository.py": 80,
    }
    for rel, minimum in thresholds.items():
        lines = line_count(rel)
        if lines < minimum:
            fail(f"{rel} appears collapsed: {lines} lines < {minimum}")
    print("line counts: PASS")

def check_cargo_manifest() -> None:
    text = read("Cargo.toml")
    for token in [
        "[package]",
        'name = "axonos-consent"',
        'version = "0.3.0"',
        'edition = "2021"',
        'license = "Apache-2.0 OR MIT"',
        "[features]",
        "[profile.release]",
    ]:
        if token not in text:
            fail(f"Cargo.toml missing token: {token}")
    print("cargo manifest: PASS")

def check_readme_surface() -> None:
    text = read("README.md")
    for token in [
        "AxonOS-native deterministic neural consent runtime",
        "Consent is not a UI checkbox",
        "External protocol compatibility claim",
        "AOS-0004",
        "AOS-0005",
        "AOS-0009",
        "AOS-0012",
        "No L3 hardware timing claim",
        "connect@axonos.org",
        "security@axonos.org",
    ]:
        if token not in text:
            fail(f"README missing token: {token}")
    print("readme surface: PASS")

def check_lib_surface() -> None:
    text = read("src/lib.rs")
    for token in [
        "#![forbid(unsafe_code)]",
        "AxonOS-native deterministic neural consent runtime",
        "SPEC_VERSION",
        "CRATE_VERSION",
        "pub mod crypto;",
        "pub mod error;",
        "pub mod interlock;",
        "pub mod state;",
        "pub mod wire;",
        "pub use crate::state::{ConsentMachine, ConsentState};",
    ]:
        if token not in text:
            fail(f"src/lib.rs missing token: {token}")
    print("lib surface: PASS")

def check_ci_workflow() -> None:
    text = read(".github/workflows/ci.yml")
    for token in [
        "name: ci",
        "workflow_dispatch",
        "permissions:",
        "contents: read",
        "cancel-in-progress",
        "tools/verify_consent_repository.py",
    ]:
        if token not in text:
            fail(f"ci.yml missing token: {token}")
    print("ci workflow: PASS")

def check_ci_job_count() -> None:
    text = read(".github/workflows/ci.yml")
    for job in JOBS:
        if re.search(rf"^\s+{re.escape(job)}:\s*$", text, flags=re.MULTILINE) is None:
            fail(f"ci.yml missing job: {job}")
    print("ci job count: PASS")

def check_contact_policy() -> None:
    stale = ["info" + "@axonos.org", "denis" + "@axonos.org", "axonosorg" + "@gmail.com"]
    for rel in PRIMARY:
        text = read(rel)
        for token in stale:
            if token in text:
                fail(f"{rel} contains stale contact: {token}")
    print("contact policy: PASS")

def check_clean_public_surface() -> None:
    stale = [
        "SYM" + ".BOT",
        "sym" + ".bot",
        "Hong" + "wei",
        "Mesh " + "Memory " + "Protocol",
    ]
    for rel in ["README.md", "Cargo.toml", "src/lib.rs", "CHANGELOG.md", "SECURITY.md"]:
        text = read(rel)
        for token in stale:
            if token in text:
                fail(f"{rel} contains stale external-protocol token: {token}")
    print("clean public surface: PASS")

def check_overclaim_guard() -> None:
    forbidden = [
        "FDA " + "510(k) approved",
        "FDA " + "clearance granted",
        "regulatory " + "approval granted",
        "L3 " + "hardware validated",
        "certified " + "medical device",
    ]
    for rel in PRIMARY:
        text = read(rel)
        for token in forbidden:
            if token in text:
                fail(f"{rel} contains overclaim token: {token}")
    print("overclaim guard: PASS")

def check_standard_mapping() -> None:
    text = read("README.md") + "\n" + read("src/lib.rs")
    for token in ["AOS-0004", "AOS-0005", "AOS-0009", "AOS-0012"]:
        if token not in text:
            fail(f"standard mapping missing token: {token}")
    print("standard mapping: PASS")

def check_security_policy() -> None:
    text = read("SECURITY.md")
    for token in [
        "security@axonos.org",
        "consent bypass",
        "withdrawal bypass",
        "PGP key publication is pending",
        "not a clinical",
    ]:
        if token not in text:
            fail(f"SECURITY.md missing token: {token}")
    print("security policy: PASS")

def check_changelog() -> None:
    text = read("CHANGELOG.md")
    for token in ["Unreleased", "Seventeen-job", "AxonOS Standard mapping", "Non-claims"]:
        if token not in text:
            fail(f"CHANGELOG missing token: {token}")
    print("changelog: PASS")

def check_license_files() -> None:
    if "Apache License" not in read("LICENSE-APACHE"):
        fail("LICENSE-APACHE does not look like Apache license text")
    if "MIT License" not in read("LICENSE-MIT"):
        fail("LICENSE-MIT does not look like MIT license text")
    print("license files: PASS")

def check_source_tree() -> None:
    require_dir("src")
    for rel in ["src/lib.rs", "src/crypto.rs", "src/error.rs", "src/interlock.rs", "src/state.rs", "src/wire.rs"]:
        require_file(rel)
    print("source tree: PASS")

def check_path_hygiene() -> None:
    suspicious = []
    for p in ROOT.rglob("*"):
        if ".git" in p.parts:
            continue
        name = p.name
        if name.startswith("=") or '"' in name or "'" in name or name.endswith(".tar"):
            suspicious.append(str(p.relative_to(ROOT)))
    if suspicious:
        fail(f"suspicious path names: {suspicious}")
    print("path hygiene: PASS")

def check_rustdoc_hygiene() -> None:
    text = read("src/lib.rs")
    if "//! # axonos-consent" not in text:
        fail("src/lib.rs missing crate-level rustdoc title")
    if "This crate does not claim" not in text:
        fail("src/lib.rs missing non-claim language")
    if line_count("src/lib.rs") < 65:
        fail("src/lib.rs collapsed")
    print("rustdoc hygiene: PASS")

CHECKS = {
    "repository-contract": check_repository_contract,
    "line-counts": check_line_counts,
    "cargo-manifest": check_cargo_manifest,
    "readme-surface": check_readme_surface,
    "lib-surface": check_lib_surface,
    "ci-workflow": check_ci_workflow,
    "ci-job-count": check_ci_job_count,
    "contact-policy": check_contact_policy,
    "clean-public-surface": check_clean_public_surface,
    "overclaim-guard": check_overclaim_guard,
    "standard-mapping": check_standard_mapping,
    "security-policy": check_security_policy,
    "changelog": check_changelog,
    "license-files": check_license_files,
    "source-tree": check_source_tree,
    "path-hygiene": check_path_hygiene,
    "rustdoc-hygiene": check_rustdoc_hygiene,
}

def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] not in CHECKS:
        print("Usage: verify_consent_repository.py <check>")
        print("")
        print("Available checks:")
        for check in JOBS:
            print(f"  - {check}")
        sys.exit(2)
    CHECKS[sys.argv[1]]()

if __name__ == "__main__":
    main()
