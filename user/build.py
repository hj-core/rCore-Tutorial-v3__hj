"""
This module is intended for building user apps.

The apps are processed in sorted name order, and test
apps, i.e., apps starting with "test_", are included only
if the environment variable "TEST" is set to 1. For each
app, it builds the binary file. The arguments passed to
it are redirected to the cargo build command.
"""

import os
import subprocess
import sys
from pathlib import Path

_APP_SRC_DIR = "src/bin"


def main():
    app_names = _collect_app_names()
    app_names.sort()

    for name in app_names:
        print("Building", name, "...")
        _cargo_build(name)


def _collect_app_names() -> list[str]:
    include_test = os.environ.get("TEST", "0") == "1"
    return [
        p.name.removesuffix(".rs")
        for p in Path(_APP_SRC_DIR).iterdir()
        if p.is_file() and p.suffix == ".rs"
        if include_test or not p.name.startswith("test_")
    ]


def _cargo_build(app_name: str):
    cmd = ["cargo", "build"]
    cmd.extend(sys.argv[1:])
    cmd.append(app_name)
    subprocess.run(cmd, check=True)


if __name__ == "__main__":
    main()
