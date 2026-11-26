#!/usr/bin/env python3
"""Validates data/sources/manifest.json (issue #57).

Rules: unique ids, required fields, known decision values, commercial
compatibility of shipped sources, no share-alike/non-commercial data marked as
shippable without conditions, sha256 format when present.
Exit code 1 with one line per violation.
"""
import json
import re
import sys
from pathlib import Path

REQUIRED = ["id", "name", "url", "license", "commercial_use", "share_alike", "obligations", "decision", "conditions"]
SHIPPED = {"bundle", "bundle-filtered", "conditional"}


def validate(manifest: dict) -> list[str]:
    errors = []
    if manifest.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    decisions = set(manifest.get("decision_values", {}))
    if not decisions:
        errors.append("decision_values missing or empty")
    sources = manifest.get("sources")
    if not isinstance(sources, list) or not sources:
        errors.append("sources missing or empty")
        sources = []
    seen = set()
    for src in sources:
        if not isinstance(src, dict):
            errors.append("source entry is not an object")
            continue
        sid = src.get("id", "?")
        for field in REQUIRED:
            if field not in src:
                errors.append(f"{sid}: missing {field}")
        if sid in seen:
            errors.append(f"{sid}: duplicate id")
        seen.add(sid)
        decision = src.get("decision")
        if decision not in decisions:
            errors.append(f"{sid}: unknown decision {decision!r}")
        if not str(src.get("url", "")).startswith("https://"):
            errors.append(f"{sid}: url must be https")
        if decision in SHIPPED:
            if src.get("commercial_use") is not True:
                errors.append(f"{sid}: shipped source must allow commercial use")
            if src.get("share_alike") is True and decision == "bundle":
                errors.append(f"{sid}: share-alike source cannot be bundled unconditionally")
            if decision != "bundle" and not src.get("conditions"):
                errors.append(f"{sid}: {decision} requires conditions")
            if not src.get("version"):
                errors.append(f"{sid}: shipped source needs a version")
        if "NC" in str(src.get("license", "")).split("-") and decision != "exclude":
            errors.append(f"{sid}: non-commercial license must be excluded")
        for f in src.get("files", []):
            sha = f.get("sha256")
            if sha is not None and not re.fullmatch(r"[0-9a-f]{64}", sha):
                errors.append(f"{sid}: invalid sha256 for {f.get('name')}")
    return errors


def self_test() -> list[str]:
    """Negative cases that must be rejected (run in CI with --self-test)."""
    import copy

    good = json.loads((Path(__file__).resolve().parent.parent / "data/sources/manifest.json").read_text(encoding="utf-8"))
    failures = []
    if validate(good):
        failures.append("committed manifest must be valid")
    cases = {
        "only schema_version": {"schema_version": 1},
        "empty sources": {**good, "sources": []},
        "missing decision_values": {k: v for k, v in good.items() if k != "decision_values"},
    }
    nc = copy.deepcopy(good)
    for s in nc["sources"]:
        if s["id"] == "D":
            s["decision"] = "bundle"
    cases["non-commercial bundled"] = nc
    dup = copy.deepcopy(good)
    dup["sources"].append(copy.deepcopy(dup["sources"][0]))
    cases["duplicate id"] = dup
    for name, manifest in cases.items():
        if not validate(manifest):
            failures.append(f"accepted invalid manifest: {name}")
    return failures


def main() -> int:
    if sys.argv[1:] == ["--self-test"]:
        failures = self_test()
        for f in failures:
            print(f"self-test: {f}")
        print("self-test ok" if not failures else "self-test FAILED")
        return 1 if failures else 0
    path = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent / "data/sources/manifest.json")
    errors = validate(json.loads(path.read_text(encoding="utf-8")))
    for e in errors:
        print(f"manifest: {e}")
    if not errors:
        print(f"manifest ok: {path}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
