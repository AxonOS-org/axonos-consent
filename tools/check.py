#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
# SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
"""The repository contract of axonos-consent.

    python3 tools/check.py           run every check; exit 1 on any failure
    python3 tools/check.py --write   rewrite vectors/SHA256SUMS first

versions   the crate, specification and MSRV agree everywhere they are stated
wire       the magic and frame length in SPEC and README match src/wire.rs
vectors    every vector has both files, and SHA256SUMS covers exactly them
spdx       every source file opens with the two SPDX lines
claims     retired claims stay retired on the public surface
badges     every number a README badge states is the number in the code
links      every relative link and SPEC anchor in the Markdown resolves
fuzzkey    the forgery fuzzer's key is the point derived from its label
hygiene    no template placeholders, no private keys, no retired files
"""

from __future__ import annotations

import hashlib
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
FAILURES: list[str] = []
SKIP = {".git", "target", "corpus"}


def fail(check: str, message: str) -> None:
    FAILURES.append(f"{check:9} {message}")


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def find(pattern: str, text: str, what: str) -> str:
    match = re.search(pattern, text, re.M)
    if not match:
        fail("versions", f"cannot find {what}")
        return "?"
    return match.group(1)


def text_files(*globs: str) -> list[pathlib.Path]:
    out = []
    for glob in globs:
        out += [p for p in ROOT.glob(glob) if p.is_file() and not SKIP & set(p.parts)]
    return sorted(set(out))


def versions() -> None:
    crate = find(r'^version = "([^"]+)"', read("Cargo.toml"), "the crate version")
    msrv = find(r'^rust-version = "([^"]+)"', read("Cargo.toml"), "the MSRV")
    spec = find(r"\*\*Version ([0-9.]+)\*\*", read("SPEC.md"), "the SPEC version")
    lib = find(r'SPEC_VERSION: &str = "([^"]+)"', read("src/lib.rs"), "SPEC_VERSION")
    cff = find(r'^version: "([^"]+)"', read("CITATION.cff"), "the CITATION version")
    top = find(r"^## \[(\d+\.\d+\.\d+)\]", read("CHANGELOG.md"), "the top CHANGELOG entry")
    readme, bib = read("README.md"), read("docs/citation.bib")
    for name, value in (("CITATION.cff", cff), ("CHANGELOG.md", top)):
        if value != crate:
            fail("versions", f"{name} says {value}, Cargo.toml says {crate}")
    if lib != spec:
        fail("versions", f"SPEC_VERSION is {lib}, SPEC.md is {spec}")
    expectations = [
        ("README.md", readme, f"**{crate}**"),
        ("README.md", readme, f'tag = "v{crate}"'),
        ("README.md", readme, f"Specification {spec}"),
        ("README.md", readme, f"MSRV-{msrv}"),
        ("docs/citation.bib", bib, f"version {spec}"),
        ("docs/citation.bib", bib, "{" + crate + "}"),
        ("CONTRIBUTING.md", read("CONTRIBUTING.md"), f"**{msrv}**"),
        (".github/workflows/ci.yml", read(".github/workflows/ci.yml"), f"toolchain: {msrv}"),
    ]
    for name, text, needle in expectations:
        if needle not in text:
            fail("versions", f"{name} does not state {needle!r}")


def wire() -> None:
    src = read("src/wire.rs")
    magic = find(r'MAGIC: \[u8; 4\] = \*b"([^"]+)"', src, "MAGIC")
    body = int(find(r"BODY_LEN: usize = (\d+);", src, "BODY_LEN"))
    signature = int(find(r"SIGNATURE_LEN: usize = (\d+);", src, "SIGNATURE_LEN"))
    frame = body + signature
    for name, needles in (
        ("SPEC.md", [f"`{magic}`", f"exactly **{frame} bytes**"]),
        ("README.md", [magic, f"{frame} bytes"]),
    ):
        text = read(name)
        for needle in needles:
            if needle not in text:
                fail("wire", f"{name} does not state {needle!r}")


def vectors(write: bool) -> int:
    d = ROOT / "vectors"
    bins = {p.name[: -len(".bin")] for p in d.glob("vector-*.bin")}
    jsons = {p.name[: -len(".expected.json")] for p in d.glob("vector-*.expected.json")}
    for stem in sorted(bins ^ jsons):
        fail("vectors", f"{stem} lacks its .bin or its .expected.json")
    files = sorted(p for p in d.glob("vector-*") if p.is_file())
    if write:
        sums = "".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n" for p in files)
        (d / "SHA256SUMS").write_text(sums)
    listed = {}
    for line in (d / "SHA256SUMS").read_text().splitlines():
        digest, _, name = line.partition("  ")
        listed[name] = digest
    if set(listed) != {p.name for p in files}:
        fail("vectors", "SHA256SUMS does not cover exactly the vector files")
    for p in files:
        if listed.get(p.name) not in (None, hashlib.sha256(p.read_bytes()).hexdigest()):
            fail("vectors", f"{p.name} does not match SHA256SUMS")
    return len(bins)


def spdx() -> None:
    lines = (
        "SPDX-License-Identifier: Apache-2.0 OR MIT",
        "SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>",
    )
    for p in text_files("src/**/*.rs", "tests/**/*.rs", "examples/*.rs", "benches/*.rs",
                        "fuzz/fuzz_targets/*.rs", "tools/*.py"):
        head = "\n".join(p.read_text(encoding="utf-8").splitlines()[:4])
        if not all(line in head for line in lines):
            fail("spdx", f"{p.relative_to(ROOT)} does not open with both SPDX lines")


RETIRED = [
    (r"\b1648\b", "the withdrawn cycle figure"),
    (r"\bCBOR\b", "the general-purpose-encoding language"),
    (r"sig_truncated|truncated tag", "the 4-byte tag"),
    (r"ObservationGate|handle_event", "the 0.8 API"),
    (r"zero external|formally verified consent|constant-time signature", "an overclaim"),
]


def claims() -> None:
    surface = text_files("README.md", "SPEC.md", "SECURITY.md", "CONTRIBUTING.md",
                         "docs/*.md", "vectors/README.md", "fuzz/README.md", "src/**/*.rs")
    for p in surface:
        text = p.read_text(encoding="utf-8")
        for pattern, what in RETIRED:
            for m in re.finditer(pattern, text):
                line = text.count("\n", 0, m.start()) + 1
                fail("claims", f"{p.relative_to(ROOT)}:{line} brings back {what}")


WORDS = {3: "three", 10: "Ten", 20: "twenty"}


def badges(vector_count: int) -> None:
    readme = read("README.md")
    proofs = read("src/proofs.rs").count("#[kani::proof]")
    models = len(re.findall(r"fn loom_", read("src/gate.rs")))
    stated = [
        (f"Kani%20%C2%B7%20{proofs}%20harnesses", f"{proofs} Kani proofs"),
        (f"conformance-{vector_count}%20vectors", f"{vector_count} vectors"),
        (f"loom, {WORDS.get(models, models)} models", f"{models} loom models"),
    ]
    for needle, what in stated:
        if needle not in readme:
            fail("badges", f"README.md does not state {what} ({needle!r})")
    if f"{WORDS.get(proofs, proofs)} Kani proofs" not in read("SPEC.md"):
        fail("badges", f"SPEC.md does not state {proofs} Kani proofs")


def slug(heading: str) -> str:
    s = heading.strip().lower()
    s = re.sub(r"[^\w\- ]", "", s)
    return s.replace(" ", "-")


def links() -> None:
    anchors = {slug(h) for h in re.findall(r"^#{1,6} (.+)$", read("SPEC.md"), re.M)}
    for p in text_files("*.md", "docs/*.md", "docs/advisories/*.md", "vectors/README.md",
                        "fuzz/README.md", ".github/*.md"):
        text = p.read_text(encoding="utf-8")
        for target in re.findall(r"\]\((?!https?:|mailto:)([^)\s]+)\)", text):
            path, _, anchor = target.partition("#")
            resolved = (p.parent / path).resolve() if path else p
            if path and not resolved.exists():
                fail("links", f"{p.relative_to(ROOT)} links to missing {target}")
            if anchor and resolved.name == "SPEC.md" and anchor not in anchors:
                fail("links", f"{p.relative_to(ROOT)} links to missing anchor {target}")


# ── the forgery fuzzer's key: a point no one holds the secret for ────────────
# Edwards25519 arithmetic in affine coordinates, enough to hash to a point and
# clear the cofactor. Self-tested below against the base point and RFC 8032.
P25519 = 2**255 - 19
D25519 = (-121665 * pow(121666, P25519 - 2, P25519)) % P25519
SQRT_M1 = pow(2, (P25519 - 1) // 4, P25519)
L25519 = 2**252 + 27742317777372353535851937790883648493
FUZZ_LABEL = b"axonos-consent/fuzz/auth_forgery: a public key whose secret no one holds"


def ed_decompress(b: bytes):
    y = int.from_bytes(b, "little")
    sign, y = y >> 255, y & ((1 << 255) - 1)
    if y >= P25519:
        return None
    x2 = (y * y - 1) * pow(D25519 * y * y + 1, P25519 - 2, P25519) % P25519
    x = pow(x2, (P25519 + 3) // 8, P25519)
    if (x * x - x2) % P25519:
        x = x * SQRT_M1 % P25519
    if (x * x - x2) % P25519 or (x == 0 and sign):
        return None
    return (P25519 - x if x & 1 != sign else x, y)


def ed_add(a, b):
    (x1, y1), (x2, y2) = a, b
    t = D25519 * x1 * x2 * y1 * y2 % P25519
    return ((x1 * y2 + x2 * y1) * pow(1 + t, P25519 - 2, P25519) % P25519,
            (y1 * y2 + x1 * x2) * pow(1 - t, P25519 - 2, P25519) % P25519)


def ed_mul(k: int, a):
    r = (0, 1)
    while k:
        if k & 1:
            r = ed_add(r, a)
        a, k = ed_add(a, a), k >> 1
    return r


def ed_compress(a) -> bytes:
    return (a[1] | ((a[0] & 1) << 255)).to_bytes(32, "little")


def derive_fuzz_key() -> bytes:
    counter = 0
    while True:
        h = hashlib.sha512(FUZZ_LABEL + counter.to_bytes(4, "little")).digest()[:32]
        point = ed_decompress(h)
        if point is not None:
            q = ed_mul(8, point)
            if q != (0, 1) and ed_mul(L25519, q) == (0, 1):
                return ed_compress(q)
        counter += 1


def fuzzkey() -> None:
    base = ed_decompress((4 * pow(5, P25519 - 2, P25519) % P25519).to_bytes(32, "little"))
    rfc_test_1 = bytes.fromhex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
    rfc_point = ed_decompress(rfc_test_1)
    if base is None or ed_mul(L25519, base) != (0, 1) or rfc_point is None \
            or ed_compress(rfc_point) != rfc_test_1:
        fail("fuzzkey", "the Edwards25519 arithmetic fails its self-test")
        return
    key = derive_fuzz_key()
    literal = ", ".join(f"0x{b:02x}" for b in key)
    for path in ("fuzz/fuzz_targets/auth_forgery.rs", "tests/security.rs"):
        text = re.sub(r"\s+", " ", read(path))
        if literal not in text:
            fail("fuzzkey", f"{path} does not hold the derived key {key.hex()}")


def hygiene() -> None:
    patterns = [r"Use this section to tell people", r"\bTODO\b", r"\bTBD\b", r"(?i)lorem ipsum",
                r"BEGIN (OPENSSH |EC |RSA )?PRIVATE KEY"]
    for p in text_files("**/*.md", "**/*.rs", "**/*.toml", "**/*.yml", "**/*.py", "**/*.cff"):
        if p.name == "check.py":
            continue
        text = p.read_text(encoding="utf-8", errors="replace")
        for pattern in patterns:
            if re.search(pattern, text):
                fail("hygiene", f"{p.relative_to(ROOT)} matches {pattern!r}")
    for retired in ("kani", "src/interlock.rs", "tools/verify_consent_repository.py", "clippy.toml"):
        if (ROOT / retired).exists():
            fail("hygiene", f"{retired} was retired and has come back")


def main() -> int:
    write = "--write" in sys.argv
    count = vectors(write)
    versions()
    wire()
    spdx()
    claims()
    badges(count)
    links()
    fuzzkey()
    hygiene()
    if FAILURES:
        print("\n".join(FAILURES))
        print(f"\n{len(FAILURES)} failure(s)")
        return 1
    print(f"repository contract holds · {count} vectors")
    return 0


if __name__ == "__main__":
    sys.exit(main())
