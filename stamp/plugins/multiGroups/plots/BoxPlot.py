# =======================================================================
# Author: Donovan Parks
#
# Box plot for multiple groups.
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

import sys

from PyQt5 import QtGui, QtCore, QtWidgets

from stamp.plugins.multiGroups.AbstractMultiGroupPlotPlugin import AbstractMultiGroupPlotPlugin, TestWindow, \
    ConfigureDialog
from stamp.plugins.multiGroups.plots.configGUI.BoxPlotUI import Ui_BoxPlotDialog

from stamp.plugins.PlotEventHandler import PlotEventHandler

from matplotlib.patches import Polygon
from matplotlib.lines import Line2D
from matplotlib.ticker import NullFormatter
from matplotlib import collections

from matplotlib.artist import setp

import numpy as np


class BoxPlot(AbstractMultiGroupPlotPlugin):
    '''
    Box plot for multiple groups.
    '''

    def __init__(self, preferences, parent=None):
        super(BoxPlot, self).__init__(preferences, parent)
        self.preferences = preferences

        self.name = 'Box plot'
        self.type = 'Exploratory'

        self.bPlotFeaturesIndividually = True

        # --- Flags required by STAMP.py MultiGroup logic ---
        self.bRunPostHocTest = False
        self.bSupportsHighlight = False
        # ---------------------------------------------------

        self.settings = preferences['Settings']
        self.figWidth = float(self.settings.value('multiple group: ' + self.name + '/width', 7.0))
        self.figHeight = float(self.settings.value('multiple group: ' + self.name + '/height', 7.0))
        self.fieldToPlot = str(
            self.settings.value('multiple group: ' + self.name + '/field to plot', 'Proportion of sequences (%)'))
        self.bShowAverages = self.settings.value('multiple group: ' + self.name + '/show averages', True, type=bool)
        self.bShowPvalue = self.settings.value('multiple group: ' + self.name + '/show p-value', True, type=bool)

    def mirrorProperties(self, plotToCopy):
        super(BoxPlot, self).mirrorProperties(plotToCopy)

        self.figWidth = plotToCopy.figWidth
        self.figHeight = plotToCopy.figHeight

        self.fieldToPlot = plotToCopy.fieldToPlot
        self.bShowAverages = plotToCopy.bShowAverages

        self.bShowPvalue = plotToCopy.bShowPvalue

        if hasattr(plotToCopy, 'bSupportsHighlight'):
            self.bSupportsHighlight = plotToCopy.bSupportsHighlight
        if hasattr(plotToCopy, 'bRunPostHocTest'):
            self.bRunPostHocTest = plotToCopy.bRunPostHocTest

    def plot(self, profile, statsResults):
        if len(profile.profileDict) <= 0 or self.preferences['Selected multiple group feature'] == '':
            self.emptyAxis()
            return

        axesColour = str(self.preferences['Axes colour'].name())

        # *** Get data for each group
        feature = self.preferences['Selected multiple group feature']

        # Use robust data fetching (list of groups -> list of samples -> list of counts)
        if self.fieldToPlot == "Number of sequences":
            raw_data = profile.getActiveFeatureFromActiveSamplesCounts([feature])
        else:  # Proportion of sequences (%)
            raw_data = profile.getActiveFeatureFromActiveSamplesProportions([feature])

        # Flatten the data structure for boxplot:
        # boxplot expects a list of vectors (one vector per group)
        data = []
        for group_data in raw_data:
            # group_data is [ [val1], [val2], ... ] because we requested [feature]
            group_vals = [x[0] for x in group_data]
            data.append(group_vals)

        if not data or all(len(d) == 0 for d in data):
            self.emptyAxis()
            return

        # --- FIX: Synchronize data and labels to avoid mismatch error ---
        num_groups = min(len(data), len(profile.activeGroupNames))
        data = data[:num_groups]
        xLabels = profile.activeGroupNames[:num_groups]
        # ---------------------------------------------------------------

        # *** Set figure size
        self.fig.clear()
        self.fig.set_size_inches(self.figWidth, self.figHeight)

        padding = 0.25  # inches
        xOffsetFigSpace = (0.4 + padding) / self.figWidth
        yOffsetFigSpace = (0.3 + padding) / self.figHeight
        axesBoxPlot = self.fig.add_axes([xOffsetFigSpace, yOffsetFigSpace,
                                         1.0 - xOffsetFigSpace - (2 * padding) / self.figWidth,
                                         1.0 - yOffsetFigSpace - (2 * padding) / self.figHeight])

        # box plot
        bp = axesBoxPlot.boxplot(data, notch=0, sym='k+', vert=1, whis=1.5)
        setp(bp['boxes'], color='black')
        setp(bp['whiskers'], color='black', linestyle='-')
        setp(bp['medians'], color='black')

        # fill boxes with desired colors
        colours = []
        for groupName in xLabels:
            colours.append(str(self.preferences['Group colours'][groupName].name()))

        for i in range(0, len(data)):
            # get box coordinates
            box = bp['boxes'][i]
            # Python 3 Fix: zip returns iterator, Polygon needs list/sequence
            boxCoords = list(zip(box.get_xdata()[0:5], box.get_ydata()[0:5]))

            # colour in box
            boxPolygon = Polygon(boxCoords, facecolor=colours[i])
            axesBoxPlot.add_patch(boxPolygon)

            # draw the median lines back over what we just filled in
            med = bp['medians'][i]
            axesBoxPlot.plot(med.get_xdata()[0:2], med.get_ydata()[0:2], 'k')

        # mark average
        if self.bShowAverages:
            for i in range(0, len(data)):
                med = bp['medians'][i]
                if len(data[i]) > 0:
                    axesBoxPlot.plot([np.average(med.get_xdata())], [np.average(data[i])], color='w', marker='*',
                                     markeredgecolor='k')

        # *** P-value label
        if self.bShowPvalue and statsResults.profile is not None:
            pValueStr = statsResults.getFeatureStatisticAsStr(feature, 'pValuesCorrected')
            if 'e' in pValueStr:
                # Simple latex formatting for sci notation
                pValueStr = pValueStr.replace('e', r'\times 10^{') + '}'
            axesBoxPlot.text(1.0, 1.0, r'$p$ = ' + pValueStr, horizontalalignment='right', verticalalignment='bottom',
                             transform=axesBoxPlot.transAxes)

        # *** Prettify scatter plot
        display_feature = feature
        if self.preferences['Truncate feature names']:
            length = self.preferences['Length of truncated feature names']
            if len(feature) > length + 3:
                display_feature = feature[0:length] + '...'

        axesBoxPlot.set_title(display_feature)
        axesBoxPlot.set_ylabel(self.fieldToPlot)

        # --- FIX: Set ticks explicitly matching data length ---
        axesBoxPlot.set_xticks(range(1, len(data) + 1))
        axesBoxPlot.set_xticklabels(xLabels)
        # -----------------------------------------------------

        for a in axesBoxPlot.yaxis.majorTicks:
            a.tick1On = True
            a.tick2On = False

        for a in axesBoxPlot.xaxis.majorTicks:
            a.tick1On = True
            a.tick2On = False

        for line in axesBoxPlot.yaxis.get_ticklines():
            line.set_color(axesColour)

        for line in axesBoxPlot.xaxis.get_ticklines():
            line.set_color(axesColour)

        for loc, spine in axesBoxPlot.spines.items():
            if loc in ['right', 'top']:
                spine.set_color('none')
            else:
                spine.set_color(axesColour)

        self.updateGeometry()
        self.draw()

    def configure(self, profile, statsResults):
        configDlg = ConfigureDialog(Ui_BoxPlotDialog)

        configDlg.ui.cboFieldToPlot.setCurrentIndex(configDlg.ui.cboFieldToPlot.findText(self.fieldToPlot))

        configDlg.ui.spinFigWidth.setValue(self.figWidth)
        configDlg.ui.spinFigHeight.setValue(self.figHeight)

        configDlg.ui.chkShowAverage.setChecked(self.bShowAverages)

        configDlg.ui.chkShowPvalue.setChecked(self.bShowPvalue)

        if configDlg.exec_() == QtWidgets.QDialog.Accepted:
            self.fieldToPlot = str(configDlg.ui.cboFieldToPlot.currentText())

            self.figWidth = configDlg.ui.spinFigWidth.value()
            self.figHeight = configDlg.ui.spinFigHeight.value()

            self.bShowAverages = configDlg.ui.chkShowAverage.isChecked()

            self.bShowPvalue = configDlg.ui.chkShowPvalue.isChecked()

            self.settings.setValue('multiple group: ' + self.name + '/column width', self.figWidth)
            self.settings.setValue('multiple group: ' + self.name + '/height', self.figHeight)
            self.settings.setValue('multiple group: ' + self.name + '/field to plot', self.fieldToPlot)
            self.settings.setValue('multiple group: ' + self.name + '/show averages', self.bShowAverages)
            self.settings.setValue('multiple group: ' + self.name + '/show p-value', self.bShowPvalue)

            self.plot(profile, statsResults)


if __name__ == "__main__":
    app = QtWidgets.QApplication(sys.argv)
    testWindow = TestWindow(BoxPlot)
    testWindow.show()
    sys.exit(app.exec_())