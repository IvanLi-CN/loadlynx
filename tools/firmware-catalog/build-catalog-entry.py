#!/usr/bin/env python3
"""Build a LoadLynx firmware catalog from local artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_sha(repo_root: Path) -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "HEAD"],
            cwd=repo_root,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except Exception:
        return "unknown"


def version_from_file(repo_root: Path, crate: str) -> str:
    version_path = repo_root / "tmp" / f"{crate}-fw-version.txt"
    if version_path.exists():
        return version_path.read_text(encoding="utf-8").strip()
    return f"{crate} unknown"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--target", choices=["digital_esp32s3", "analog_stm32g431"], required=True)
    parser.add_argument("--artifact-id", required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--package-version", required=True)
    parser.add_argument("--build-profile", default=os.environ.get("PROFILE", "release"))
    parser.add_argument("--protocol", default="loadlynx.cdc.v1")
    parser.add_argument("--feature", action="append", default=[])
    parser.add_argument("--file", action="append", type=Path, required=True)
    parser.add_argument("--file-kind", action="append", default=[])
    parser.add_argument("--flash-address", action="append", default=[])
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    repo_root = args.repo_root.resolve()
    crate = "digital" if args.target == "digital_esp32s3" else "analog"
    files = []
    for index, raw_path in enumerate(args.file):
        path = raw_path if raw_path.is_absolute() else repo_root / raw_path
        kind = args.file_kind[index] if index < len(args.file_kind) else "elf"
        flash_address = args.flash_address[index] if index < len(args.flash_address) else None
        files.append(
            {
                "kind": kind,
                "path": str(path),
                "sha256": sha256(path),
                "size": path.stat().st_size,
                "flash_address": int(flash_address, 0) if flash_address else None,
            }
        )

    artifact = {
        "artifact_id": args.artifact_id,
        "name": args.name,
        "target": args.target,
        "package_version": args.package_version,
        "git_sha": git_sha(repo_root),
        "build_id": version_from_file(repo_root, crate),
        "build_profile": args.build_profile,
        "features": sorted(set(args.feature)),
        "protocol": args.protocol,
        "defmt": {
            "enabled": True,
            "encoding": "defmt-espflash" if args.target == "digital_esp32s3" else "defmt",
            "elf_sha256": files[0]["sha256"] if files else None,
            "table_sha256": None,
        },
        "files": files,
    }
    catalog = {"schema_version": "1", "artifacts": [artifact]}
    rendered = json.dumps(catalog, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")


if __name__ == "__main__":
    main()
