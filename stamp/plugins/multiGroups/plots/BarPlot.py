# =======================================================================
# Author: Donovan Parks
#
# Bar plot for multiple groups.
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
import numpy as np

from stamp.plugins.multiGroups.AbstractMultiGroupPlotPlugin import AbstractMultiGroupPlotPlugin, TestWindow, \
    ConfigureDialog
from stamp.plugins.multiGroups.plots.configGUI.BarPlotUI import Ui_BarPlotDialog

from stamp.metagenomics.stats.CI.WilsonCI import WilsonCI


class BarPlot(AbstractMultiGroupPlotPlugin):
    '''
    Bar plot for multiple groups.
    '''

    def __init__(self, preferences, parent=None):
        super(BarPlot, self).__init__(preferences, parent)
        self.preferences = preferences

        self.name = 'Bar plot'
        self.type = 'Exploratory'

        self.bPlotFeaturesIndividually = True

        # --- Flags required by STAMP.py MultiGroup logic ---
        self.bRunPostHocTest = False
        self.bSupportsHighlight = False
        # ---------------------------------------------------

        self.settings = preferences['Settings']

        # Explicit type casting for PyQt5
        self.figColWidth = float(self.settings.value('multiple group: ' + self.name + '/column width', 3.0))
        self.figHeight = float(self.settings.value('multiple group: ' + self.name + '/height', 4.0))
        self.fieldToPlot = str(
            self.settings.value('multiple group: ' + self.name + '/field to plot', 'Proportion of sequences (%)'))
        self.bShowAverages = self.settings.value('multiple group: ' + self.name + '/show averages', True, type=bool)
        self.legendPos = int(self.settings.value('multiple group: ' + self.name + '/legend position', 0))
        self.bShowPvalue = self.settings.value('multiple group: ' + self.name + '/show p-value', True, type=bool)

    def mirrorProperties(self, plotToCopy):
        super(BarPlot, self).mirrorProperties(plotToCopy)

        self.name = plotToCopy.name
        self.figColWidth = plotToCopy.figColWidth
        self.figHeight = plotToCopy.figHeight
        self.fieldToPlot = plotToCopy.fieldToPlot
        self.bShowAverages = plotToCopy.bShowAverages
        self.legendPos = plotToCopy.legendPos
        self.bShowPvalue = plotToCopy.bShowPvalue

        if hasattr(plotToCopy, 'bSupportsHighlight'):
            self.bSupportsHighlight = plotToCopy.bSupportsHighlight
        if hasattr(plotToCopy, 'bRunPostHocTest'):
            self.bRunPostHocTest = plotToCopy.bRunPostHocTest

    def plot(self, profile, statsResults):
        if len(profile.profileDict) <= 0:
            self.emptyAxis()
            return

        # *** Colour of plot elements
        axesColour = str(self.preferences['Axes colour'].name())

        # *** Create plot
        self.fig.clear()

        # get data
        if not statsResults.selectedFeatures:
            self.emptyAxis()
            return

        feature = statsResults.selectedFeatures[0]

        vals = []
        cis = []

        # Retrieve counts AND proportions to ensure we have data even if counts/normalization fails
        # Returns: list of groups -> list of samples -> list of counts
        raw_feature_counts = profile.getActiveFeatureFromActiveSamplesCounts([feature])
        raw_feature_props = profile.getActiveFeatureFromActiveSamplesProportions([feature])

        # Determine valid range to avoid IndexError
        num_groups = min(len(profile.activeGroupNames), len(raw_feature_counts))

        # set plot dimensions based on valid groups
        figWidth = max(num_groups * self.figColWidth, 1.0)
        self.fig.set_size_inches(figWidth, self.figHeight)

        plotRect = [0.2 / figWidth, 0.1 / self.figHeight, 1.0 - 0.3 / figWidth, 1.0 - 0.3 / self.figHeight]
        axes = self.fig.add_axes(plotRect)

        if self.fieldToPlot == "Number of sequences":
            # Implementation for raw counts if needed
            for i in range(num_groups):
                group_samples_data = raw_feature_counts[i]
                k = sum([sample_counts[0] for sample_counts in group_samples_data])
                vals.append(k)
                cis.append(0)  # No CI for raw counts usually
        else:  # Proportion of sequences (%)
            # calculate proportions and CIs
            ciCalc = WilsonCI()

            # Safe loop using determined range
            for i in range(num_groups):
                groupName = profile.activeGroupNames[i]
                group_samples_counts = raw_feature_counts[i]
                group_samples_props = raw_feature_props[i]

                # --- Calculate k (count of feature in this group) ---
                k = sum([sample_counts[0] for sample_counts in group_samples_counts])

                # --- Calculate n (total sequences in this group) ---
                n = 0
                try:
                    if hasattr(profile, 'numSequencesInGroup'):
                        n = profile.numSequencesInGroup(groupName, profile.metadata)
                    else:
                        # Fallback: Sum parentCounts for active samples in this group
                        feature_entry = profile.profileDict[feature]

                        # Use sample list from profile to get indices
                        if i < len(profile.activeSamplesInGroups):
                            current_samples = profile.activeSamplesInGroups[i]

                            # Check if getSampleIndex exists
                            if hasattr(profile, 'getSampleIndex'):
                                for sample in current_samples:
                                    idx = profile.getSampleIndex(sample)
                                    n += feature_entry.parentCounts[idx]
                            elif hasattr(profile, 'sampleIndex'):
                                for sample in current_samples:
                                    if sample in profile.sampleIndex:
                                        idx = profile.sampleIndex[sample]
                                        n += feature_entry.parentCounts[idx]
                except Exception:
                    n = 0
                # ----------------------------------------------------

                if n > 0:
                    p = k * 100.0 / n
                    vals.append(p)

                    l, u = ciCalc.calc(k, n, 0.05)
                    cis.append(u * 100.0 - p)
                else:
                    # FALLBACK: If n calculation failed (n=0), use the average of the proportions
                    # This ensures the bar is visible even if we can't calc CIs
                    if group_samples_props:
                        # Average of [prop1, prop2, ...]
                        avg_prop = np.mean([p[0] for p in group_samples_props])
                        vals.append(avg_prop)
                    else:
                        vals.append(0.0)
                    cis.append(0.0)

        # plot bars
        ind = np.arange(len(vals))
        width = 0.35

        colours = []
        for i in range(num_groups):
            groupName = profile.activeGroupNames[i]
            colours.append(str(self.preferences['Group colours'][groupName].name()))

        if len(vals) > 0:
            rects = axes.bar(ind, vals, width, color=colours, yerr=cis, ecolor='k')

            # axis labels
            axes.set_ylabel(self.fieldToPlot)
            axes.set_xticks(ind)  # Center ticks

            labels = []
            for i in range(num_groups):
                name = profile.activeGroupNames[i]
                if len(name) > 12:
                    labels.append(name[:10] + '...')
                else:
                    labels.append(name)
            axes.set_xticklabels(labels)

            # show p-value
            if self.bShowPvalue and statsResults.profile is not None:
                pValue = statsResults.getFeatureStatisticAsStr(feature, 'pValuesCorrected')
                if 'e' in pValue:
                    pValStr = '$P = %s$' % (pValue.replace('e', '\\times 10^{') + '}')
                else:
                    pValStr = '$P = %s$' % (pValue)

                axes.text(1.02, 0.5, pValStr, horizontalalignment='left', verticalalignment='center',
                          transform=axes.transAxes)

        # *** Prettify plot
        for a in axes.yaxis.majorTicks:
            a.tick1On = True
            a.tick2On = False

        for a in axes.xaxis.majorTicks:
            a.tick1On = False
            a.tick2On = False

        for line in axes.yaxis.get_ticklines():
            line.set_color(axesColour)

        for loc, spine in axes.spines.items():
            if loc in ['right', 'top']:
                spine.set_color('none')
            else:
                spine.set_color(axesColour)

        self.updateGeometry()
        self.draw()

    def configure(self, profile, statsResults):
        configDlg = ConfigureDialog(Ui_BarPlotDialog)

        configDlg.ui.spinFigColWidth.setValue(self.figColWidth)
        configDlg.ui.spinFigHeight.setValue(self.figHeight)

        configDlg.ui.cboFieldToPlot.setCurrentIndex(configDlg.ui.cboFieldToPlot.findText(self.fieldToPlot))

        configDlg.ui.chkShowAverage.setChecked(self.bShowAverages)

        if self.legendPos == 0:
            configDlg.ui.radioLegendPosBest.setChecked(True)
        elif self.legendPos == 1:
            configDlg.ui.radioLegendPosUpperRight.setChecked(True)
        elif self.legendPos == 7:
            configDlg.ui.radioLegendPosCentreRight.setChecked(True)
        elif self.legendPos == 4:
            configDlg.ui.radioLegendPosLowerRight.setChecked(True)
        elif self.legendPos == 2:
            configDlg.ui.radioLegendPosUpperLeft.setChecked(True)
        elif self.legendPos == 6:
            configDlg.ui.radioLegendPosCentreLeft.setChecked(True)
        elif self.legendPos == 3:
            configDlg.ui.radioLegendPosLowerLeft.setChecked(True)
        else:
            configDlg.ui.radioLegendPosNone.setChecked(True)

        configDlg.ui.chkShowPvalue.setChecked(self.bShowPvalue)

        if configDlg.exec_() == QtWidgets.QDialog.Accepted:
            self.figColWidth = configDlg.ui.spinFigColWidth.value()
            self.figHeight = configDlg.ui.spinFigHeight.value()
            self.fieldToPlot = str(configDlg.ui.cboFieldToPlot.currentText())
            self.bShowAverages = configDlg.ui.chkShowAverage.isChecked()

            # legend position
            if configDlg.ui.radioLegendPosBest.isChecked():
                self.legendPos = 0
            elif configDlg.ui.radioLegendPosUpperRight.isChecked():
                self.legendPos = 1
            elif configDlg.ui.radioLegendPosCentreRight.isChecked():
                self.legendPos = 7
            elif configDlg.ui.radioLegendPosLowerRight.isChecked():
                self.legendPos = 4
            elif configDlg.ui.radioLegendPosUpperLeft.isChecked():
                self.legendPos = 2
            elif configDlg.ui.radioLegendPosCentreLeft.isChecked():
                self.legendPos = 6
            elif configDlg.ui.radioLegendPosLowerLeft.isChecked():
                self.legendPos = 3
            else:
                self.legendPos = -1

            self.bShowPvalue = configDlg.ui.chkShowPvalue.isChecked()

            self.settings.setValue('multiple group: ' + self.name + '/column width', self.figColWidth)
            self.settings.setValue('multiple group: ' + self.name + '/height', self.figHeight)
            self.settings.setValue('multiple group: ' + self.name + '/field to plot', self.fieldToPlot)
            self.settings.setValue('multiple group: ' + self.name + '/show averages', self.bShowAverages)
            self.settings.setValue('multiple group: ' + self.name + '/legend position', self.legendPos)
            self.settings.setValue('multiple group: ' + self.name + '/show p-value', self.bShowPvalue)

            self.plot(profile, statsResults)


if __name__ == "__main__":
    app = QtWidgets.QApplication(sys.argv)
    testWindow = TestWindow(BarPlot)
    testWindow.show()
    sys.exit(app.exec_())