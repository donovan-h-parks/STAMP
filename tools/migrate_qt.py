#!/usr/bin/env python3
"""Mechanical, compile-verifiable PyQt4 -> PyQt6 transforms.

Deliberately EXCLUDES the correctness-critical work that needs a running
PyQt6 to verify: old-style SIGNAL/SLOT signal connections, scoped-enum
migration, QDesktopWidget removal, and .qrc resource handling. Those are
tracked separately in QT_MIGRATION_TODO.md.
"""
import io
import os
import re
import sys

# QtGui classes in Qt4 that live in QtWidgets in Qt6.
TO_WIDGETS = {
    "QApplication", "QSizePolicy", "QFormLayout", "QLabel", "QHBoxLayout",
    "QVBoxLayout", "QComboBox", "QMessageBox", "QFrame", "QCheckBox",
    "QSpacerItem", "QRadioButton", "QDoubleSpinBox", "QGroupBox", "QPushButton",
    "QDialogButtonBox", "QWidget", "QDialog", "QSpinBox", "QLineEdit",
    "QFileDialog", "QGridLayout", "QDockWidget", "QAbstractItemView",
    "QScrollArea", "QMainWindow", "QColorDialog", "QToolButton", "QMenu",
    "QTableWidgetItem", "QListWidget", "QLayout", "QTableView",
    "QProgressDialog", "QAbstractSpinBox", "QTabWidget", "QSplitter",
    "QTextEdit", "QTableWidget", "QStatusBar", "QStackedWidget", "QMenuBar",
    "QInputDialog",
}
# QtGui classes in Qt4 that live in QtCore in Qt6.
TO_CORE = {"QItemSelectionModel"}
# QtGui classes that REMAIN in QtGui in Qt6 (incl. QAction, moved back from QtWidgets).
STAY_GUI = {"QIcon", "QPixmap", "QPalette", "QColor", "QCursor", "QBrush",
            "QFont", "QAction"}


def fix_imports(line):
    # Normalise the four PyQt4 import shapes and always make QtWidgets available.
    if re.match(r"^\s*from PyQt4 import ", line):
        return re.sub(r"from PyQt4 import .*",
                      "from PyQt6 import QtCore, QtGui, QtWidgets", line.rstrip("\n")) + "\n"
    line = line.replace("from PyQt4.", "from PyQt6.")
    line = line.replace("import PyQt4", "import PyQt6")
    # matplotlib Qt backend: qt4agg -> qtagg (module name change in mpl 3.5+)
    line = line.replace("backend_qt4agg", "backend_qtagg")
    line = line.replace("backend_qt4", "backend_qtagg")
    line = line.replace("NavigationToolbar2QTAgg", "NavigationToolbar2QT")
    return line


def fix_namespaces(line):
    # QtGui.<Widget> -> QtWidgets.<Widget>
    def repl(m):
        name = m.group(1)
        if name in TO_WIDGETS:
            return "QtWidgets." + name
        if name in TO_CORE:
            return "QtCore." + name
        return "QtGui." + name  # STAY_GUI and anything else untouched
    line = re.sub(r"QtGui\.([A-Za-z_][A-Za-z0-9_]*)", repl, line)
    # exec_ -> exec (removed in PyQt6)
    line = re.sub(r"\.exec_\(", ".exec(", line)
    return line


def process(path):
    with io.open(path, "r", encoding="utf-8", errors="surrogateescape") as f:
        src = f.readlines()
    out = []
    for line in src:
        line = fix_imports(line)
        line = fix_namespaces(line)
        out.append(line)
    new = "".join(out)
    if new != "".join(src):
        with io.open(path, "w", encoding="utf-8", errors="surrogateescape") as f:
            f.write(new)
        return True
    return False


def main(root):
    changed = 0
    for dirpath, _, filenames in os.walk(root):
        if "/.git" in dirpath or "/tools" in dirpath:
            continue
        for fn in filenames:
            if fn.endswith(".py"):
                if process(os.path.join(dirpath, fn)):
                    changed += 1
    print(f"files changed: {changed}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else ".")
