#!/usr/bin/env python3
"""
Headless regression smoke-test for the STAMP Python 3.13 / PyQt6 migration.

Why this exists
---------------
STAMP *launches* fine after the mechanical py2->3 / PyQt4->PyQt6 migration, but the
interesting runtime bugs only fire when data is loaded and an analysis or plot is run.
This harness builds the real MainWindow, loads a bundled example dataset (bypassing the
file-open dialog), then drives every statistical test, post-hoc test, and plot type across
the sample / group / multi-group modes, reporting OK/FAIL per action instead of dying on the
first crash. It runs with the Qt "offscreen" platform, so it needs no display.

Usage
-----
    QT_QPA_PLATFORM=offscreen .venv313/bin/python tools/py313_smoketest.py
    # optional: point at a different profile + metadata pair
    QT_QPA_PLATFORM=offscreen .venv313/bin/python tools/py313_smoketest.py \
        examples/FinegoldAutism/Autism.spf examples/FinegoldAutism/Austism.metadata.tsv

Exit code is non-zero if any action fails, so it can gate CI.
"""

import os
import sys
import traceback

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

from numpy import seterr
seterr(all="ignore")

from PyQt6 import QtGui, QtWidgets

import stamp.STAMP as S
from stamp.metagenomics.fileIO.StampIO import StampIO
from stamp.metagenomics.fileIO.MetadataIO import MetadataIO

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

DEFAULT_PROFILE = os.path.join(REPO_ROOT, "examples/EnterotypesArumugam/Enterotypes.profile.spf")
DEFAULT_METADATA = os.path.join(REPO_ROOT, "examples/EnterotypesArumugam/Enterotypes.metadata.tsv")

PROFILE = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_PROFILE
METADATA = sys.argv[2] if len(sys.argv) > 2 else DEFAULT_METADATA

failures = []


def default_preferences():
    prefs = {}
    prefs["Pseudocount"] = 0.5
    prefs["Replicates"] = 1000
    prefs["Truncate feature names"] = True
    prefs["Length of truncated feature names"] = 50
    prefs["Axes colour"] = QtGui.QColor("#7f7f7f")
    prefs["All other samples colour"] = QtGui.QColor("#7f7f7f")
    prefs["Minimum reported p-value exponent"] = -15
    prefs["Sample 1 colour"] = QtGui.QColor(128, 177, 211)
    prefs["Sample 2 colour"] = QtGui.QColor(253, 180, 98)
    prefs["Group colours"] = {}
    prefs["Highlighted sample features"] = []
    prefs["Highlighted group features"] = []
    prefs["Highlighted multiple group features"] = []
    prefs["Selected group feature"] = ""
    prefs["Selected multiple group feature"] = ""
    prefs["Last directory"] = ""
    return prefs


def step(name, fn):
    """Run one action, record failures, keep going."""
    try:
        fn()
        print(f"OK   : {name}")
    except Exception as e:
        failures.append(name)
        print(f"FAIL : {name}: {type(e).__name__}: {e}")
        traceback.print_exc()


def iterate(cbo, action, label):
    """Select each real (non-separator) entry of a combo box and run `action`."""
    for i in range(cbo.count()):
        cbo.setCurrentIndex(i)
        name = cbo.currentText()
        if name == "":            # skip inserted separators, which have empty text
            continue
        try:
            action()
            print(f"   {label} OK: {name}")
        except Exception as e:
            failures.append(f"{label}: {name}")
            print(f"   {label} FAIL: {name}: {type(e).__name__}: {e}")
            traceback.print_exc()


def load_profile(w, prefs):
    """Replicates STAMP.MainWindow.loadProfile without the file-open dialog."""
    stampIO = StampIO(prefs)
    w.profileTree, errMsg = stampIO.read(PROFILE)
    assert errMsg is None, errMsg

    metadataIO = MetadataIO(prefs)
    w.metadata, _warn = metadataIO.read(METADATA, w.profileTree)

    w.populateSampleComboBoxes()

    w.ui.cboParentalLevel.clear()
    w.ui.cboParentalLevel.addItem("Entire sample")
    for header in w.profileTree.hierarchyHeadings[0:-1]:
        w.ui.cboParentalLevel.addItem(header)
    w.ui.cboParentalLevel.setCurrentIndex(0)

    w.ui.cboProfileLevel.clear()
    for header in w.profileTree.hierarchyHeadings:
        w.ui.cboProfileLevel.addItem(header)
    w.ui.cboProfileLevel.setCurrentIndex(0)

    if w.metadata is not None and len(w.metadata.getFeatures()) != 0:
        w.groupLegendDlg.initLegend(w.profileTree, w.metadata, w.metadata.getFeatures()[0])
        prefs["Group colours"] = w.groupLegendDlg.groupColourDict

    w.populateGroupComboBoxes()
    w.multiGroupHierarchicalLevelsChanged()
    w.groupHierarchicalLevelsChanged()
    w.sampleHierarchicalLevelsChanged()
    w.groupFeaturesTableUpdate()
    w.multiGroupFeaturesTableUpdate()
    w.metadataDlg.setTable(w.metadata)


def main():
    app = QtWidgets.QApplication(sys.argv)  # noqa: F841 (kept alive for the widgets)
    prefs = default_preferences()
    os.chdir(S.getMainDir())

    w = S.MainWindow(prefs)
    print("MainWindow constructed")
    print(f"profile : {PROFILE}")
    print(f"metadata: {METADATA}")

    step("loadProfile", lambda: load_profile(w, prefs))

    w.bAutoRecalculate = True

    # SAMPLE mode
    step("all sample stat tests", lambda: iterate(w.ui.cboSampleStatTests, w.sampleRunTest, "sample test"))
    step("all sample plots", lambda: iterate(w.ui.cboSamplePlots, w.samplePlotUpdate, "sample plot"))
    # GROUP mode
    step("all group stat tests", lambda: iterate(w.ui.cboGroupStatTests, w.groupRunTest, "group test"))
    step("all group plots", lambda: iterate(w.ui.cboGroupPlots, w.groupPlotUpdate, "group plot"))
    # MULTI-GROUP mode
    step("all multi-group stat tests", lambda: iterate(w.ui.cboMultiGroupStatTests, w.multiGroupRunTest, "mg test"))
    step("all post-hoc tests", lambda: iterate(w.ui.cboPostHocTest, w.multiGroupPlotUpdate, "mg posthoc"))
    step("all multi-group plots", lambda: iterate(w.ui.cboMultiGroupPlots, w.multiGroupPlotUpdate, "mg plot"))

    print("=" * 60)
    if failures:
        print(f"FAILED ({len(failures)}):")
        for f in failures:
            print(f"  - {f}")
        sys.exit(1)
    print("ALL ACTIONS PASSED")


if __name__ == "__main__":
    main()
