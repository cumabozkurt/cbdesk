#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""SPDX 2.3 JSON from `cargo metadata`. This is a package list, not a file hash of every source."""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path


def spdx_id(name: str, version: str) -> str:
    raw = f"SPDXRef-{name}-{version}"
    return re.sub(r"[^A-Za-z0-9.-]", "-", raw)


def document(locked: bool) -> dict:
    command = ["cargo", "metadata", "--format-version", "1"]
    if locked:
        command.append("--locked")
    metadata = json.loads(subprocess.check_output(command, text=True))
    packages = []
    for package in metadata["packages"]:
        license_text = package.get("license") or "NOASSERTION"
        location = package.get("repository") or "NOASSERTION"
        packages.append(
            {
                "SPDXID": spdx_id(package["name"], package["version"]),
                "name": package["name"],
                "versionInfo": package["version"],
                "downloadLocation": location,
                "licenseConcluded": license_text,
                "licenseDeclared": license_text,
                "filesAnalyzed": False,
            }
        )
    return {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": "cbdesk",
        "documentNamespace": "https://github.com/cumabozkurt/cbdesk/spdx/0.0.0",
        "creationInfo": {"creators": ["Tool: tools/release/sbom.py"]},
        "packages": packages,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("-o", "--output")
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    doc = document(args.locked)
    names = {package["name"] for package in doc["packages"]}
    if "cbdesk-proto" not in names or "cbdesk-controller" not in names:
        raise SystemExit("sbom is missing a workspace package")
    if args.check:
        print(f"sbom: {len(doc['packages'])} packages")
        return
    text = json.dumps(doc, indent=2) + "\n"
    if args.output:
        Path(args.output).write_text(text, encoding="utf-8")
        print(f"wrote {args.output}")
    else:
        sys.stdout.write(text)


if __name__ == "__main__":
    main()
