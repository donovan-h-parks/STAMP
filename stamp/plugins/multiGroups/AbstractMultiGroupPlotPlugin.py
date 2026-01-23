# =======================================================================
# Author: Donovan Parks
#
# Abstract base class specifying interface of a multiple group plot plugin.
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
import numpy as np

from PyQt5 import QtGui, QtCore, QtWidgets

from matplotlib.backends.backend_qt5agg import FigureCanvasQTAgg as FigureCanvas
from matplotlib.backends.backend_qt5agg import NavigationToolbar2QT as NavigationToolbar
from matplotlib.figure import Figure
import matplotlib.transforms as mtransforms


class AbstractMultiGroupPlotPlugin(FigureCanvas):
    '''
    Abstract base class specifying interface of a multiple group plot plugin.
    '''

    def __init__(self, preferences, parent=None):
        self.preferences = preferences

        self.fig = Figure(facecolor='white', dpi=96)

        super(AbstractMultiGroupPlotPlugin, self).__init__(self.fig)

        self.setParent(parent)
        self.setSizePolicy(QtWidgets.QSizePolicy.Fixed, QtWidgets.QSizePolicy.Fixed)
        self.updateGeometry()

        self.cid = None

        self.type = '<none>'
        self.name = '<none>'

        # Default flags to prevent AttributeErrors in STAMP.py
        self.bSupportsHighlight = False
        self.bPlotFeaturesIndividually = True
        self.bRunPostHocTest = False

    def checkFlags(self):
        """
        Returns self to allow STAMP.py to check flags like bRunPostHocTest.
        """
        return self

    def mouseEventCallback(self, callback):
        if self.cid is not None:
            self.mpl_disconnect(self.cid)

        self.cid = self.mpl_connect('button_press_event', callback)

    def plot(self, profile, statsResults):
        pass

    def configure(self, profile, statsResults):
        pass

    def savePlot(self, filename, dpi=300):
        # Python 3 safe string split
        format = filename.split('.')[-1]
        if format in ['png', 'pdf', 'ps', 'eps', 'svg']:
            self.fig.savefig(filename, format=format, dpi=dpi, facecolor='white', edgecolor='white')
        else:
            pass

    def clear(self):
        self.fig.clear()

    def mirrorProperties(self, plotToCopy):
        self.type = plotToCopy.type
        self.name = plotToCopy.name
        self.bSupportsHighlight = plotToCopy.bSupportsHighlight

        # Mirror new flags if they exist
        if hasattr(plotToCopy, 'bRunPostHocTest'):
            self.bRunPostHocTest = plotToCopy.bRunPostHocTest
        if hasattr(plotToCopy, 'bPlotFeaturesIndividually'):
            self.bPlotFeaturesIndividually = plotToCopy.bPlotFeaturesIndividually

    def labelExtents(self, xLabels, xFontSize, xRotation, yLabels, yFontSize, yRotation):
        self.fig.clear()

        tempAxes = self.fig.add_axes([0, 0, 1.0, 1.0])

        tempAxes.set_xticks(np.arange(len(xLabels)))
        tempAxes.set_yticks(np.arange(len(yLabels)))

        xText = tempAxes.set_xticklabels(xLabels, size=xFontSize, rotation=xRotation)
        yText = tempAxes.set_yticklabels(yLabels, size=yFontSize, rotation=yRotation)

        # FIX: inverse_transformed -> transformed(inverted())
        bboxes = []
        for label in xText:
            bbox = label.get_window_extent(self.get_renderer())
            bboxi = bbox.transformed(self.fig.transFigure.inverted())
            bboxes.append(bboxi)
        xLabelBounds = mtransforms.Bbox.union(bboxes)

        bboxes = []
        for label in yText:
            bbox = label.get_window_extent(self.get_renderer())
            bboxi = bbox.transformed(self.fig.transFigure.inverted())
            bboxes.append(bboxi)
        yLabelBounds = mtransforms.Bbox.union(bboxes)

        self.fig.clear()

        return xLabelBounds, yLabelBounds

    def xLabelExtents(self, labels, fontSize, rotation=0):
        self.fig.clear()

        tempAxes = self.fig.add_axes([0, 0, 1.0, 1.0])
        tempAxes.set_xticks(np.arange(len(labels)))
        xLabels = tempAxes.set_xticklabels(labels, size=fontSize, rotation=rotation)

        # FIX: inverse_transformed -> transformed(inverted())
        bboxes = []
        for label in xLabels:
            bbox = label.get_window_extent(self.get_renderer())
            bboxi = bbox.transformed(self.fig.transFigure.inverted())
            bboxes.append(bboxi)
        xLabelBounds = mtransforms.Bbox.union(bboxes)

        self.fig.clear()

        return xLabelBounds

    def yLabelExtents(self, labels, fontSize, rotation=0):
        self.fig.clear()

        tempAxes = self.fig.add_axes([0, 0, 1.0, 1.0])
        tempAxes.set_yticks(np.arange(len(labels)))
        yLabels = tempAxes.set_yticklabels(labels, size=fontSize, rotation=rotation)

        # FIX: inverse_transformed -> transformed(inverted())
        bboxes = []
        for label in yLabels:
            bbox = label.get_window_extent(self.get_renderer())
            bboxi = bbox.transformed(self.fig.transFigure.inverted())
            bboxes.append(bboxi)
        yLabelBounds = mtransforms.Bbox.union(bboxes)

        self.fig.clear()

        return yLabelBounds

    def emptyAxis(self, title=''):
        self.fig.clear()
        self.fig.set_size_inches(6, 4)
        emptyAxis = self.fig.add_axes([0.1, 0.1, 0.8, 0.8])

        emptyAxis.set_ylabel('No active features or degenerate plot', fontsize=8)
        emptyAxis.set_xlabel('No active features or degenerate plot', fontsize=8)
        emptyAxis.set_yticks([])
        emptyAxis.set_xticks([])
        emptyAxis.set_title(title)

        # Python 3 dictionary iteration
        for loc, spine in emptyAxis.spines.items():
            if loc in ['right', 'top']:
                spine.set_color('none')

        self.updateGeometry()
        self.draw()

    def formatLabels(self, labels):
        formattedLabels = []
        for label in labels:
            value = float(label.get_text())
            if value < 0.01:
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
        # Python 3 integer division
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

        # PyQt5 Layouts are in QtWidgets
        layout = QtWidgets.QVBoxLayout(self.main_widget)
        testPlot = PlotClass(self.main_widget)

        if statsResults != '':
            testPlot.plot(statsResults)
        else:
            testPlot.emptyAxis()
        layout.addWidget(testPlot)

        self.main_widget.setFocus()
        self.setCentralWidget(self.main_widget)