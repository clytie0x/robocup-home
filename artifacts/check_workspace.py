#!/usr/bin/env python3

# Membership and dependency edges only. This is not a task-behavior result.

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PIN = "1.99.1"
EDITION = "2024"
EXPECTED = {
    "domain": set(),
    "harness": {
        "domain",
        "hri",
        "manipulation",
        "navigation",
        "perception",
        "sim_runtime",
        "task_3_1",
        "task_3_2",
    },
    "hri": {"domain"},
    "manipulation": {"domain"},
    "navigation": {"domain"},
    "perception": {"domain"},
    "physical": set(),
    "sim_runtime": {"domain"},
    "task_3_1": {"domain", "hri", "manipulation", "navigation", "perception"},
    "task_3_2": {"domain"},
}


def cycles(graph):
    color = {}
    problems = []

    def visit(node, stack):
        color[node] = 1
        for nxt in sorted(graph[node]):
            state = color.get(nxt, 0)
            if state == 1:
                problems.append("cycle " + " ".join(stack + [nxt]))
            elif state == 0:
                visit(nxt, stack + [nxt])
        color[node] = 2

    for node in sorted(graph):
        if color.get(node, 0) == 0:
            visit(node, [node])
    return problems


def main():
    raw = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--no-deps"],
        cwd=ROOT,
        text=True,
    )
    meta = json.loads(raw)
    members = set(meta["workspace_members"])
    packages = [pkg for pkg in meta["packages"] if pkg["id"] in members]
    problems = []
    manifest = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    if 'resolver = "3"' not in manifest:
        problems.append('resolver is not "3"')
    tool = (ROOT / "rust-toolchain.toml").read_text(encoding="utf-8")
    if f'channel = "{PIN}"' not in tool:
        problems.append(f"toolchain channel is not {PIN}")
    if "rustfmt" not in tool or "clippy" not in tool:
        problems.append("toolchain is missing rustfmt or clippy")
    names = {pkg["name"] for pkg in packages}
    if names != set(EXPECTED) or len(packages) != len(names):
        problems.append("members " + " ".join(sorted(names)))
    for pkg in packages:
        name = pkg["name"]
        if pkg.get("edition") != EDITION:
            problems.append(f"{name} edition {pkg.get('edition')}")
        if pkg.get("rust_version") != PIN:
            problems.append(f"{name} rust-version {pkg.get('rust_version')}")
        if pkg.get("publish") != []:
            problems.append(f"{name} publish {pkg.get('publish')}")
        deps = set()
        for dep in pkg["dependencies"]:
            if dep.get("kind") not in (None, "normal"):
                problems.append(f"{name} extra {dep.get('kind')} {dep['name']}")
                continue
            if dep.get("optional") or dep.get("target") or dep.get("source"):
                problems.append(f"{name} unexpected {dep['name']}")
            deps.add(dep["name"])
        if name in EXPECTED and deps != EXPECTED[name]:
            problems.append(name + " deps " + " ".join(sorted(deps)))
    problems.extend(cycles(EXPECTED))
    if problems:
        print("\n".join(problems))
        return 1
    print(f"ok members={len(names)} pin={PIN} edition={EDITION}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
