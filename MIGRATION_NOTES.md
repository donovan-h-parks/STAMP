# STAMP → Python 3.12 / PyQt6 migration notes

Baseline: STAMP 2.1.3 (Python 2, PyQt4), as uploaded.
Target: Python 3.9–3.12, numpy 2.x / scipy / matplotlib 3.x, PyQt6, biom-format 2.1.x.

Every change is a separate git commit, so `git diff dad28d2 HEAD` shows the full migration
and `git show <stage>` shows each stage in isolation.

## Done and verified in this environment

**Stage 0 — build system** (`7685951`)
- Replaced the distutils `setup.py` (distutils was removed from the stdlib in Python 3.12)
  with a `pyproject.toml` (setuptools backend) plus a thin `setup.py` shim.
- Dependencies modernised; **`six` removed** (unused) and **`pyqi` removed** (archived,
  Python-2-only; it was referenced only by the old build/packaging scripts, never by the
  STAMP runtime).
- Verified: `pyproject.toml` parses; setuptools discovers all 34 packages; entry point
  `STAMP = stamp.STAMP:main` resolves.

**Stage 1 — Python 2 → 3 syntax** (`3afb1a2`), 99 files
- `print` statements → `print()`; `xrange` → `range`; `.iteritems/.iterkeys/.itervalues`
  → `.items/.keys/.values`; `map(string.strip, …)` → list comprehensions (map is lazy in
  py3 and the results are reused); `string.lower` sort key → `str.lower`.
- 18 implicit relative imports → absolute package imports (e.g. `from mainUI import …`
  → `from stamp.mainUI import …`).
- 9 `cmp(…)` uses → `key=` functions (including py2's `sorted(seq, cmp, key=…)` in
  `TableHelper.SortTable`, rewritten as pure key functions that preserve ordering exactly).
- Fixed invalid escape sequences (`"Show\hide"`) behaviour-preservingly.
- Verified: **the entire tree byte-compiles under Python 3.12**, and the non-GUI numeric /
  file-IO layer (`stamp.metagenomics.stats.*`, `fileIO.StampIO`, `fileIO.MetadataIO`)
  imports cleanly against installed numpy 2.4 / scipy 1.17 / matplotlib 3.10.

**Stage 2 — mechanical PyQt4 → PyQt6** (`fc7d128`), 77 files
- All imports → `from PyQt6 import QtCore, QtGui, QtWidgets`.
- `QtGui.<Widget>` → `QtWidgets.<Widget>` for the 42 widget/layout/dialog classes actually
  used; classes that stay in QtGui (`QIcon`, `QPixmap`, `QColor`, `QCursor`, `QBrush`,
  `QFont`, `QPalette`, and **`QAction`** — moved back to QtGui in Qt6) left untouched;
  `QItemSelectionModel` → `QtCore`.
- `.exec_()` → `.exec()` (54 sites); matplotlib `backend_qt4agg` → `backend_qtagg`.
- `QtCore.QStringList()` → `[]`; `QVariant` model returns → plain objects / `None`;
  `QDesktopWidget().screenGeometry()` → `QApplication.primaryScreen().geometry()` (14 sites;
  QDesktopWidget was removed in Qt6).
- Verified: **whole tree still byte-compiles**; no live PyQt4 references remain (only the
  auto-generated "created by PyQt4 UI code generator" comment banners).

> Note: "byte-compiles" verifies syntax, not GUI runtime. PyQt6 could not be installed in
> this sandbox (no network), so the GUI has not been *run*. The remaining stages below need
> a real PyQt6 install to verify.

## Remaining work (stages 3–5) — needs a running PyQt6

### Stage 3 — old-style signals → new-style (247 sites, 40 files)
`self.connect(obj, SIGNAL('sig'), slot)` → `obj.<signal>.connect(slot)` and
`self.emit(SIGNAL('sig'), args)` → `self.<signal>.emit(args)`.

Standard signals (mechanical once the map is fixed):

| PyQt4 SIGNAL string                | PyQt6 new-style attribute |
|------------------------------------|---------------------------|
| `clicked()`, `clicked(bool)`       | `.clicked`                |
| `toggled(bool)`                    | `.toggled`                |
| `triggered()`                      | `.triggered`              |
| `editingFinished()`                | `.editingFinished`        |
| `accepted()` / `rejected()`        | `.accepted` / `.rejected` |
| `currentIndexChanged(int)`         | `.currentIndexChanged`    |
| `currentChanged(int)` (QTabWidget) | `.currentChanged`         |
| `itemClicked(QTableWidgetItem*)`   | `.itemClicked`            |
| `topLevelChanged(bool)`            | `.topLevelChanged`        |
| `dockLocationChanged(...)`         | `.dockLocationChanged`    |
| `layoutChanged()` / `layoutAboutToBeChanged()` | `.layoutChanged` / `.layoutAboutToBeChanged` |

**Traps that must be handled by hand (Qt6 overload removals):**
- `activated(QString)` (39 sites, QComboBox) → **`.textActivated`**, *not* `.activated`
  (in Qt6 `activated` carries only `int`).
- `currentIndexChanged(QString)` → **`.currentTextChanged`**.

**Custom signals** — must be declared as `pyqtSignal` class attributes and emitted new-style:
- `GroupLegendDlg`: `legendItemChanged`, `legendFieldChanged`, `legendActiveGroupsChanged`
- `MetadataTableDlg`: `activeSamplesChanged`

### Stage 4 — scoped enums (Qt6 requires fully-qualified enums)
Top-level `QtCore.Qt.*` members in use and their Qt6 enum classes:

| members | Qt6 scope |
|---|---|
| AlignCenter, AlignHCenter, AlignVCenter, AlignLeft, AlignRight, AlignLeading, AlignTrailing | `Qt.AlignmentFlag` |
| AllDockWidgetAreas, Bottom/Left/Right/TopDockWidgetArea, DockWidgetArea | `Qt.DockWidgetArea` |
| ApplicationModal, NonModal, WindowModal | `Qt.WindowModality` |
| ArrowCursor, WaitCursor | `Qt.CursorShape` |
| AscendingOrder, DescendingOrder | `Qt.SortOrder` |
| Checked, Unchecked | `Qt.CheckState` |
| DisplayRole | `Qt.ItemDataRole` |
| Horizontal | `Qt.Orientation` |
| LeftToRight, RightToLeft | `Qt.LayoutDirection` |
| SolidPattern | `Qt.BrushStyle` |
| StrongFocus, WheelFocus | `Qt.FocusPolicy` |
| WA_DeleteOnClose | `Qt.WidgetAttribute` |

Widget-class enums in the generated `*UI.py` files need the same treatment, e.g.
`QSizePolicy.Expanding` → `QSizePolicy.Policy.Expanding`, `QFrame.HLine/Sunken` →
`QFrame.Shape.HLine` / `QFrame.Shadow.Sunken`, `QDialogButtonBox.Ok` →
`QDialogButtonBox.StandardButton.Ok`, `QMessageBox.Yes/Warning` →
`QMessageBox.StandardButton.Yes` / `QMessageBox.Icon.Warning`, `QIcon.Normal/Off` →
`QIcon.Mode.Normal` / `QIcon.State.Off`.

### Stage 5 — Qt resources (`.qrc`) and final smoke test
- The generated UI files reference compiled Qt resources (`import resources` / icon paths
  like `:/icons/...`). PyQt6 dropped the `pyrcc` resource compiler, so icons should either be
  loaded from disk via `QtCore.QDir`/direct paths, or resources compiled with an alternative
  (e.g. the `rcc` from PySide6 tooling). Simplest path: load icons from the packaged
  `stamp/icons/` directory instead of the `:/` resource scheme.
- The `data()`/`headerData()` role checks in `GenericTable.py` use `Qt.DisplayRole` etc.,
  covered by stage 4.
- Recommended verification: `pip install pyqt6 biom-format`, then launch with the bundled
  `examples/` datasets and exercise each plugin family (samples / groups / multiGroups → the
  statistical tests, effect-size filters, and each plot type).

## Stage 6 — runtime GUI bug fixes (verified on Python 3.13 / PyQt6 6.x / numpy 2.5 / scipy 1.18 / matplotlib 3.10 / biom 2.1.17)

The tree byte-compiled but crashed at runtime once data was loaded and analyses/plots
were exercised. Fixed the following classes of bug (each verified by loading
`examples/EnterotypesArumugam/` and running every statistical test, post-hoc test and plot
type across sample / group / multi-group modes — 46/46 pass — plus BIOM import):

1. **`dict.keys()/.values()` used as a list** (py3 returns a lazy view, not a list):
   - Getters returning bare views that callers index: `Metadata.getSampleNames/getFeatures`,
     `SampleProfile/GroupProfile/MultiGroupProfile.getFeatures` → wrapped in `list(...)`.
   - Subscripts `d.keys()[0]` in `Metadata.getFeatures`, `MultiGroupProfile.getFeatureMatrix`,
     and `featuresToPlot = profile.profileDict.keys()` in both `HeatmapPlot`s.

2. **`zip()` used as a list** (py3 returns a single-use iterator): `ProfileBarPlots`,
   `MultCompCorrectionPlots` (`.sort()` on a zip), `groups/multiGroups BoxPlot` (passed to
   `Polygon`), and `PlotEventHandler`/`MultiPlotEventHandler` (stored zip consumed on first
   mouse-click, killing plot tooltips after one click).

3. **`except Exception as note:` then using `note` after the block** — py3 deletes the
   exception name at end of the `except` clause. `Ttest.run` returned an unbound `note`.

4. **Removed numpy/scipy aliases**: `scipy.array` → `numpy.asarray`,
   `scipy.random.random_integers` → `numpy.random.randint(0, n, n)` (Bootstrap.py),
   `scipy.arange` → `numpy.arange` (StoreyFDR.py).

5. **Removed matplotlib APIs**: `Bbox.inverse_transformed(T)` →
   `Bbox.transformed(T.inverted())` (12 sites across the three `Abstract*PlotPlugin`s);
   `pylab.cm.spectral` → `pylab.cm.nipy_spectral` (both HeatmapPlots).

6. **PyQt6 signal disconnect**: `signal.disconnect(slot)` raises `TypeError` when not
   connected (PyQt4 returned `False`). Guarded the first-call disconnect in
   `metadataTableDlg.setTable`.

7. **biom-format 2.1 API**: `table.sample_ids` → `table.ids(axis='sample')`
   (createProfileBiomDlg).

8. **Robustness**: `PlotsManager.display` now guards a `None` widget from `takeWidget()`, so
   one failing plot no longer poisons every subsequent plot switch.

9. **Signal shadowed by a same-named slot method** (`plotDlg.py`): the new-style port produced
   `self.topLevelChanged.connect(self.topLevelChanged)`, but `PlotDlg(QDockWidget)` defines
   methods `topLevelChanged`/`dockLocationChanged` that shadow the inherited `QDockWidget`
   signals of the same name, so `self.topLevelChanged` resolved to the *method* →
   `'function' object has no attribute 'connect'`. This crashed **View → Send plot to new
   window** for every plot. Renamed the handlers to `onTopLevelChanged`/`onDockLocationChanged`.

### Button / menu / slot coverage
Every one of the 75 connected slots on the main window (buttons, menu actions, toggles,
combo-box `textActivated`, colour pickers, table-selection handlers) was invoked headlessly
with the modal dialogs stubbed: **74/75 run clean** (the 75th is `close`, intentionally not
auto-invoked). Bug #9 was the only failure and is fixed. One pre-existing (non-migration)
latent edge case remains: `groupTableFeatureChanged`/`multiGroupSideTableFeatureChanged` do
`selected.indexes()[0]`, which would `IndexError` if `selectionChanged` ever fires with an
empty selection (e.g. clicking below the last row). Same code existed in the PyQt4 original;
left as-is unless we choose to add a guard.

## A note on numerical results
Python 3 changed `/` from floor to true division for ints (131 division sites in the stats
code). Most operate on numpy float arrays and are unaffected, but this is the one class of
change that can shift results silently. Running `STAMP_test.py` against the pre-migration
outputs is the intended safety net once the GUI-independent tests can execute.
