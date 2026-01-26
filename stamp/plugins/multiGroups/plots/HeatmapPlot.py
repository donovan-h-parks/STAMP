# =======================================================================
# Author: Donovan Parks
#
# Heatmap for multiple groups.
#
# Copyright 2013 Donovan Parks
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
from stamp.plugins.multiGroups.plots.configGUI.HeatmapPlotUI import Ui_HeatmapPlotDialog

from stamp.plugins.PlotEventHandler import PlotEventHandler

import matplotlib.pyplot as plt
import matplotlib as mpl
from matplotlib.patches import Rectangle

import numpy
import scipy.cluster.hierarchy as cluster
import scipy.spatial.distance as dist


class HeatmapPlot(AbstractMultiGroupPlotPlugin):
    '''
    Heatmap plot.
    '''

    def __init__(self, preferences, parent=None):
        super(HeatmapPlot, self).__init__(preferences, parent)

        self.discreteColourMap = mpl.colors.ListedColormap([
            (141 / 255.0, 211 / 255.0, 199 / 255.0), (255 / 255.0, 255 / 255.0, 179 / 255.0),
            (190 / 255.0, 186 / 255.0, 218 / 255.0), (251 / 255.0, 128 / 255.0, 114 / 255.0),
            (128 / 255.0, 177 / 255.0, 211 / 255.0), (253 / 255.0, 180 / 255.0, 98 / 255.0),
            (179 / 255.0, 222 / 255.0, 105 / 255.0), (252 / 255.0, 205 / 255.0, 229 / 255.0),
            (217 / 255.0, 217 / 255.0, 217 / 255.0), (188 / 255.0, 128 / 255.0, 189 / 255.0),
            (204 / 255.0, 235 / 255.0, 197 / 255.0), (255 / 255.0, 237 / 255.0, 111 / 255.0)
        ])

        self.preferences = preferences

        self.name = 'Heatmap plot'
        self.type = 'Exploratory'

        self.bPlotFeaturesIndividually = False

        # --- Flags required by STAMP.py MultiGroup logic ---
        self.bRunPostHocTest = False
        self.bSupportsHighlight = False
        # --------------------------------------------------------

        self.settings = preferences['Settings']
        # Explicit type casting for PyQt5 QSettings
        self.fieldToPlot = str(
            self.settings.value('group: ' + self.name + '/field to plot', 'Proportion of sequences (%)'))
        self.bPlotOnlyActiveFeatures = self.settings.value('group: ' + self.name + '/plot only active features', False,
                                                           type=bool)
        self.figWidth = float(self.settings.value('group: ' + self.name + '/width', 7.0))
        self.figHeight = float(self.settings.value('group: ' + self.name + '/height', 7.0))
        self.sortColMethod = str(
            self.settings.value('group: ' + self.name + '/sort col method', 'Average neighbour (UPGMA)'))
        self.sortRowMethod = str(
            self.settings.value('group: ' + self.name + '/sort row method', 'Average neighbour (UPGMA)'))
        self.bShowColDendrogram = self.settings.value('group: ' + self.name + '/show col dendrogram', True, type=bool)
        self.bShowRowDendrogram = self.settings.value('group: ' + self.name + '/show row dendrogram', True, type=bool)
        self.colourmap = str(self.settings.value('group: ' + self.name + '/colourmap', 'Blues'))
        self.legendPos = int(self.settings.value('group: ' + self.name + '/legend position', 3))
        self.clusteringColThreshold = float(
            self.settings.value('group: ' + self.name + '/clustering col threshold', 0.75))
        self.clusteringRowThreshold = float(
            self.settings.value('group: ' + self.name + '/clustering row threshold', 0.75))
        self.dendrogramHeight = float(self.settings.value('group: ' + self.name + '/dendrogram col height', 1.5))
        self.dendrogramWidth = float(self.settings.value('group: ' + self.name + '/dendrogram row width', 1.5))

    def checkFlags(self):
        """Helper method to satisfy STAMP.py calls like self.multiGroupPlot.checkFlags()."""
        return self

    def mirrorProperties(self, plotToCopy):
        super(HeatmapPlot, self).mirrorProperties(plotToCopy)

        self.bPlotFeaturesIndividually = False

        self.name = plotToCopy.name

        self.fieldToPlot = plotToCopy.fieldToPlot
        self.bPlotOnlyActiveFeatures = plotToCopy.bPlotOnlyActiveFeatures

        self.figWidth = plotToCopy.figWidth
        self.figHeight = plotToCopy.figHeight

        self.sortColMethod = plotToCopy.sortColMethod
        self.sortRowMethod = plotToCopy.sortRowMethod
        self.bShowColDendrogram = plotToCopy.bShowColDendrogram
        self.bShowRowDendrogram = plotToCopy.bShowRowDendrogram
        self.colourmap = plotToCopy.colourmap
        self.legendPos = plotToCopy.legendPos

        self.clusteringColThreshold = plotToCopy.clusteringColThreshold
        self.clusteringRowThreshold = plotToCopy.clusteringRowThreshold

        self.dendrogramHeight = plotToCopy.dendrogramHeight
        self.dendrogramWidth = plotToCopy.dendrogramWidth

        # Mirror flags if present
        if hasattr(plotToCopy, 'bRunPostHocTest'):
            self.bRunPostHocTest = plotToCopy.bRunPostHocTest
        if hasattr(plotToCopy, 'bSupportsHighlight'):
            self.bSupportsHighlight = plotToCopy.bSupportsHighlight

    def plotDendrogram(self, matrix, axis, dendrogramMethod, clusteringThreshold, orientation, bPlot):
        d = dist.pdist(matrix)

        # FIX: Pass 'd' directly to linkage, do NOT use dist.squareform(d)
        if dendrogramMethod == 'Average neighbour (UPGMA)':
            linkage = cluster.linkage(d, method='average')
        elif dendrogramMethod == 'Centroid':
            linkage = cluster.linkage(d, method='centroid')
        elif dendrogramMethod == 'Nearest neighbour':
            linkage = cluster.linkage(d, method='single')
        elif dendrogramMethod == 'Furthest neighbour':
            linkage = cluster.linkage(d, method='complete')
        elif dendrogramMethod == 'Ward':
            linkage = cluster.linkage(d, method='ward')

        dendrogram = cluster.dendrogram(linkage, orientation=orientation, link_color_func=lambda k: 'k', ax=axis,
                                        no_plot=not bPlot)
        index = cluster.fcluster(linkage, clusteringThreshold * max(linkage[:, 2]), 'distance')
        axis.set_xticks([])
        axis.set_yticks([])

        return index, dendrogram['leaves']

    def plot(self, profile, statsResults):

        # Python 3: keys() returns a view, convert to list
        featuresToPlot = list(profile.profileDict.keys())
        if self.bPlotOnlyActiveFeatures:
            featuresToPlot = statsResults.activeFeatures

        if len(featuresToPlot) <= 1 or len(profile.activeGroupNames) <= 1:
            self.emptyAxis()
            return
        elif len(featuresToPlot) > 1000 or len(profile.activeSamplesInGroups) > 1000:
            QtWidgets.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.ArrowCursor))
            QtWidgets.QMessageBox.information(self, 'Too much data!',
                                              'Heatmap plots are limited to 1000 samples and 1000 features.',
                                              QtWidgets.QMessageBox.Ok)
            QtWidgets.QApplication.instance().restoreOverrideCursor()
            self.emptyAxis()
            return

        # *** Colour of plot elements
        groupColours = {}
        for groupName in profile.activeGroupNames:
            groupColours[groupName] = str(self.preferences['Group colours'][groupName].name())

        # *** Colour map for category dendrogram on left
        if self.colourmap == "Blues":
            self.matrixColourmap = plt.cm.Blues
        elif self.colourmap == "Blue to red to green":
            self.matrixColourmap = plt.cm.brg
        elif self.colourmap == "Blue to white to red":
            self.matrixColourmap = plt.cm.bwr
        elif self.colourmap == "Cool to warm":
            self.matrixColourmap = plt.cm.cool
        elif self.colourmap == "Grayscale":
            self.matrixColourmap = plt.cm.gist_yarg
        elif self.colourmap == "Jet":
            self.matrixColourmap = plt.cm.jet
        elif self.colourmap == "Orange to red":
            self.matrixColourmap = plt.cm.OrRd
        elif self.colourmap == "Paired":
            self.matrixColourmap = plt.cm.Paired
        elif self.colourmap == "Purple to green":
            self.matrixColourmap = plt.cm.PRGn
        elif self.colourmap == "Reds":
            self.matrixColourmap = plt.cm.Reds
        elif self.colourmap == "Red to blue":
            self.matrixColourmap = plt.cm.RdBu
        elif self.colourmap == "Red to yellow to blue":
            self.matrixColourmap = plt.cm.RdYlBu
        elif self.colourmap == "Spectral":
            self.matrixColourmap = plt.cm.spectral
        elif self.colourmap == "Yellow to orange to red":
            self.matrixColourmap = plt.cm.YlOrRd
        else:
            self.matrixColourmap = plt.cm.Blues

        # *** Get data for each group
        if self.fieldToPlot == "Number of sequences":
            data = profile.getActiveFeatureFromActiveSamplesCounts(featuresToPlot)
        else:  # Proportion of sequences (%)
            data = profile.getActiveFeatureFromActiveSamplesProportions(featuresToPlot)

        matrix = []
        for r in range(0, len(data)):
            row = []
            for d in data[r]:
                row += d
            matrix.append(row)

        matrix = numpy.array(matrix)

        # *** Get heatmap data
        colHeaders = []
        for sampleNames in profile.activeSamplesInGroups:
            colHeaders += sampleNames
        rowHeaders = featuresToPlot

        # *** Find longest label
        bTruncate = False
        if self.preferences['Truncate feature names']:
            length = self.preferences['Length of truncated feature names']
            bTruncate = True

        longestLabelLen = 0
        longestRowLabel = ''
        for i in range(0, len(rowHeaders)):
            if bTruncate and len(rowHeaders[i]) > length + 3:
                rowHeaders[i] = rowHeaders[i][0:length] + '...'

            if len(rowHeaders[i]) > longestLabelLen:
                longestLabelLen = len(rowHeaders[i])
                longestRowLabel = rowHeaders[i]

        longestLabelLen = 0
        longestColLabel = ''
        for i in range(0, len(colHeaders)):
            if bTruncate and len(colHeaders[i]) > length + 3:
                colHeaders[i] = colHeaders[i][0:length] + '...'

            if len(colHeaders[i]) > longestLabelLen:
                longestLabelLen = len(colHeaders[i])
                longestColLabel = colHeaders[i]

        # *** Check sorting method and adjust dendrogram parameters appropriately
        if self.sortRowMethod == 'Alphabetical order' or self.sortRowMethod == 'Mean abundance':
            self.bShowRowDendrogram = False

        if self.sortColMethod == 'Alphabetical order' or self.sortColMethod == 'Mean abundance':
            self.bShowColDendrogram = False

        # *** Set figure size
        self.fig.clear()
        self.fig.set_size_inches(self.figWidth, self.figHeight)

        xLabelBounds, yLabelBounds = self.labelExtents([longestColLabel], 8, 90, [longestRowLabel], 8, 0)

        # position all figure elements
        colourBarWidthX = 0.2 / self.figWidth
        colourBarWidthY = 0.2 / self.figHeight
        marginX = 0.1 / self.figWidth
        marginY = 0.1 / self.figHeight

        if self.bShowRowDendrogram:
            dendrogramWidth = self.dendrogramWidth / self.figWidth
        else:
            dendrogramWidth = 0.2 / self.figWidth

        if self.bShowColDendrogram:
            dendrogramHeight = self.dendrogramHeight / self.figHeight
        else:
            dendrogramHeight = 0.2 / self.figHeight

        cellSizeX = max((1.0 - 2 * 0.02 - dendrogramWidth - colourBarWidthX - 2 * marginX - yLabelBounds.width),
                        0.01) * self.figWidth / len(colHeaders)
        cellSizeY = max((1.0 - 2 * 0.02 - dendrogramHeight - colourBarWidthY - 2 * marginY - xLabelBounds.height),
                        0.01) * self.figHeight / len(rowHeaders)
        cellSize = min(cellSizeX, cellSizeY)

        cellSizeXPer = cellSize / self.figWidth
        cellSizeYPer = cellSize / self.figHeight

        paddingX = 0.5 * (1.0 - dendrogramWidth - 2 * marginX - colourBarWidthX - cellSizeXPer * len(
            colHeaders) - yLabelBounds.width)
        paddingY = 0.5 * (1.0 - dendrogramHeight - 2 * marginY - colourBarWidthY - cellSizeYPer * len(
            rowHeaders) - xLabelBounds.height)

        rowDendrogramX = paddingX
        rowDendrogramY = paddingY + (xLabelBounds.height)
        rowDendrogramW = dendrogramWidth
        rowDendrogramH = cellSizeYPer * len(rowHeaders)

        rowClusterBarX = rowDendrogramX + rowDendrogramW + marginX
        rowClusterBarY = rowDendrogramY
        rowClusterBarW = colourBarWidthX
        rowClusterBarH = rowDendrogramH

        colDendrogramX = rowClusterBarX + rowClusterBarW + marginX
        colDendrogramY = rowDendrogramY + rowDendrogramH + marginY + colourBarWidthY + marginY
        colDendrogramW = cellSizeXPer * len(colHeaders)
        colDendrogramH = dendrogramHeight

        colClusterBarX = colDendrogramX
        colClusterBarY = rowDendrogramY + rowDendrogramH + marginY
        colClusterBarW = colDendrogramW
        colClusterBarH = colourBarWidthY

        heatmapX = rowClusterBarX + rowClusterBarW + marginX
        heatmapY = rowDendrogramY
        heatmapW = colDendrogramW
        heatmapH = rowDendrogramH

        legendHeight = 0.2 / self.figHeight
        legendW = min(0.8 * yLabelBounds.width, 1.25 / self.figWidth)
        legendH = legendHeight
        legendX = heatmapX + heatmapW + 0.2 / self.figWidth
        legendY = 1.0 - legendHeight - (2 * yLabelBounds.height) - marginY
        if not self.bShowColDendrogram:
            # move legend to side
            legendX = heatmapX + 0.5 * (heatmapW - legendW)
            legendY = heatmapY + heatmapH + (1.5 * yLabelBounds.height) + 0.1 / self.figWidth

        # plot dendrograms
        # FIX: Orientation 'left' puts root on left and leaves pointing to the right (towards the heatmap)
        if self.sortRowMethod == 'Alphabetical order':
            leafIndex1 = numpy.argsort(rowHeaders)[::-1]
        elif self.sortRowMethod == 'Mean abundance':
            leafIndex1 = numpy.argsort(numpy.mean(matrix, axis=1))
        else:
            axisRowDendrogram = self.fig.add_axes([rowDendrogramX, rowDendrogramY, rowDendrogramW, rowDendrogramH],
                                                  frame_on=False)
            ind1, leafIndex1 = self.plotDendrogram(matrix, axisRowDendrogram, self.sortRowMethod,
                                                   self.clusteringRowThreshold, 'left', bPlot=self.bShowRowDendrogram)

        if self.sortColMethod == 'Alphabetical order':
            leafIndex2 = numpy.argsort(colHeaders)
        elif self.sortColMethod == 'Mean abundance':
            leafIndex2 = numpy.argsort(numpy.mean(matrix, axis=0))
        else:
            axisColDendrogram = self.fig.add_axes([colDendrogramX, colDendrogramY, colDendrogramW, colDendrogramH],
                                                  frame_on=False)
            ind2, leafIndex2 = self.plotDendrogram(matrix.T, axisColDendrogram, self.sortColMethod,
                                                   self.clusteringColThreshold, 'top', bPlot=self.bShowColDendrogram)

        # *** Handle mouse events
        xCell = []
        yCell = []
        tooltips = []
        for x in range(0, len(colHeaders)):
            for y in range(0, len(rowHeaders)):
                xCell.append(x)
                yCell.append(y)

                tooltip = rowHeaders[leafIndex1[y]] + ', ' + colHeaders[leafIndex2[x]] + '\n'

                if self.fieldToPlot == "Number of sequences":
                    tooltip += '%d' % (matrix[leafIndex1[y]][leafIndex2[x]])
                else:
                    tooltip += '%.3f' % (matrix[leafIndex1[y]][leafIndex2[x]]) + '%'
                tooltips.append(tooltip)

        self.plotEventHandler = PlotEventHandler(xCell, yCell, tooltips, 0.4, 0.4)
        self.mouseEventCallback(self.plotEventHandler)

        # plot column clustering bars
        sampleColourMap = []

        for i in leafIndex2:
            groupName = profile.getSampleGroup(colHeaders[i])
            sampleColourMap.append(groupColours[groupName])

        sampleColourMap = mpl.colors.ListedColormap(sampleColourMap)
        matrix = matrix[:, leafIndex2]

        if self.bShowColDendrogram:
            ind2 = ind2[leafIndex2]
            axc = self.fig.add_axes(
                [colClusterBarX, colClusterBarY, colClusterBarW, colClusterBarH])  # axes for column side colorbar
            dc = numpy.array(numpy.arange(len(leafIndex2)), dtype=int)
            dc.shape = (1, len(leafIndex2))
            # FIX: aspect='auto' for visibility
            axc.imshow(dc, aspect='auto', origin='lower', cmap=sampleColourMap)
            axc.set_xticks([])
            axc.set_yticks([])

        # plot row clustering bars
        matrix = matrix[leafIndex1, :]

        if self.bShowRowDendrogram:
            ind1 = ind1[leafIndex1]
            axr = self.fig.add_axes([rowClusterBarX, rowClusterBarY, rowClusterBarW, rowClusterBarH])
            dr = numpy.array(ind1, dtype=int)
            dr.shape = (len(ind1), 1)
            # FIX: aspect='auto' for visibility
            axr.imshow(dr, aspect='auto', origin='lower', cmap=self.discreteColourMap)
            axr.set_xticks([])
            axr.set_yticks([])

        # determine scale for colour map
        minValue = 1e6
        maxValue = 0
        for row in matrix:
            minValue = min(minValue, min(row))
            maxValue = max(maxValue, max(row))
        norm = mpl.colors.Normalize(minValue, maxValue)

        # plot heatmap
        axisHeatmap = self.fig.add_axes([heatmapX, heatmapY, heatmapW, heatmapH])

        # FIX: Use imshow with aspect='auto' so zoom works properly
        axisHeatmap.imshow(matrix, origin='lower', aspect='auto', cmap=self.matrixColourmap, norm=norm)

        # Use standard ticks
        axisHeatmap.set_xticks(numpy.arange(len(colHeaders)))
        axisHeatmap.set_yticks(numpy.arange(len(rowHeaders)))

        # Reorder labels according to clustering
        reordered_rows = [rowHeaders[i] for i in leafIndex1]
        reordered_cols = [colHeaders[i] for i in leafIndex2]

        # FIX: Move Y-axis labels to the right to avoid overlap with tree
        axisHeatmap.yaxis.tick_right()

        axisHeatmap.set_yticklabels(reordered_rows)
        axisHeatmap.set_xticklabels(reordered_cols, rotation=90)

        # Remove tick marks to reduce clutter
        axisHeatmap.tick_params(axis='both', which='both', length=0)

        # plot colour map legend
        axisColourMap = self.fig.add_axes([legendX, legendY, legendW, legendH], frame_on=False)  # axes for colorbar
        colourBar = mpl.colorbar.ColorbarBase(axisColourMap, cmap=self.matrixColourmap, norm=norm,
                                              orientation='horizontal')

        if self.fieldToPlot == "Number of sequences":
            axisColourMap.set_title("# sequences")
        else:
            axisColourMap.set_title("abundance (%)")

        colourBar.set_ticks([minValue, 0.5 * (maxValue - minValue) + minValue, maxValue])
        colourBar.set_ticklabels(
            ['%.1f' % minValue, '%.1f' % (0.5 * (maxValue - minValue) + minValue), '%.1f' % maxValue])

        # plot legend
        if self.legendPos != -1:
            groupRecs = []
            groups = []
            for group, colour in groupColours.items():
                groupRecs.append(Rectangle((0, 0), 1, 1, fc=colour))
                groups.append(group)

            legend = self.fig.legend(groupRecs, groups, loc=self.legendPos, ncol=int(len(groups) / 2 + 0.5))
            legend.get_frame().set_linewidth(0)

        self.updateGeometry()
        self.draw()

    def configure(self, profile, statsResults):
        configDlg = ConfigureDialog(Ui_HeatmapPlotDialog)

        configDlg.ui.cboFieldToPlot.setCurrentIndex(configDlg.ui.cboFieldToPlot.findText(self.fieldToPlot))
        configDlg.ui.chkPlotOnlyActiveFeatures.setChecked(self.bPlotOnlyActiveFeatures)

        configDlg.ui.spinFigWidth.setValue(self.figWidth)
        configDlg.ui.spinFigHeight.setValue(self.figHeight)

        configDlg.ui.cboColSortMethod.setCurrentIndex(configDlg.ui.cboColSortMethod.findText(self.sortColMethod))
        configDlg.ui.cboRowSortMethod.setCurrentIndex(configDlg.ui.cboRowSortMethod.findText(self.sortRowMethod))

        configDlg.ui.chkShowColDendrogram.setChecked(self.bShowColDendrogram)
        configDlg.ui.chkShowRowDendrogram.setChecked(self.bShowRowDendrogram)

        configDlg.ui.cboColourMap.setCurrentIndex(configDlg.ui.cboColourMap.findText(self.colourmap))

        # legend position
        if self.legendPos == 1:
            configDlg.ui.radioLegendPosUpperRight.setChecked(True)
        elif self.legendPos == 4:
            configDlg.ui.radioLegendPosLowerRight.setChecked(True)
        elif self.legendPos == 2:
            configDlg.ui.radioLegendPosUpperLeft.setChecked(True)
        elif self.legendPos == 3:
            configDlg.ui.radioLegendPosLowerLeft.setChecked(True)
        else:
            configDlg.ui.radioLegendPosNone.setChecked(True)

        configDlg.ui.spinColClusteringThreshold.setValue(self.clusteringColThreshold)
        configDlg.ui.spinRowClusteringThreshold.setValue(self.clusteringRowThreshold)

        configDlg.ui.spinDendrogramColHeight.setValue(self.dendrogramHeight)
        configDlg.ui.spinDendrogramRowWidth.setValue(self.dendrogramWidth)

        if configDlg.exec_() == QtWidgets.QDialog.Accepted:
            self.fieldToPlot = str(configDlg.ui.cboFieldToPlot.currentText())
            self.bPlotOnlyActiveFeatures = configDlg.ui.chkPlotOnlyActiveFeatures.isChecked()

            self.figWidth = configDlg.ui.spinFigWidth.value()
            self.figHeight = configDlg.ui.spinFigHeight.value()

            self.sortColMethod = str(configDlg.ui.cboColSortMethod.currentText())
            self.sortRowMethod = str(configDlg.ui.cboRowSortMethod.currentText())

            self.bShowColDendrogram = configDlg.ui.chkShowColDendrogram.isChecked()
            self.bShowRowDendrogram = configDlg.ui.chkShowRowDendrogram.isChecked()

            self.colourmap = str(configDlg.ui.cboColourMap.currentText())

            # legend position
            if configDlg.ui.radioLegendPosUpperRight.isChecked():
                self.legendPos = 1
            elif configDlg.ui.radioLegendPosLowerRight.isChecked():
                self.legendPos = 4
            elif configDlg.ui.radioLegendPosUpperLeft.isChecked():
                self.legendPos = 2
            elif configDlg.ui.radioLegendPosLowerLeft.isChecked():
                self.legendPos = 3
            else:
                self.legendPos = -1

            self.clusteringColThreshold = configDlg.ui.spinColClusteringThreshold.value()
            self.clusteringRowThreshold = configDlg.ui.spinRowClusteringThreshold.value()

            self.dendrogramHeight = configDlg.ui.spinDendrogramColHeight.value()
            self.dendrogramWidth = configDlg.ui.spinDendrogramRowWidth.value()

            self.settings.setValue('group: ' + self.name + '/field to plot', self.fieldToPlot)
            self.settings.setValue('group: ' + self.name + '/plot only active features', self.bPlotOnlyActiveFeatures)
            self.settings.setValue('group: ' + self.name + '/width', self.figWidth)
            self.settings.setValue('group: ' + self.name + '/height', self.figHeight)
            self.settings.setValue('group: ' + self.name + '/sort col method', self.sortColMethod)
            self.settings.setValue('group: ' + self.name + '/sort row method', self.sortRowMethod)
            self.settings.setValue('group: ' + self.name + '/show col dendrogram', self.bShowColDendrogram)
            self.settings.setValue('group: ' + self.name + '/show row dendrogram', self.bShowRowDendrogram)
            self.settings.setValue('group: ' + self.name + '/colourmap', self.colourmap)
            self.settings.setValue('group: ' + self.name + '/legend position', self.legendPos)
            self.settings.setValue('group: ' + self.name + '/clustering col threshold', self.clusteringColThreshold)
            self.settings.setValue('group: ' + self.name + '/clustering row threshold', self.clusteringRowThreshold)
            self.settings.setValue('group: ' + self.name + '/dendrogram col height', self.dendrogramHeight)
            self.settings.setValue('group: ' + self.name + '/dendrogram row width', self.dendrogramWidth)

            self.plot(profile, statsResults)


if __name__ == "__main__":
    app = QtWidgets.QApplication(sys.argv)
    testWindow = TestWindow(HeatmapPlot)
    testWindow.show()
    sys.exit(app.exec_())