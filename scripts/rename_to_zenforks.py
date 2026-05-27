#!/usr/bin/env python3
"""
Rename the 11 cubecl-* crates that need patches or are transitive consumers
of patched crates, to zenforks-cubecl-*. Adds explicit `[lib] name = "cubecl_*"`
shim so source code paths `use cubecl_runtime::*` still resolve.

This is mechanical TOML editing — re-read PHASE8F_STATE.md for the rename set.
"""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent

# Crates to rename — package name changes, lib name (with underscores) stays the same.
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

def renamed_pkg(orig: str) -> str:
    return f"zenforks-{orig}"

def lib_name(orig: str) -> str:
    # cubecl-runtime -> cubecl_runtime
    return orig.replace("-", "_")

def patch_cargo_toml(path: Path, is_renamed_crate: bool, original_name: str | None):
    text = path.read_text()
    orig = text

    if is_renamed_crate:
        assert original_name is not None
        # 1. Change [package] name = "cubecl-foo" -> "zenforks-cubecl-foo"
        # Only the FIRST occurrence under [package] table.
        text = re.sub(
            rf'^name\s*=\s*"{re.escape(original_name)}"',
            f'name = "{renamed_pkg(original_name)}"',
            text,
            count=1,
            flags=re.MULTILINE,
        )

        # 2. Add or replace [lib] section with explicit name = "cubecl_foo"
        wanted_lib_name = lib_name(original_name)
        if re.search(r'^\[lib\]', text, flags=re.MULTILINE):
            # [lib] exists — does it have name = ...?
            if re.search(r'^\[lib\][^\[]*?^name\s*=', text, flags=re.MULTILINE | re.DOTALL):
                # Already has name= under [lib]; replace it
                text = re.sub(
                    r'(\[lib\][^\[]*?)^name\s*=\s*"[^"]*"',
                    rf'\1name = "{wanted_lib_name}"',
                    text,
                    flags=re.MULTILINE | re.DOTALL,
                )
            else:
                # [lib] exists without name= — insert after [lib]
                text = re.sub(
                    r'^\[lib\]\s*$\n',
                    f'[lib]\nname = "{wanted_lib_name}"\n',
                    text,
                    count=1,
                    flags=re.MULTILINE,
                )
        else:
            # No [lib] section — add it BEFORE the first [dependencies] section
            # (or at end if no [dependencies]).
            lib_block = f'\n[lib]\nname = "{wanted_lib_name}"\n'
            if re.search(r'^\[dependencies\]', text, flags=re.MULTILINE):
                text = re.sub(
                    r'^(\[dependencies\])',
                    f'{lib_block}\n\\1',
                    text,
                    count=1,
                    flags=re.MULTILINE,
                )
            else:
                text = text.rstrip() + lib_block + "\n"

    # 3. For each dep on a renamed crate, add `package = "zenforks-cubecl-foo"`.
    # Handles two forms:
    #   cubecl-foo = "0.10.0"
    #   cubecl-foo = { path = "...", version = "0.10.0", ... }
    # We only add `package = "zenforks-..."` if the key is bare (no `package`).
    for orig_dep in RENAMED:
        new_pkg = renamed_pkg(orig_dep)

        # Pattern: cubecl-foo = { ... } (table form)
        # Insert package = "..." right after the opening brace.
        # Only at start-of-line (not in random text).
        pattern = re.compile(
            rf'^(\s*){re.escape(orig_dep)}(\s*=\s*\{{)([^}}]*?)(\}})',
            re.MULTILINE,
        )

        def replace_table(m):
            indent, eq_brace, inner, close = m.groups()
            if 'package' in inner:
                # Already aliased — don't touch
                return m.group(0)
            # Insert package=... as the first key inside the table
            new_inner = f' package = "{new_pkg}",{inner}'
            return f'{indent}{orig_dep}{eq_brace}{new_inner}{close}'

        text = pattern.sub(replace_table, text)

        # Pattern: cubecl-foo = "0.10.0"  (bare string form)
        # Convert to: cubecl-foo = { package = "zenforks-cubecl-foo", version = "0.10.0" }
        bare_pattern = re.compile(
            rf'^(\s*){re.escape(orig_dep)}(\s*=\s*)"([^"]*)"\s*$',
            re.MULTILINE,
        )
        text = bare_pattern.sub(
            rf'\1{orig_dep}\2{{ package = "{new_pkg}", version = "\3" }}',
            text,
        )

    if text != orig:
        path.write_text(text)
        return True
    return False

def main():
    # Step 1: rename packages in the 11 target crates
    for orig in RENAMED:
        p = ROOT / "crates" / orig / "Cargo.toml"
        if not p.exists():
            print(f"MISSING: {p}")
            continue
        changed = patch_cargo_toml(p, is_renamed_crate=True, original_name=orig)
        print(f"{'CHANGED' if changed else 'noop'}: {p}")

    # Step 2: also patch dep-aliasing in all OTHER Cargo.toml files in the workspace
    # (examples, xtask, leaf crates that still depend on renamed crates).
    other_tomls = []
    for p in (ROOT / "crates").glob("*/Cargo.toml"):
        if p.parent.name not in RENAMED:
            other_tomls.append(p)
    for p in (ROOT / "examples").glob("*/Cargo.toml"):
        other_tomls.append(p)
    xtask_toml = ROOT / "xtask" / "Cargo.toml"
    if xtask_toml.exists():
        other_tomls.append(xtask_toml)

    for p in other_tomls:
        changed = patch_cargo_toml(p, is_renamed_crate=False, original_name=None)
        print(f"{'CHANGED' if changed else 'noop'}: {p}")

if __name__ == "__main__":
    main()
