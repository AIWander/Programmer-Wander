#!/usr/bin/env python3
"""Codex CLI adapter for the optional Programmer-Wander guard policy."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


if __name__ == "__main__":
    policy = Path(__file__).resolve().parents[2] / "shared" / "policy" / "programmer_hook.py"
    raise SystemExit(subprocess.call([sys.executable, str(policy), *sys.argv[1:], "--host", "codex"]))
