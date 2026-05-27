#!/usr/bin/env python3
"""
Bump renamed crates from 0.10.0 to 0.10.1, leave non-renamed crates at 0.10.0.

Renamed crates use `version.workspace = true`, so we'd normally just
bump `workspace.package.version`. But that would also bump cubecl-common,
cubecl-ir, etc. — which we DON'T republish. So instead:

- Bump `workspace.package.version` to "0.10.1"
- For each NON-RENAMED crate, override with literal `version = "0.10.0"`
  in its [package] section so it stays at upstream's version (and
  won't be re-published — these aren't ours).
- Update inter-fork dep `version = "0.10.0"` -> `version = "0.10.1"` for
  the renamed deps only.
"""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent

RENAMED = {
    "cubecl-runtime",
    "cubecl-cuda",
    "cubecl-wgpu",
    "cubecl-core",
    "cubecl-opt",
    "cubecl-cpp",
    "cubecl-cpu",
    "cubecl-hip",
    "cubecl-spirv",
    "cubecl-std",
    "cubecl",
}
# Non-renamed crates that have `version.workspace = true` and that we
# need to KEEP at 0.10.0 (not publish — these stay on upstream)
NON_RENAMED_IN_WORKSPACE = {
    "cubecl-common",
    "cubecl-ir",
    "cubecl-macros",
    "cubecl-macros-internal",
    "cubecl-zspace",
}

def main():
    # 1. Bump workspace.package.version
    ws_toml = ROOT / "Cargo.toml"
    text = ws_toml.read_text()
    text = re.sub(
        r'^(version\s*=\s*)"0\.10\.0"',
        r'\1"0.10.1"',
        text,
        count=1,
        flags=re.MULTILINE,
    )
    ws_toml.write_text(text)
    print(f"BUMPED workspace.package.version -> 0.10.1: {ws_toml}")

    # 2. For each non-renamed crate, override version to 0.10.0 explicitly
    for c in NON_RENAMED_IN_WORKSPACE:
        p = ROOT / "crates" / c / "Cargo.toml"
        if not p.exists():
            continue
        text = p.read_text()
        # Replace `version.workspace = true` with `version = "0.10.0"` in [package]
        new_text = re.sub(
            r'^version\.workspace\s*=\s*true',
            'version = "0.10.0"',
            text,
            count=1,
            flags=re.MULTILINE,
        )
        if new_text != text:
            p.write_text(new_text)
            print(f"PINNED 0.10.0: {p}")
        else:
            print(f"noop (no version.workspace): {p}")

    # 3. Update inter-fork dep version strings: 0.10.0 -> 0.10.1
    # Match within any Cargo.toml that has cubecl-<renamed> = { ... version = "0.10.0" ... }
    all_tomls = list((ROOT / "crates").glob("*/Cargo.toml")) + \
                list((ROOT / "examples").glob("*/Cargo.toml"))

    for p in all_tomls:
        text = p.read_text()
        changed = False
        for dep in RENAMED:
            # Pattern: line containing `<dep>` somewhere and `version = "0.10.0"` on the same or
            # continuation line. We update version = "0.10.0" -> "0.10.1" only inside that line.
            # Because the patterns are inline table form (single line),
            # we match the full line and rewrite version inline.
            pattern = re.compile(
                rf'^(\s*{re.escape(dep)}\s*=\s*\{{[^}}]*?)version\s*=\s*"0\.10\.0"',
                re.MULTILINE | re.DOTALL,
            )
            new_text, n = pattern.subn(
                rf'\1version = "0.10.1"',
                text,
            )
            if n > 0:
                text = new_text
                changed = True

            # Also handle multi-line table form (less common but possible)
            pattern2 = re.compile(
                rf'^(\s*{re.escape(dep)}\s*=\s*\{{[^}}]*?)version\s*=\s*"=\s*0\.10\.0"',
                re.MULTILINE | re.DOTALL,
            )
            new_text, n = pattern2.subn(
                rf'\1version = "=0.10.1"',
                text,
            )
            if n > 0:
                text = new_text
                changed = True

        if changed:
            p.write_text(text)
            print(f"UPDATED inter-fork deps: {p}")

if __name__ == "__main__":
    main()
