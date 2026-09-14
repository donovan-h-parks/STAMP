#!/usr/bin/env python3
"""Apply the PyQt6 resource fix to an existing STAMP checkout.

Run once from your project root (the folder that contains the `stamp/` package):

    python fix_resources.py

It does two things:
  1. Overwrites stamp/STAMP_rc.py with a disk-loading shim (PyQt6 has no .qrc
     compiler, and the old PyQt4 compiled blob is incompatible).
  2. Rewrites resource-scheme icon paths ":/icons/icons/<name>.png" to the Qt
     search-path scheme "icons:<name>.png" across the stamp/ package.

Safe to run more than once (idempotent).
"""
import io
import os
import sys

SHIM = '''\
#=======================================================================
# PyQt6 replacement for the old pyrcc-generated resource module.
#
# PyQt6 removed the .qrc resource compiler (pyrcc), and the PyQt4-vintage
# compiled blob is incompatible with PyQt6's qRegisterResourceData. Instead
# of a compiled resource, we register a Qt search path so that the existing
# "icons:<name>.png" references resolve to the packaged stamp/icons/ folder.
#=======================================================================

import os
from PyQt6 import QtCore

_ICON_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "icons")
QtCore.QDir.addSearchPath("icons", _ICON_DIR)


def qInitResources():
    # Retained as a no-op so legacy `qInitResources()` calls still work.
    pass


def qCleanupResources():
    pass
'''


def main():
    if not os.path.isdir("stamp"):
        sys.exit("error: run this from the folder that contains the 'stamp/' package")

    # 1. Replace the resource module.
    with io.open(os.path.join("stamp", "STAMP_rc.py"), "w", encoding="utf-8") as f:
        f.write(SHIM)
    print("wrote stamp/STAMP_rc.py (disk-loading shim)")

    # 2. Rewrite icon paths (only the resource scheme; never touches http:// URLs).
    changed = 0
    for dirpath, _, filenames in os.walk("stamp"):
        for fn in filenames:
            if not fn.endswith(".py") or fn == "STAMP_rc.py":
                continue
            p = os.path.join(dirpath, fn)
            with io.open(p, "r", encoding="utf-8", errors="surrogateescape") as f:
                src = f.read()
            if ":/icons/icons/" in src:
                with io.open(p, "w", encoding="utf-8", errors="surrogateescape") as f:
                    f.write(src.replace(":/icons/icons/", "icons:"))
                changed += 1
    print(f"rewrote icon paths in {changed} files")
    print("done.")


if __name__ == "__main__":
    main()