"""Make the legacy STAMP package importable without PyQt4 or matplotlib.

The statistics code only touches Qt through two table helpers and one
message box, so a stub module is enough to run it headlessly.
"""
import os
import sys
import types

LEGACY_DIR = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
REPO_DIR = os.path.normpath(os.path.join(LEGACY_DIR, '..'))


class _AnythingMeta(type):
    def __getattr__(cls, name):
        if name.startswith('__'):
            raise AttributeError(name)
        return _Anything()


class _Anything(object):
    """Stands in for any Qt class: subclassable, callable, attribute-tolerant."""
    __metaclass__ = _AnythingMeta
    UserType = 1000

    def __init__(self, *args, **kwargs):
        pass

    def __getattr__(self, name):
        return _Anything()

    def __call__(self, *args, **kwargs):
        return _Anything()


class _QtModule(types.ModuleType):
    def __getattr__(self, name):
        if name.startswith('__'):
            raise AttributeError(name)
        return _Anything


def setup():
    pyqt = types.ModuleType('PyQt4')
    pyqt.QtGui = _QtModule('PyQt4.QtGui')
    pyqt.QtCore = _QtModule('PyQt4.QtCore')
    sys.modules['PyQt4'] = pyqt
    sys.modules['PyQt4.QtGui'] = pyqt.QtGui
    sys.modules['PyQt4.QtCore'] = pyqt.QtCore
    if LEGACY_DIR not in sys.path:
        sys.path.insert(0, LEGACY_DIR)
