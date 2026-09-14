#!/usr/bin/env bash
#
# One-command setup for the STAMP Python 3.13 / PyQt6 build.
# Creates a virtualenv, installs STAMP + all dependencies, and runs the headless
# regression smoke-test to prove the install works.
#
# Usage:
#     ./setup_py313.sh
#
# See PYTHON313_SETUP.md for the full handoff (system packages, running the GUI, status).

set -euo pipefail
cd "$(dirname "$0")"

VENV=".venv313"

# --- pick a Python 3.13 interpreter -----------------------------------------
PY=""
for cand in python3.13 python3 python; do
    if command -v "$cand" >/dev/null 2>&1; then
        ver="$("$cand" -c 'import sys; print("%d.%d" % sys.version_info[:2])' 2>/dev/null || echo 0.0)"
        major="${ver%%.*}"; minor="${ver##*.}"
        if [ "$major" = "3" ] && [ "$minor" -ge 9 ]; then PY="$cand"; break; fi
    fi
done
if [ -z "$PY" ]; then
    echo "ERROR: need Python >= 3.9 (3.13 recommended). Install it, then re-run." >&2
    echo "  Debian/Ubuntu: sudo apt-get install python3.13 python3.13-venv python3.13-dev" >&2
    echo "  macOS:         brew install python@3.13" >&2
    exit 1
fi
echo ">> Using $("$PY" --version) ($PY)"

# --- warn about the Qt runtime libs on Linux --------------------------------
if [ "$(uname)" = "Linux" ]; then
    echo ">> Linux detected. If the GUI later fails with a Qt 'xcb'/platform-plugin error,"
    echo "   install the shared libs it needs:"
    echo "     sudo apt-get install -y libgl1 libegl1 libxkbcommon0 libdbus-1-3 libfontconfig1 libfreetype6 xvfb"
fi

# --- create venv + install --------------------------------------------------
echo ">> Creating virtualenv at $VENV"
"$PY" -m venv "$VENV"
"$VENV/bin/python" -m pip install --upgrade pip
echo ">> Installing STAMP (editable) + dependencies"
"$VENV/bin/python" -m pip install -e .

# --- verify -----------------------------------------------------------------
echo ">> Byte-compiling the tree"
"$VENV/bin/python" -m compileall -q stamp

echo ">> Running headless regression smoke-test"
QT_QPA_PLATFORM=offscreen "$VENV/bin/python" tools/py313_smoketest.py

cat <<EOF

============================================================
 Setup complete.

 Launch the GUI:
     $VENV/bin/python -m stamp
   (headless VM with no display:)
     xvfb-run -a $VENV/bin/python -m stamp

 Re-run the regression test any time:
     QT_QPA_PLATFORM=offscreen $VENV/bin/python tools/py313_smoketest.py

 Full handoff / status: see PYTHON313_SETUP.md
============================================================
EOF
