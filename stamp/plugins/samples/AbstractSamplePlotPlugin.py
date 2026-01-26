# =======================================================================
# Author: Donovan Parks
#
# Abstract base class specifying interface of a sample plot plugin.
#
# Copyright 2011 Donovan Parks
#
# This file is part of STAMP.
#
# STAMP is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# STAMP is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.  If not, see <http://www.gnu.org/licenses/>.
# =======================================================================

import time

from PyQt5 import QtGui, QtCore, QtWidgets

from matplotlib.backends.backend_qt5agg import FigureCanvasQTAgg as FigureCanvas
from matplotlib.backends.backend_qt5agg import NavigationToolbar2QT as NavigationToolbar
from matplotlib.figure import Figure
import matplotlib.transforms as mtransforms

import numpy as np


class AbstractSamplePlotPlugin(QtWidgets.QWidget):
    '''
    Abstract base class specifying interface of a sample plot plugin.
    Modified to include a NavigationToolbar for zooming/panning.
    '''

    def __init__(self, preferences, parent=None):
        super(AbstractSamplePlotPlugin, self).__init__(parent)
        self.preferences = preferences

        self.type = '<none>'
        self.name = '<none>'

        # Create Figure and Canvas
        self.fig = Figure(facecolor='white', dpi=96)
        self.canvas = FigureCanvas(self.fig)

        # Create Toolbar
        self.toolbar = NavigationToolbar(self.canvas, self)

        # Layout: Toolbar on top, Canvas below
        layout = QtWidgets.QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)
        layout.addWidget(self.toolbar)
        layout.addWidget(self.canvas)

        # Policy
        self.setSizePolicy(QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Expanding)
        self.canvas.setSizePolicy(QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Expanding)
        self.canvas.updateGeometry()

        self.cid = None

    def mouseEventCallback(self, callback):
        if self.cid is not None:
            self.canvas.mpl_disconnect(self.cid)

        self.cid = self.canvas.mpl_connect('button_press_event', callback)

    # Proxy methods to maintain compatibility with code expecting a FigureCanvas
    def draw(self):
        self.canvas.draw()

    def updateGeometry(self):
        self.canvas.updateGeometry()
        super(AbstractSamplePlotPlugin, self).updateGeometry()

    def get_renderer(self):
        return self.canvas.get_renderer()

    def plot(self, profile, statsResults):
        pass

    def configure(self, profile, statsResults):
        pass

    def savePlot(self, filename, dpi=300):
        # Python 3 friendly substring check
        format = filename.split('.')[-1]
        if format in ['png', 'pdf', 'ps', 'eps', 'svg']:
            self.fig.savefig(filename, format=format, dpi=dpi, facecolor='white', edgecolor='white')
        else:
            pass

    def clear(self):
        self.fig.clear()

    def mirrorProperties(self, plotToCopy):
        pass

    def labelExtents(self, xLabels, xFontSize, xRotation, yLabels, yFontSize, yRotation):
        self.fig.clear()

        tempAxes = self.fig.add_axes([0, 0, 1.0, 1.0])

        tempAxes.set_xticks(np.arange(len(xLabels)))
        tempAxes.set_yticks(np.arange(len(yLabels)))

        xText = tempAxes.set_xticklabels(xLabels, size=xFontSize, rotation=xRotation)
        yText = tempAxes.set_yticklabels(yLabels, size=yFontSize, rotation=yRotation)

        bboxes = []
        for label in xText:
            bbox = label.get_window_extent(self.get_renderer())
            bboxi = bbox.inverse_transformed(self.fig.transFigure)
            bboxes.append(bboxi)
        if bboxes:
            xLabelBounds = mtransforms.Bbox.union(bboxes)
        else:
            xLabelBounds = mtransforms.Bbox.from_bounds(0, 0, 0, 0)

        bboxes = []
        for label in yText:
            bbox = label.get_window_extent(self.get_renderer())
            bboxi = bbox.inverse_transformed(self.fig.transFigure)
            bboxes.append(bboxi)
        if bboxes:
            yLabelBounds = mtransforms.Bbox.union(bboxes)
        else:
            yLabelBounds = mtransforms.Bbox.from_bounds(0, 0, 0, 0)

        self.fig.clear()

        return xLabelBounds, yLabelBounds

    def xLabelExtents(self, labels, fontSize, rotation=0):
        self.fig.clear()

        tempAxes = self.fig.add_axes([0, 0, 1.0, 1.0])
        tempAxes.set_xticks(np.arange(len(labels)))
        xLabels = tempAxes.set_xticklabels(labels, size=fontSize, rotation=rotation)

        bboxes = []
        for label in xLabels:
            bbox = label.get_window_extent(self.get_renderer())
            bboxi = bbox.inverse_transformed(self.fig.transFigure)
            bboxes.append(bboxi)
        if bboxes:
            xLabelBounds = mtransforms.Bbox.union(bboxes)
        else:
            xLabelBounds = mtransforms.Bbox.from_bounds(0, 0, 0, 0)

        self.fig.clear()

        return xLabelBounds

    def yLabelExtents(self, labels, fontSize, rotation=0):
        self.fig.clear()

        tempAxes = self.fig.add_axes([0, 0, 1.0, 1.0])
        tempAxes.set_yticks(np.arange(len(labels)))
        yLabels = tempAxes.set_yticklabels(labels, size=fontSize, rotation=rotation)

        bboxes = []
        for label in yLabels:
            bbox = label.get_window_extent(self.get_renderer())
            bboxi = bbox.inverse_transformed(self.fig.transFigure)
            bboxes.append(bboxi)
        if bboxes:
            yLabelBounds = mtransforms.Bbox.union(bboxes)
        else:
            yLabelBounds = mtransforms.Bbox.from_bounds(0, 0, 0, 0)

        self.fig.clear()

        return yLabelBounds

    def emptyAxis(self):
        self.fig.clear()
        self.fig.set_size_inches(6, 4)
        emptyAxis = self.fig.add_axes([0.1, 0.1, 0.8, 0.8])

        emptyAxis.set_ylabel('No active features or degenerate plot', fontsize=8)
        emptyAxis.set_xlabel('No active features or degenerate plot', fontsize=8)
        emptyAxis.set_yticks([])
        emptyAxis.set_xticks([])

        for loc, spine in emptyAxis.spines.items():
            if loc in ['right', 'top']:
                spine.set_color('none')

        self.updateGeometry()
        self.draw()

    def formatLabels(self, labels):
        formattedLabels = []
        for label in labels:
            try:
                value = float(label.get_text())
            except ValueError:
                formattedLabels.append(label.get_text())
                continue

            if value < 0.01 and value > 0:
                valueStr = '%.2e' % value
                if 'e-00' in valueStr:
                    valueStr = valueStr.replace('e-00', 'e-')
                elif 'e-0' in valueStr:
                    valueStr = valueStr.replace('e-0', 'e-')
            else:
                valueStr = '%.3f' % value

            formattedLabels.append(valueStr)

        return formattedLabels


class ConfigureDialog(QtWidgets.QDialog):
    def __init__(self, configDialogUI, parent=None):
        super(ConfigureDialog, self).__init__(parent)

        # initialize GUI
        self.ui = configDialogUI()
        self.ui.setupUi(self)

        self.centerWindow()

    def centerWindow(self):
        screen = QtWidgets.QDesktopWidget().screenGeometry()
        size = self.geometry()
        # Use integer division
        self.move((screen.width() - size.width()) // 2, (screen.height() - size.height()) // 2)


class TestWindow(QtWidgets.QMainWindow):
    '''
    Simple Qt window for testing plots.
    '''

    def __init__(self, PlotClass, statsResults=''):
        '''
        Create window with plot.
        PlotClass - a class object (not instance) inherited from AbstractPlotPlugin
        '''
        super(TestWindow, self).__init__()

        self.setAttribute(QtCore.Qt.WA_DeleteOnClose)
        self.setWindowTitle("Test Window")

        self.main_widget = QtWidgets.QWidget(self)

        layout = QtWidgets.QVBoxLayout(self.main_widget)

        # Create a dummy preferences dict if running in isolation
        dummy_prefs = {
            'Axes colour': QtGui.QColor('black'),
            'Sample 1 colour': QtGui.QColor('blue'),
            'Sample 2 colour': QtGui.QColor('red'),
            'Group colours': {},
            'Truncate feature names': False,
            'Settings': QtCore.QSettings('STAMP_Test', 'Test')
        }

        # Instantiate with preferences
        try:
            testPlot = PlotClass(dummy_prefs, parent=self.main_widget)
        except TypeError:
            # Fallback if class expects different signature
            testPlot = PlotClass(self.main_widget)

        if statsResults != '':
            testPlot.plot(statsResults)
        else:
            testPlot.emptyAxis()
        layout.addWidget(testPlot)

        self.main_widget.setFocus()
        self.setCentralWidget(self.main_widget)