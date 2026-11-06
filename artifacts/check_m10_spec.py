#!/usr/bin/env python3

# Design-inventory check for section 3.2. This is not a task score.
# CI does not run this file. The local command is in WORKLOG.md.

import hashlib
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "docs" / "tasks" / "3.2-general-purpose-service-robot.md"
EXCERPT = ROOT / "artifacts" / "sources" / "professor-3.2.txt"
TASK_31 = ROOT / "docs" / "tasks" / "3.1-carry-my-luggage.md"

EXPECTED_HASH = "da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600"
TASK_31_HASH = "b88ea111e77f639c3fe589c10874c222ba087b3c07eb3cf462154ba14411bc06"
ROW_COUNT = 41

OPEN_IDS = (
    "GPSR-OPEN-OUT",
    "GPSR-OPEN-CMD",
    "GPSR-OPEN-RETURN",
    "GPSR-OPEN-HELP",
    "GPSR-OPEN-ABSENT",
    "GPSR-OPEN-START",
)
HARDWARE_IDS = ("GPSR-HW-DOOR", "GPSR-HW-ARENA")

EXCERPT_PHRASES = (
    "3.2 General Purpose Service Robot",
    "Execute 3 commands requested by the operator.",
    "When the door opens, the",
    "The maximum time for this test is 5 minutes.",
    "The main task allows partial scoring (per completed command).",
    "e.g. after two retries",
    "https://github.com/johaq/CommandGenerator",
    "3×400",
    "3×100",
    "3×–50",
    "penalty3×–400",
    "–500",
    "–100",
    "1500",
    "don’t reveal them to the teams",
)


def page_body(text):
    return text.split("BEGIN_PAGE_TEXT", 1)[1].split("END_PAGE_TEXT", 1)[0]


def rows(spec):
    found = []
    for line in spec.splitlines():
        if not line.startswith("| GPSR-"):
            continue
        found.append([cell.strip() for cell in line.strip("|").split("|")])
    return found


def collect(spec, excerpt, task_31):
    problems = []
    if task_31 is not None:
        digest = hashlib.sha256(task_31).hexdigest()
        if digest != TASK_31_HASH:
            problems.append("section 3.1 specification hash changed")
    if not excerpt.startswith("NORMATIVE EXCERPT"):
        problems.append("section 3.2 excerpt lost its header")
    if excerpt.startswith("NONBASELINE"):
        problems.append("section 3.2 excerpt is marked nonbaseline")
    if EXPECTED_HASH not in excerpt:
        problems.append("section 3.2 excerpt lost the professor hash")
    body = page_body(excerpt)
    for phrase in EXCERPT_PHRASES:
        if phrase not in body:
            problems.append(f"excerpt lost {phrase}")
        if phrase not in spec:
            problems.append(f"spec lost {phrase}")
    if "1015" in spec or "1115" in spec or "Help Me Carry" in spec:
        problems.append("section 3.2 spec borrows another task total")
    found = rows(spec)
    if len(found) != ROW_COUNT:
        problems.append(f"row count {len(found)}")
    seen = set()
    for cells in found:
        if len(cells) != 8:
            problems.append(f"row width {cells[0] if cells else '?'}")
            continue
        row_id, _requirement, kind, _source, _test, result, gate, block = cells
        seen.add(row_id)
        if kind not in {"official", "derived", "assumption"}:
            problems.append(f"{row_id} kind {kind}")
        if result != "not-run":
            problems.append(f"{row_id} result {result}")
        if gate not in {"sim", "hardware"}:
            problems.append(f"{row_id} gate {gate}")
        if block not in {"none", "open"}:
            problems.append(f"{row_id} block {block}")
        if row_id in OPEN_IDS:
            if block != "open" or "This row cannot pass." not in cells[1]:
                problems.append(f"{row_id} open row lost its block")
        elif block == "open":
            problems.append(f"{row_id} unexpected open block")
        if row_id in HARDWARE_IDS and gate != "hardware":
            problems.append(f"{row_id} left hardware")
    for row_id in OPEN_IDS + HARDWARE_IDS:
        if row_id not in seen:
            problems.append(f"missing {row_id}")
    return problems


def self_test():
    failures = []
    spec = SPEC.read_text(encoding="utf-8")
    excerpt = EXCERPT.read_text(encoding="utf-8")
    task_31 = TASK_31.read_bytes()
    if collect(spec, excerpt, task_31):
        failures.append("current inventory should pass")
        failures.extend(collect(spec, excerpt, task_31)[:8])
        return failures
    passed = spec.replace("| not-run | sim | none |", "| pass | sim | none |", 1)
    if not any("result pass" in item for item in collect(passed, excerpt, task_31)):
        failures.append("a pass result should fail")
    dropped = spec.replace("| GPSR-OPEN-CMD |", "| GPSR-OPEN-CMD-HIDDEN |", 1)
    if not any("GPSR-OPEN-CMD" in item for item in collect(dropped, excerpt, task_31)):
        failures.append("dropping an open row should fail")
    borrowed = spec.replace("The filename says 2025.", "The filename says 2025. 1015.", 1)
    if not any("borrows" in item for item in collect(borrowed, excerpt, task_31)):
        failures.append("a borrowed total should fail")
    quiet = excerpt.replace("1500", "1501", 1)
    if not any("1500" in item for item in collect(spec, quiet, task_31)):
        failures.append("a changed total in the excerpt should fail")
    moved = bytes(task_31).replace(b"1015", b"1016", 1)
    if not any("section 3.1" in item for item in collect(spec, excerpt, moved)):
        failures.append("an edited section 3.1 file should fail")
    return failures


def main(argv):
    if "--self-test" in argv:
        failures = self_test()
        if failures:
            print("\n".join(failures))
            return 1
        print("self-test ok")
        return 0
    problems = collect(
        SPEC.read_text(encoding="utf-8"),
        EXCERPT.read_text(encoding="utf-8"),
        TASK_31.read_bytes(),
    )
    if problems:
        print("\n".join(problems))
        return 1
    print(f"ok rows={ROW_COUNT} not-run={ROW_COUNT} open={len(OPEN_IDS)} hardware={len(HARDWARE_IDS)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
