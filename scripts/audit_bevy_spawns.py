#!/usr/bin/env python3
"""Print the main-world entity spawn sites Bevy added or removed between two
versions, for auditing q_screens' persistent types (src/persistent.rs).

Usage: audit_bevy_spawns.py <old-version> <new-version> [-C <lines>] [--registry <dir>]

Both versions' `bevy_*` crates are read from the cargo registry. Test code and
async-task spawns are ignored, and line numbers are dropped so unrelated edits
don't show up. A new site that spawns an engine-internal entity means a new
registration and test in src/persistent.rs.
"""

import argparse
import os
import re
import signal
from pathlib import Path

SPAWN = re.compile(r"\.\s*(?:spawn(?:_empty|_batch)?|with_child)\s*\(")
TEST_ATTR = re.compile(r"#\[cfg\((?:all\()?test\b[^\]]*\]")
# `.spawn(...)` on a task pool or executor spawns a future, not an entity.
TASK = re.compile(r"\.\s*spawn\s*\(\s*(async|move|\{|future|f\)|task|AssertUnwind|fut)")


def find_registry(versions: list[str]) -> Path:
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    for index in sorted((cargo_home / "registry/src").glob("*")):
        if all((index / f"bevy_ecs-{v}").is_dir() for v in versions):
            return index
    raise SystemExit(
        f"no registry under {cargo_home}/registry/src has bevy_ecs for {' and '.join(versions)}; "
        "run `cargo fetch` in a project on each version, or pass --registry"
    )


def strip_test_items(src: str) -> str:
    """Blank out every item gated on `cfg(test)`, from the attribute to the end
    of its item (`;` or the matching `}`)."""
    chars = list(src)
    pos = 0
    while m := TEST_ATTR.search(src, pos):
        brace, semi = src.find("{", m.end()), src.find(";", m.end())
        if brace == -1 or (semi != -1 and semi < brace):
            end = semi + 1 if semi != -1 else len(src)
        else:
            depth, end = 0, len(src)
            for i in range(brace, len(src)):
                depth += {"{": 1, "}": -1}.get(src[i], 0)
                if depth == 0:
                    end = i + 1
                    break
        for i in range(m.start(), end):
            if chars[i] != "\n":
                chars[i] = " "
        pos = end
    return "".join(chars)


def spawn_sites(registry: Path, version: str) -> dict[tuple[str, str, str], tuple[Path, int]]:
    """(crate, file, call text) -> (source path, line) for every main-world spawn
    call. Keys leave out line numbers so unrelated edits don't show up."""
    sites = {}
    for path in registry.glob(f"bevy_*-{version}/src/**/*.rs"):
        relative = path.relative_to(registry)
        crate = relative.parts[0].removesuffix(f"-{version}")
        file = "/".join(relative.parts[1:])
        if "/tests/" in f"/{file}" or file.endswith("tests.rs"):
            continue
        body = strip_test_items(path.read_text(encoding="utf-8", errors="ignore"))
        for m in SPAWN.finditer(body):
            start = body.rfind("\n", 0, m.start()) + 1
            if body[start : body.find("\n", m.start())].strip().startswith(("//", "#")):
                continue
            context = " ".join(body[m.start() : m.start() + 70].split())
            if not TASK.match(context):
                sites.setdefault((crate, file, context), (path, body.count("\n", 0, m.start()) + 1))
    return sites


def show(label: str, site, location, context: int) -> None:
    """Print a site like `rg -C <context>`: `:` marks the match, `-` context."""
    crate, file, _ = site
    path, line = location
    print(f"{label} {crate}/{file}:{line}")
    lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
    for n in range(max(1, line - context), min(len(lines), line + context) + 1):
        print(f"    {n}{':' if n == line else '-'} {lines[n - 1]}")
    print()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("old", help="previously audited Bevy version, e.g. 0.19.1")
    parser.add_argument("new", help="Bevy version to audit, e.g. 0.20.0")
    parser.add_argument("--registry", type=Path, default=None)
    parser.add_argument(
        "-C", "--context", type=int, default=3, help="lines of context around each site"
    )
    args = parser.parse_args()
    registry = args.registry or find_registry([args.old, args.new])

    old, new = spawn_sites(registry, args.old), spawn_sites(registry, args.new)
    crates = lambda sites: {crate for crate, _, _ in sites}  # noqa: E731
    both = crates(old) & crates(new)
    for crate in sorted(crates(new) - both):
        print(f"NEW CRATE {crate}: not downloaded for {args.old} or new; review all of its sites:\n")
        for site in sorted((s for s in new if s[0] == crate), key=lambda s: (s[1], new[s][1])):
            show("SITE   ", site, new[site], args.context)
    for crate in sorted(crates(old) - both):
        print(f"GONE CRATE {crate}: not downloaded for {args.new} or removed\n")
    by_line = lambda sites: lambda s: (s[0], s[1], sites[s][1])  # noqa: E731
    added = sorted((s for s in new.keys() - old.keys() if s[0] in both), key=by_line(new))
    removed = sorted((s for s in old.keys() - new.keys() if s[0] in both), key=by_line(old))
    for site in added:
        show("ADDED  ", site, new[site], args.context)
    for site in removed:
        show("REMOVED", site, old[site], args.context)
    print(f"{len(added)} added, {len(removed)} removed ({args.old} -> {args.new})")


if __name__ == "__main__":
    # Exit quietly when piped into `head` or `less`, like other CLI tools.
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    main()
