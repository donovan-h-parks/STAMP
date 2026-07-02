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
