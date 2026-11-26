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
    seen = set()
    for src in manifest.get("sources", []):
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


def main() -> int:
    path = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent / "data/sources/manifest.json")
    errors = validate(json.loads(path.read_text(encoding="utf-8")))
    for e in errors:
        print(f"manifest: {e}")
    if not errors:
        print(f"manifest ok: {path}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
