#!/usr/bin/env python3

import hashlib
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "docs" / "tasks" / "3.1-carry-my-luggage.md"
README = ROOT / "README.md"
WORKLOG = ROOT / "WORKLOG.md"
SOURCES = ROOT / "artifacts" / "sources"
SUMS = SOURCES / "SHA256SUMS"
TASK_EXCERPT = SOURCES / "professor-3.1.txt"
GENERAL_EXCERPT = SOURCES / "professor-general.txt"

EXPECTED_HASH = "da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600"
EXPECTED_SIZE = 1129757
EXPECTED_PAGES = 52
LAPTOP_PATH = r"C:\Users\teera\Downloads\at-home-robocup-my-2025-1.pdf"

REQUIRED_OPEN = (
    "CML-GEN-MAIN",
    "CML-SHEET-START",
    "CML-OPEN-PARTIAL",
    "CML-OPEN-ABSENT",
    "CML-OPEN-CAP",
    "CML-OPEN-DROP",
    "CML-OPEN-FOLLOW",
)

REQUIRED_ASSUMPTIONS = (
    "CML-ASM-AVOID",
    "CML-ASM-SIZE",
    "CML-ASM-FEW",
    "CML-ASM-CLOCK",
    "CML-ASM-ORDER",
)

REQUIRED_ADDED = ("CML-DER-CAP", "CML-BAG-DEF")

REQUIRED_PHRASES = (
    "3.1 Carry My Luggage",
    "1015",
    "101.5",
    "5 minutes",
    "within 2 m",
    EXPECTED_HASH,
    LAPTOP_PATH,
    "2024-07-09",
    "not the baseline",
    "not-run",
    "arbitrary order",
    "stiff, vertical handles",
    "previously unknown public place",
)


def page_body(path):
    text = path.read_text(encoding="utf-8")
    return text.split("BEGIN_PAGE_TEXT", 1)[1].split("END_PAGE_TEXT", 1)[0]


def score_rows(body):
    rows = []
    for line in body.splitlines():
        match = re.match(r"^(.*?)\s+([–-]?\d+)\s*$", line.strip())
        if not match:
            continue
        label = match.group(1).strip()
        if not label or label.startswith("Chapter") or label.startswith("---"):
            continue
        points = int(match.group(2).replace("–", "-"))
        rows.append((label, points))
    return rows


def spec_score_rows(spec):
    rows = []
    capture = False
    for line in spec.splitlines():
        if line.startswith("| Label |"):
            capture = True
            continue
        if capture:
            if not line.startswith("|"):
                break
            if line.startswith("| ---"):
                continue
            cells = [cell.strip() for cell in line.strip("|").split("|")]
            if len(cells) < 2:
                continue
            rows.append((cells[0], int(cells[1])))
    return rows


def requirement_rows(spec):
    rows = []
    for line in spec.splitlines():
        if not line.startswith("| CML-"):
            continue
        rows.append([cell.strip() for cell in line.strip("|").split("|")])
    return rows


def arithmetic(rows):
    total = next(points for label, points in rows if label.startswith("Total Score"))
    outstanding = next(points for label, points in rows if label.startswith("Outstanding"))
    positive = []
    for label, points in rows:
        if label.startswith("Total Score"):
            continue
        if points < 0:
            break
        positive.append(points)
    return positive, total, outstanding


def check_math(rows, origin):
    problems = []
    positive, total, outstanding = arithmetic(rows)
    summed = sum(positive)
    if summed != total:
        problems.append(f"{origin} positive sum {summed} != printed total {total}")
    if summed != 1015 or total != 1015:
        problems.append(f"{origin} total {total} sum {summed} is not 1015")
    if len(positive) != 9:
        problems.append(f"{origin} expected 9 positive lines, found {len(positive)}")
    if outstanding != 101:
        problems.append(f"{origin} outstanding {outstanding} is not the printed 101")
    if summed * 10 // 100 != outstanding:
        problems.append(f"{origin} printed outstanding is not the integer tenth of the total")
    return problems


def check_rows(spec):
    problems = []
    rows = requirement_rows(spec)
    ids = [row[0] for row in rows]
    if len(ids) != len(set(ids)):
        problems.append("duplicate requirement ids")
    for row in rows:
        if len(row) != 8:
            problems.append(f"{row[0] if row else '?'} has {len(row)} cells")
            continue
        ident, _requirement, kind, _source, test, result, gate, block = row
        if kind not in {"official", "derived", "assumption"}:
            problems.append(f"{ident} kind {kind}")
        if result != "not-run":
            problems.append(f"{ident} result {result}")
        if gate not in {"sim", "hardware"}:
            problems.append(f"{ident} gate {gate}")
        if block not in {"open", "none"}:
            problems.append(f"{ident} block {block}")
        for marker in ("Pass:", "Fail:", "Boundary:"):
            if marker not in test:
                problems.append(f"{ident} missing {marker}")
        if block == "open" and "cannot pass" not in test:
            problems.append(f"{ident} open without cannot pass")
    for ident in REQUIRED_OPEN + REQUIRED_ASSUMPTIONS + REQUIRED_ADDED:
        if ident not in ids:
            problems.append(f"missing {ident}")
    open_ids = [row[0] for row in rows if len(row) == 8 and row[7] == "open"]
    if set(open_ids) != set(REQUIRED_OPEN) or len(open_ids) != 7:
        problems.append(f"open rows {open_ids}")
    by_id = {row[0]: row for row in rows if len(row) == 8}
    if by_id.get("CML-DER-CAP", [""] * 8)[2] != "derived":
        problems.append("CML-DER-CAP is not derived")
    if by_id.get("CML-BAG-DEF", [""] * 8)[2] != "official":
        problems.append("CML-BAG-DEF is not official")
    hw = by_id.get("CML-HW-OUT")
    if hw and "p.3" not in hw[3]:
        problems.append("CML-HW-OUT does not cite printed p.3")
    if "order changes mid-trip" in spec or "in a fixed order" in spec:
        problems.append("obstacle order invents a fixed-order constraint")
    for phrase in REQUIRED_PHRASES:
        if phrase not in spec:
            problems.append(f"missing phrase {phrase}")
    if "1115" in spec or "Help Me Carry" in spec or "5.1 Help" in spec:
        problems.append("spec still treats the 2025.1 task as normative")
    if "http://" in spec or "https://" in spec:
        problems.append("spec invents or cites a URL for rule text")
    return problems


def check_sources(spec):
    problems = []
    excerpt_rows = score_rows(page_body(TASK_EXCERPT))
    written_rows = spec_score_rows(spec)
    if written_rows != excerpt_rows:
        problems.append("printed score table does not match the professor excerpt")
    problems.extend(check_math(excerpt_rows, "professor"))
    general = page_body(GENERAL_EXCERPT)
    for phrase in (
        "have a higher priority",
        "penalty of 250 points",
        "3.1 Carry My Luggage",
        "within a 2m distance",
        "5 minutes",
        "1015",
        "stiff, vertical handles",
        "previously unknown public place",
        "in arbitrary order",
    ):
        haystack = page_body(TASK_EXCERPT) + "\n" + general
        if phrase not in haystack:
            problems.append(f"professor excerpt lost {phrase}")
    header = TASK_EXCERPT.read_text(encoding="utf-8").split("BEGIN_PAGE_TEXT", 1)[0]
    if EXPECTED_HASH not in header:
        problems.append("task excerpt header lost the professor hash")
    if LAPTOP_PATH not in header:
        problems.append("task excerpt header lost the laptop path")
    for path in SOURCES.glob("*.txt"):
        text = path.read_text(encoding="utf-8")
        if path.name.startswith("professor-"):
            if text.startswith("NONBASELINE"):
                problems.append(f"{path.name} is marked nonbaseline")
        elif not text.startswith("NONBASELINE"):
            problems.append(f"{path.name} is not marked NONBASELINE")
    return problems


def professor_pdf_path():
    env = os.environ.get("PROFESSOR_PDF")
    if env:
        return Path(env)
    candidate = Path(
        "/home/ubuntu/.cursor/projects/agent-repos-robocup-home/uploads/at-home-robocup-my-2025-1_763a.pdf"
    )
    if candidate.exists():
        return candidate
    return None


def check_pdf(path):
    problems = []
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if digest != EXPECTED_HASH:
        problems.append(f"professor PDF hash {digest}")
    if len(data) != EXPECTED_SIZE:
        problems.append(f"professor PDF size {len(data)}")
    if shutil.which("pdfinfo"):
        info = subprocess.check_output(["pdfinfo", str(path)], text=True, errors="replace")
        match = re.search(r"^Pages:\s+(\d+)$", info, re.M)
        if not match or int(match.group(1)) != EXPECTED_PAGES:
            problems.append("professor PDF page count is not 52")
        if "2024-07-09" not in info and "Jul  9" not in info and "July 9" not in info:
            problems.append("professor PDF creation date is not 2024-07-09")
    if shutil.which("pdftotext"):
        text = subprocess.check_output(["pdftotext", "-layout", str(path), "-"], text=True, errors="replace")
        if "3.1 Carry My Luggage" not in text or "1015" not in text:
            problems.append("professor PDF text lost section 3.1 or total 1015")
        if "5.1 Help Me Carry" in text:
            problems.append("professor PDF unexpectedly contains the 2025.1 task title")
    return problems


def relative_links(root):
    problems = []
    link = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    for path in root.rglob("*.md"):
        if ".git" in path.parts:
            continue
        text = path.read_text(encoding="utf-8")
        for match in link.finditer(text):
            target = match.group(1).split("#", 1)[0].strip()
            if not target or "://" in target or target.startswith("mailto:"):
                continue
            if not (path.parent / target).resolve().exists():
                problems.append(f"missing link {path.relative_to(root)} -> {target}")
    return problems


def thai_hits(root):
    hits = []
    for path in root.rglob("*"):
        if not path.is_file() or ".git" in path.parts:
            continue
        if path.suffix not in {".md", ".py", ".txt", ".rs", ".toml", ".yml", ".yaml"}:
            continue
        data = path.read_text(encoding="utf-8", errors="replace")
        for number, line in enumerate(data.splitlines(), 1):
            if any("\u0e00" <= char <= "\u0e7f" for char in line):
                hits.append(f"thai {path.relative_to(root)}:{number}")
    return hits


def check_readme_and_log():
    problems = []
    readme = README.read_text(encoding="utf-8")
    if "5.1 Help Me Carry" in readme or "1115" in readme:
        problems.append("README still points at the 2025.1 baseline")
    if "3.1 Carry My Luggage" not in readme:
        problems.append("README does not name section 3.1")
    log = WORKLOG.read_text(encoding="utf-8")
    current = log.split("## Current status (documentation bootstrap", 1)[0]
    if EXPECTED_HASH not in current:
        problems.append("current WORKLOG does not record the professor hash")
    if "not the normative baseline" not in current and "not the baseline" not in current:
        problems.append("current WORKLOG does not retire the previous baseline")
    return problems


def hash_sources():
    lines = []
    for path in sorted(SOURCES.iterdir()):
        if not path.is_file() or path.name == "SHA256SUMS":
            continue
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        lines.append(f"{digest}  {path.name}")
    return "\n".join(lines) + "\n"


def check_sums():
    if not SUMS.exists():
        return ["SHA256SUMS missing"]
    if SUMS.read_text(encoding="utf-8") != hash_sources():
        return ["SHA256SUMS does not match artifacts/sources"]
    return []


def collect(spec=None, pdf=None):
    spec = SPEC.read_text(encoding="utf-8") if spec is None else spec
    problems = []
    problems.extend(check_rows(spec))
    problems.extend(check_sources(spec))
    problems.extend(check_sums())
    problems.extend(check_readme_and_log())
    problems.extend(relative_links(ROOT))
    problems.extend(thai_hits(ROOT))
    pdf_path = professor_pdf_path() if pdf is None else pdf
    if pdf_path is not None and Path(pdf_path).exists():
        problems.extend(check_pdf(Path(pdf_path)))
    elif pdf is not None:
        problems.append("professor PDF missing")
    return problems


def self_test():
    failures = []
    spec = SPEC.read_text(encoding="utf-8")
    if collect(spec):
        failures.append("current spec should pass before mutations")
        failures.extend(collect(spec)[:8])
        return failures
    tampered = spec.replace("| 1015 | total |", "| 1016 | total |", 1)
    if not any("does not match" in item or "1015" in item for item in collect(tampered)):
        failures.append("tampered total should fail")
    passed = spec.replace("| not-run | sim | none |", "| pass | sim | none |", 1)
    if not any("result pass" in item for item in collect(passed)):
        failures.append("pass result should fail")
    contaminated = spec.replace("not the baseline", "not the baseline\n1115 is the total.", 1)
    if not any("2025.1" in item or "1115" in item for item in collect(contaminated)):
        failures.append("1115 contamination should fail")
    opened = spec.replace(
        "This row cannot pass while both readings are treated as settled.",
        "Both readings are settled.",
        1,
    )
    if not any("cannot pass" in item for item in collect(opened)):
        failures.append("removed cannot-pass should fail")
    ordered = spec.replace("in an arbitrary order", "in a fixed order", 1)
    if not any("fixed-order" in item or "arbitrary order" in item for item in collect(ordered)):
        failures.append("fixed obstacle order should fail")
    rows = score_rows(page_body(TASK_EXCERPT))
    broken = list(rows)
    broken[-1] = (broken[-1][0], 999)
    if not check_math(broken, "self"):
        failures.append("broken excerpt math should fail")
    return failures


def main(argv):
    if "--self-test" in argv:
        failures = self_test()
        if failures:
            print("\n".join(failures))
            return 1
        print("self-test ok")
        return 0
    if "--write-sums" in argv:
        SUMS.write_text(hash_sources(), encoding="utf-8")
        print(f"wrote {SUMS}")
        return 0
    problems = collect()
    if problems:
        print("\n".join(problems))
        return 1
    spec = SPEC.read_text(encoding="utf-8")
    print(f"ok rows={len(requirement_rows(spec))} pdf={professor_pdf_path() is not None}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
