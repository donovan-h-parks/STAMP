#!/usr/bin/env python3
"""Mechanical Python 2 -> 3 transforms for the STAMP codebase.

Scope: syntax/idiom fixes that are independent of the Qt port.
Deliberately conservative and line-oriented so every change is reviewable
in `git diff`. Qt-specific transforms live in migrate_qt.py.
"""
import io
import os
import re
import sys

# Exact rewrites for the ~18 py2 implicit-relative imports (module -> package path).
RELATIVE_IMPORT_FIXES = {
    "from mainUI import": "from stamp.mainUI import",
    "from createProfileRITA_UI import": "from stamp.GUI.createProfileRITA_UI import",
    "from metadataTableDlgUI import": "from stamp.GUI.metadataTableDlgUI import",
    "from preferencesUI import": "from stamp.GUI.preferencesUI import",
    "from createProfileMothurUI import": "from stamp.GUI.createProfileMothurUI import",
    "from assignCOGUI import": "from stamp.GUI.assignCOGUI import",
    "from multCompCorrectionInfoUI import": "from stamp.GUI.multCompCorrectionInfoUI import",
    "from groupLegendDlgUI import": "from stamp.GUI.groupLegendDlgUI import",
    "from loadDataDlgUI import": "from stamp.GUI.loadDataDlgUI import",
    "from plotDlgUI import": "from stamp.GUI.plotDlgUI import",
    "from createProfileCoMetUI import": "from stamp.GUI.createProfileCoMetUI import",
    "from selectFeaturesUI import": "from stamp.GUI.selectFeaturesUI import",
    "from createProfileBiomUI import": "from stamp.GUI.createProfileBiomUI import",
    "from createProfileMgRastUI import": "from stamp.GUI.createProfileMgRastUI import",
    "from customizeHeadingsUI import": "from stamp.GUI.customizeHeadingsUI import",
    "from statsTableDlgUI import": "from stamp.GUI.statsTableDlgUI import",
}

PRINT_RE = re.compile(r"^(\s*)print(\s+)(\S.*?)(\s*)$")
PRINT_BARE_RE = re.compile(r"^(\s*)print\s*$")
MAP_STRIP_RE = re.compile(r"map\(string\.strip,\s*([\w.]+\([^()]*\)|[\w.]+)\)")


def transform_line(line):
    # Shebangs -> portable python3
    if line.startswith("#!") and "python" in line and "python3" not in line:
        return "#!/usr/bin/env python3\n"

    # Skip comment-only lines for statement transforms
    stripped = line.lstrip()

    # print statements -> print() (single-line only; codebase has no `print >>` or trailing-comma prints)
    m = PRINT_RE.match(line.rstrip("\n"))
    if m and not stripped.startswith("#"):
        indent, _, expr, _ = m.groups()
        # Don't touch already-functional print(...) that happens to have a space: `print (x)` -> fine to wrap
        line = f"{indent}print({expr})\n"
    else:
        mb = PRINT_BARE_RE.match(line.rstrip("\n"))
        if mb and not stripped.startswith("#"):
            line = f"{mb.group(1)}print()\n"

    # xrange -> range
    line = re.sub(r"\bxrange\b", "range", line)

    # dict iterator methods -> py3 views
    line = line.replace(".iteritems(", ".items(")
    line = line.replace(".iterkeys(", ".keys(")
    line = line.replace(".itervalues(", ".values(")

    # map(string.strip, X) -> materialized list comprehension (map is lazy in py3; results are reused)
    line = MAP_STRIP_RE.sub(r"[__s.strip() for __s in \1]", line)

    # string.lower used as a sort key -> str.lower
    line = line.replace("key=string.lower", "key=str.lower")

    # implicit relative imports -> absolute package imports
    for old, new in RELATIVE_IMPORT_FIXES.items():
        if old in line:
            line = line.replace(old, new)

    return line


def process(path):
    with io.open(path, "r", encoding="utf-8", errors="surrogateescape") as f:
        src = f.readlines()
    out = [transform_line(l) for l in src]
    new = "".join(out)
    if new != "".join(src):
        with io.open(path, "w", encoding="utf-8", errors="surrogateescape") as f:
            f.write(new)
        return True
    return False


def main(root):
    changed = 0
    for dirpath, _, filenames in os.walk(root):
        if "/.git" in dirpath:
            continue
        for fn in filenames:
            if fn.endswith(".py") and fn != "migrate_py2to3.py" and fn != "migrate_qt.py":
                if process(os.path.join(dirpath, fn)):
                    changed += 1
    print(f"files changed: {changed}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else ".")
