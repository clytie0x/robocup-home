#!/usr/bin/env python3

import hashlib
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "docs" / "tasks" / "3.1-carry-my-luggage.md"
SOURCES = ROOT / "artifacts" / "sources"
SUMS = SOURCES / "SHA256SUMS"
EXCERPT_2025 = SOURCES / "rb2025-5.1-help-me-carry.txt"
EXCERPT_2024 = SOURCES / "rb2024-3.1-carry-my-luggage.txt"
EXCERPT_GENERAL = SOURCES / "rb2025-general-applicable.txt"

REQUIRED_OPEN = (
    "CML-OPEN-MAIN",
    "CML-OPEN-STEP4",
    "CML-OPEN-QUEUE",
    "CML-OPEN-AVOID",
    "CML-OPEN-SIZES",
    "CML-OPEN-FEW",
    "CML-OPEN-BEG",
    "CML-OPEN-ABSENT",
    "CML-OPEN-FLOOR",
    "CML-OPEN-START",
    "CML-OPEN-CAP",
    "CML-OPEN-DROP",
    "CML-OPEN-RETURN",
    "CML-OPEN-FOLLOW",
    "CML-OPEN-BASE",
    "CML-OPEN-DOOR",
    "CML-OPEN-981",
)

REQUIRED_PHRASES = (
    "5.1 Help Me Carry",
    "test:carry-my-luggage",
    "3.1 Team Registration",
    "3.1 Carry My Luggage",
    "1015",
    "1115",
    "111.5",
    "Brazil Open",
    "Help-me-Carry",
    "not-run",
    "6 kph",
    "not an input of the 2025.1 PDF",
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
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        rows.append(cells)
    return rows


def source_keys(spec):
    keys = set()
    capture = False
    for line in spec.splitlines():
        if line.startswith("| Key |"):
            capture = True
            continue
        if capture:
            if not line.startswith("|"):
                break
            if line.startswith("| ---"):
                continue
            cells = [cell.strip() for cell in line.strip("|").split("|")]
            if cells and cells[0]:
                keys.add(cells[0])
    return keys


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
    if len(positive) != 9:
        problems.append(f"{origin} expected 9 positive lines, found {len(positive)}")
    truncated = summed * 10 // 100
    if outstanding != truncated:
        problems.append(f"{origin} outstanding {outstanding} != integer tenth {truncated}")
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
            resolved = (path.parent / target).resolve()
            if not resolved.exists():
                problems.append(f"missing link {path.relative_to(root)} -> {target}")
    return problems


def thai_hits(root):
    hits = []
    for path in root.rglob("*"):
        if not path.is_file() or ".git" in path.parts:
            continue
        if path.suffix not in {".md", ".py", ".txt"}:
            continue
        data = path.read_text(encoding="utf-8", errors="replace")
        for number, line in enumerate(data.splitlines(), 1):
            if any("\u0e00" <= char <= "\u0e7f" for char in line):
                hits.append(f"thai {path.relative_to(root)}:{number}")
    return hits


def check_spec(spec):
    problems = []
    rows = requirement_rows(spec)
    ids = [row[0] for row in rows]
    if len(ids) != len(set(ids)):
        problems.append("duplicate requirement ids")
    keys = source_keys(spec)
    for row in rows:
        if len(row) != 8:
            problems.append(f"{row[0]} has {len(row)} cells")
            continue
        ident, _requirement, kind, source, test, result, gate, block = row
        if kind not in {"official", "derived", "assumption"}:
            problems.append(f"{ident} kind {kind}")
        if result != "not-run":
            problems.append(f"{ident} result {result}")
        if gate not in {"sim", "hardware", "unspecified"}:
            problems.append(f"{ident} gate {gate}")
        if block not in {"open", "none"}:
            problems.append(f"{ident} block {block}")
        for marker in ("Pass:", "Fail:", "Boundary:"):
            if marker not in test:
                problems.append(f"{ident} missing {marker}")
        if block == "open" and "cannot pass" not in test:
            problems.append(f"{ident} open without cannot pass")
        for token in re.findall(r"\bS\d+[A-Z]*\b", source):
            if token not in keys:
                problems.append(f"{ident} undefined source key {token}")
    for ident in REQUIRED_OPEN:
        if ident not in ids:
            problems.append(f"missing {ident}")
    for phrase in REQUIRED_PHRASES:
        if phrase not in spec:
            problems.append(f"missing phrase {phrase}")
    if "every condition passed" in spec.lower():
        problems.append("unsupported completeness claim")
    excerpt_rows = score_rows(page_body(EXCERPT_2025))
    written_rows = spec_score_rows(spec)
    if written_rows != excerpt_rows:
        problems.append("printed score table does not match the 2025.1 excerpt")
        problems.append(f"excerpt {excerpt_rows}")
        problems.append(f"spec {written_rows}")
    problems.extend(check_math(excerpt_rows, "2025.1"))
    problems.extend(check_math(score_rows(page_body(EXCERPT_2024)), "2024.2"))
    if "penalty of 250 points" not in EXCERPT_GENERAL.read_text(encoding="utf-8"):
        problems.append("general excerpt lost the 250 point sentence")
    if "1115" not in page_body(EXCERPT_2025):
        problems.append("2025 excerpt lost 1115")
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
    expected = hash_sources()
    if not SUMS.exists():
        return ["SHA256SUMS missing"]
    actual = SUMS.read_text(encoding="utf-8")
    if actual != expected:
        return ["SHA256SUMS does not match artifacts/sources"]
    return []


def unexpected_code(root):
    names = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")
    found = [name for name in names if (root / name).exists()]
    if (root / ".github").exists():
        found.append(".github")
    if found:
        return ["unexpected code or CI: " + ", ".join(found)]
    return []


def self_test():
    failures = []
    good = score_rows(page_body(EXCERPT_2025))
    if check_math(good, "self"):
        failures.append("2025 math should pass")
    broken = list(good)
    broken[-1] = (broken[-1][0], broken[-1][1] + 1)
    if not check_math(broken, "self"):
        failures.append("tampered total should fail")
    bad_row = "| CML-X | text | official | S25 p.1 | Pass: a. Fail: b. Boundary: c. | pass | sim | none |"
    parsed = requirement_rows(bad_row + "\n")
    if not parsed or parsed[0][5] == "not-run":
        failures.append("pass result should be visible to the checker")
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
    problems = []
    problems.extend(check_spec(SPEC.read_text(encoding="utf-8")))
    problems.extend(check_sums())
    problems.extend(relative_links(ROOT))
    problems.extend(thai_hits(ROOT))
    problems.extend(unexpected_code(ROOT))
    if problems:
        print("\n".join(problems))
        return 1
    print(f"ok rows={len(requirement_rows(SPEC.read_text(encoding='utf-8')))}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
