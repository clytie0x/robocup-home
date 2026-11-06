#!/usr/bin/env python3

# Design-inventory check for section 3.2. This is not a task score.
# CI runs this file from the spec-checker step, after the section 3.1 checker.

import hashlib
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "docs" / "tasks" / "3.2-general-purpose-service-robot.md"
EXCERPT = ROOT / "artifacts" / "sources" / "professor-3.2.txt"
GENERAL = ROOT / "artifacts" / "sources" / "professor-general.txt"
TASK_31 = ROOT / "docs" / "tasks" / "3.1-carry-my-luggage.md"

EXPECTED_HASH = "da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600"
TASK_31_HASH = "6325c732a0c7819e43ba9d65d801eb147316364b5afe701d43f7b93dd7a49633"
ROW_COUNT = 42
OUTSTANDING_LABEL = "Outstanding performance (see sec. 2.3.3)"
ABSENT_PROSE = "it gets a penalty of 250 points."
REPEAT_SENTENCE = "The default operator may repeat the command up to three times."

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


def flat(text):
    return " ".join(text.split())


def score_cell(spec, label):
    prefix = f"| {label} |"
    for line in spec.splitlines():
        if line.startswith(prefix):
            cells = [cell.strip() for cell in line.strip("|").split("|")]
            if len(cells) >= 2:
                return cells[1]
    return None


def excerpt_score(body, label):
    for line in body.splitlines():
        if label in line:
            tokens = line.split()
            if tokens:
                return tokens[-1]
    return None


def row_by_id(found, row_id):
    for cells in found:
        if cells and cells[0] == row_id:
            return cells
    return None


def collect(spec, excerpt, general, task_31):
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
    for row_id in OPEN_IDS + HARDWARE_IDS + ("GPSR-GEN-REPEAT", "GPSR-PROSE-ABSENT", "GPSR-SCORE-OUT"):
        if row_id not in seen:
            problems.append(f"missing {row_id}")
    if score_cell(spec, OUTSTANDING_LABEL) != "150":
        problems.append("sheet outstanding is not 150")
    if excerpt_score(body, OUTSTANDING_LABEL) != "150":
        problems.append("excerpt outstanding is not 150")
    general_body = page_body(general)
    general_flat = flat(general_body)
    if ABSENT_PROSE not in general_flat:
        problems.append("general excerpt lost the 250 point absence prose")
    if REPEAT_SENTENCE not in general_flat:
        problems.append("general excerpt lost the default-operator repeat sentence")
    prose = row_by_id(found, "GPSR-PROSE-ABSENT")
    if prose is None or "250 points" not in prose[1] or "2.3.1" not in prose[1]:
        problems.append("absence prose row lost 250 points")
    absent = row_by_id(found, "GPSR-OPEN-ABSENT")
    if absent is None or absent[7] != "open" or "This row cannot pass." not in absent[1]:
        problems.append("absence conflict is not open")
    elif "250 points" not in absent[1] or "–500" not in absent[1] or "Both are in this PDF." not in absent[1]:
        problems.append("absence conflict lost a cited number")
    repeat = row_by_id(found, "GPSR-GEN-REPEAT")
    if repeat is None or repeat[5] != "not-run" or REPEAT_SENTENCE not in repeat[1]:
        problems.append("default-operator repeat row is missing or credited")
    return problems


def self_test():
    failures = []
    spec = SPEC.read_text(encoding="utf-8")
    excerpt = EXCERPT.read_text(encoding="utf-8")
    general = GENERAL.read_text(encoding="utf-8")
    task_31 = TASK_31.read_bytes()
    if collect(spec, excerpt, general, task_31):
        failures.append("current inventory should pass")
        failures.extend(collect(spec, excerpt, general, task_31)[:8])
        return failures
    passed = spec.replace("| not-run | sim | none |", "| pass | sim | none |", 1)
    if not any("result pass" in item for item in collect(passed, excerpt, general, task_31)):
        failures.append("a pass result should fail")
    dropped = spec.replace("| GPSR-OPEN-CMD |", "| GPSR-OPEN-CMD-HIDDEN |", 1)
    if not any("GPSR-OPEN-CMD" in item for item in collect(dropped, excerpt, general, task_31)):
        failures.append("dropping an open row should fail")
    borrowed = spec.replace("The filename says 2025.", "The filename says 2025. 1015.", 1)
    if not any("borrows" in item for item in collect(borrowed, excerpt, general, task_31)):
        failures.append("a borrowed total should fail")
    quiet = excerpt.replace("1500", "1501", 1)
    if not any("1500" in item for item in collect(spec, quiet, general, task_31)):
        failures.append("a changed total in the excerpt should fail")
    outstanding = spec.replace("| 150 | no |", "| 999 | no |", 1)
    if not any("outstanding" in item for item in collect(outstanding, excerpt, general, task_31)):
        failures.append("sheet outstanding 999 should fail")
    prose = spec.replace(
        "a penalty of 250 points when a team",
        "a penalty of 500 points when a team",
        1,
    )
    if not any("250" in item for item in collect(prose, excerpt, general, task_31)):
        failures.append("absence prose 500 should fail")
    general_prose = general.replace(ABSENT_PROSE, "it gets a penalty of 500 points.", 1)
    if not any("250" in item for item in collect(spec, excerpt, general_prose, task_31)):
        failures.append("general excerpt absence prose 500 should fail")
    conflict = spec.replace("Both are in this PDF. ", "", 1)
    if not any("conflict" in item for item in collect(conflict, excerpt, general, task_31)):
        failures.append("removing the absence conflict sentence should fail")
    moved = bytes(task_31).replace(b"1015", b"1016", 1)
    if not any("section 3.1" in item for item in collect(spec, excerpt, general, moved)):
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
        GENERAL.read_text(encoding="utf-8"),
        TASK_31.read_bytes(),
    )
    if problems:
        print("\n".join(problems))
        return 1
    print(f"ok rows={ROW_COUNT} not-run={ROW_COUNT} open={len(OPEN_IDS)} hardware={len(HARDWARE_IDS)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
