#=======================================================================
# Author: Donovan Parks
#
# Heatmap for two groups.
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
#=======================================================================

import sys

from PyQt4 import QtGui, QtCore

from stamp.plugins.multiGroups.AbstractMultiGroupPlotPlugin import AbstractMultiGroupPlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.multiGroups.plots.configGUI.HeatmapPlotUI import Ui_HeatmapPlotDialog

from stamp.plugins.PlotEventHandler import PlotEventHandler
import stamp.plugins.common.hierarchy as cluster

from matplotlib import pylab
import matplotlib as mpl
from matplotlib.patches import Rectangle

import numpy

import scipy.spatial.distance as dist

class HeatmapPlot(AbstractMultiGroupPlotPlugin):
	'''
	Heatmap plot.
	'''
	def __init__(self, preferences, parent=None):
		AbstractMultiGroupPlotPlugin.__init__(self, preferences, parent)
		

		self.discreteColourMap = mpl.colors.ListedColormap([(141/255.0, 211/255.0, 199/255.0),(255/255.0, 255/255.0, 179/255.0),\
														(190/255.0, 186/255.0, 218/255.0),(251/255.0, 128/255.0, 114/255.0),\
														(128/255.0, 177/255.0, 211/255.0),(253/255.0, 180/255.0, 98/255.0),\
														(179/255.0, 222/255.0, 105/255.0),(252/255.0, 205/255.0, 229/255.0),\
														(217/255.0, 217/255.0, 217/255.0), (188/255.0, 128/255.0, 189/255.0),\
														(204/255.0, 235/255.0, 197/255.0),(255/255.0, 237/255.0, 111/255.0)])
		
		self.preferences = preferences
		
		self.name = 'Heatmap plot'
		self.type = 'Exploratory'
		
		self.bPlotFeaturesIndividually = False
		
		self.settings = preferences['Settings']
		self.fieldToPlot = self.settings.value('multiple group: ' + self.name + '/field to plot', 'Proportion of sequences (%)').toString()
		self.figWidth = self.settings.value('multiple group: ' + self.name + '/width', 7.0).toDouble()[0]
		self.figHeight = self.settings.value('multiple group: ' + self.name + '/height', 7.0).toDouble()[0]
		self.dendrogramMethod = self.settings.value('multiple group: ' + self.name + '/dendrogram method', 'Average neighbour (UPGMA)').toString()
		self.bShowTopDendrogram = self.settings.value('multiple group: ' + self.name + '/show top dendrogram', True).toBool()
		self.bShowLeftDendrogram = self.settings.value('multiple group: ' + self.name + '/show left dendrogram', True).toBool()
		self.colourmap = self.settings.value('multiple group: ' + self.name + '/colourmap', 'Blues').toString()
		self.legendPos = self.settings.value('multiple group: ' + self.name + '/legend position', 3).toInt()[0]
		self.clusteringThreshold = self.settings.value('multiple group: ' + self.name + '/clustering threshold', 0.75).toDouble()[0]
		self.dendrogramHeight = self.settings.value('multiple group: ' + self.name + '/dendrogram height', 1.5).toDouble()[0]
		self.dendrogramWidth = self.settings.value('multiple group: ' + self.name + '/dendrogram width', 1.5).toDouble()[0]

	def mirrorProperties(self, plotToCopy):
		super(HeatmapPlot, self).mirrorProperties(plotToCopy)
		
		self.name = plotToCopy.name
		
		self.fieldToPlot = plotToCopy.fieldToPlot
		
		self.figWidth = plotToCopy.figWidth
		self.figHeight = plotToCopy.figHeight
		
		self.dendrogramMethod = plotToCopy.dendrogramMethod
		self.bShowTopDendrogram = plotToCopy.bShowTopDendrogram
		self.bShowLeftDendrogram = plotToCopy.bShowLeftDendrogram
		self.colourmap = plotToCopy.colourmap
		self.legendPos = plotToCopy.legendPos
		
		self.clusteringThreshold = plotToCopy.clusteringThreshold
		self.dendrogramHeight = plotToCopy.dendrogramHeight
		self.dendrogramWidth = plotToCopy.dendrogramWidth
		
	def plotDendrogram(self, matrix, axis, clusteringThreshold, orientation, bPlot):
		d = dist.pdist(matrix)
		
		if self.dendrogramMethod == 'Average neighbour (UPGMA)':
			linkage = cluster.linkage(dist.squareform(d), method='average')
		elif self.dendrogramMethod == 'Centroid':
			linkage = cluster.linkage(dist.squareform(d), method='centroid')
		elif self.dendrogramMethod == 'Nearest neighbour':
			linkage = cluster.linkage(dist.squareform(d), method='single')
		elif self.dendrogramMethod == 'Furthest neighbour':
			linkage = cluster.linkage(dist.squareform(d), method='complete')
		elif self.dendrogramMethod == 'Ward':
			linkage = cluster.linkage(dist.squareform(d), method='ward')
			
		dendrogram = cluster.dendrogram(linkage, orientation=orientation, link_color_func=lambda k: 'k', axis=axis, no_plot = not bPlot)
		index = cluster.fcluster(linkage, clusteringThreshold * max(linkage[:,2]), 'distance')
		axis.set_xticks([])
		axis.set_yticks([])
		
		return index, dendrogram['leaves']

	def plot(self, profile, statsResults):
		if len(profile.profileDict) <= 0 or len(profile.activeGroupNames) == 0:
			self.emptyAxis()
			return
		elif len(profile.profileDict) > 100 or len(profile.samplesInGroup1) + len(profile.samplesInGroup2) > 100:
			QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.ArrowCursor))
			QtGui.QMessageBox.information(self, 'Too much data!', 'Heatmap plots are limited to 100 samples and 100 features.', QtGui.QMessageBox.Ok)
			QtGui.QApplication.instance().restoreOverrideCursor()
			self.emptyAxis()
			return
			
		# *** Colour of plot elements
		groupColours = {}
		for groupName in profile.activeGroupNames:
			groupColours[groupName] = str(self.preferences['Group colours'][groupName].name())
		
		# *** Colour map for category dendrogram on left
		if self.colourmap == "Blues":
			self.matrixColourmap = pylab.cm.Blues
		elif self.colourmap == "Blue to red to green":
			self.matrixColourmap = pylab.cm.brg
		elif self.colourmap == "Blue to white to red":
			self.matrixColourmap = pylab.cm.bwr
		elif self.colourmap == "Cool to warm":
			self.matrixColourmap = pylab.cm.cool
		elif self.colourmap == "Grayscale":
			self.matrixColourmap = pylab.cm.gist_yarg
		elif self.colourmap == "Jet":
			self.matrixColourmap = pylab.cm.jet
		elif self.colourmap == "Orange to red":
			self.matrixColourmap = pylab.cm.OrRd
		elif self.colourmap == "Paired":
			self.matrixColourmap = pylab.cm.Paired
		elif self.colourmap == "Purple to green":
			self.matrixColourmap = pylab.cm.PRGn
		elif self.colourmap == "Reds":
			self.matrixColourmap = pylab.cm.Reds
		elif self.colourmap == "Red to blue":
			self.matrixColourmap = pylab.cm.RdBu
		elif self.colourmap == "Spectral":
			self.matrixColourmap = pylab.cm.spectral
		elif self.colourmap == "Yellow to orange to red":
			self.matrixColourmap = pylab.cm.YlOrRd
		
		# *** Get data for each group
		if self.fieldToPlot == "Number of sequences":
			data = profile.getActiveFeatureCountsAll()
		else: # Proportion of sequences (%)
			data = profile.getActiveFeatureProportionsAll()
			
		matrix = []
		for r in xrange(0, len(data)):
			row = []
			for d in data[r]:
				row += d
			matrix.append(row)

		matrix = numpy.array(matrix)
			
		# *** Get heatmap data
		colHeaders = [] 
		for sampleNames in profile.activeSamplesInGroups:
			colHeaders += sampleNames
		rowHeaders = profile.getFeatures()
		
		# *** Find longest label
		bTruncate = False
		if self.preferences['Truncate feature names']:
			length = self.preferences['Length of truncated feature names']
			bTruncate = True
		
		longestLabelLen = 0
		longestRowLabel = ''
		for i in xrange(0, len(rowHeaders)):
			if bTruncate and len(rowHeaders[i]) > length+3:
				rowHeaders[i] = rowHeaders[i][0:length] + '...'
				
			if len(rowHeaders[i]) > longestLabelLen:
				longestLabelLen = len(rowHeaders[i])
				longestRowLabel = rowHeaders[i]
				
		longestLabelLen = 0
		longestColLabel = ''
		for i in xrange(0, len(colHeaders)):
			if bTruncate and len(colHeaders[i]) > length+3:
				colHeaders[i] = colHeaders[i][0:length] + '...'
				
			if len(colHeaders[i]) > longestLabelLen:
				longestLabelLen = len(colHeaders[i])
				longestColLabel = colHeaders[i]
					
		# *** Set figure size
		self.fig.clear()
		self.fig.set_size_inches(self.figWidth, self.figHeight)
		
		xLabelBounds, yLabelBounds = self.labelExtents([longestColLabel], 8, 90, [longestRowLabel], 8, 0)
		
		# position all figure elements
		colourBarWidthX = 0.2 / self.figWidth
		colourBarWidthY = 0.2 / self.figHeight
		marginX = 0.05 / self.figWidth
		marginY = 0.05 / self.figHeight
		
		dendrogramWidth = self.dendrogramWidth / self.figWidth
		dendrogramHeight = self.dendrogramHeight / self.figHeight

		cellSizeX =  (1.0 - 2*0.02 - dendrogramWidth - colourBarWidthX - 2*marginX - yLabelBounds.width)*self.figWidth/len(colHeaders)
		cellSizeY =  (1.0 - 2*0.02 - dendrogramHeight - colourBarWidthY - 2*marginY - xLabelBounds.height)*self.figHeight/len(rowHeaders)
		cellSize = min(cellSizeX, cellSizeY)

		cellSizeXPer = cellSize / self.figWidth
		cellSizeYPer = cellSize / self.figHeight
		
		paddingX = 0.5*(1.0 - dendrogramWidth - 2*marginX - colourBarWidthX - cellSizeXPer*len(colHeaders) - yLabelBounds.width)
		paddingY = 0.5*(1.0 - dendrogramHeight - 2*marginY - colourBarWidthY - cellSizeYPer*len(rowHeaders) - xLabelBounds.height)

		rowDendrogramX = paddingX
		rowDendrogramY = paddingY + (xLabelBounds.height)
		rowDendrogramW = dendrogramWidth
		rowDendrogramH = cellSizeYPer*len(rowHeaders)

		rowClusterBarX = rowDendrogramX + rowDendrogramW + marginX
		rowClusterBarY = rowDendrogramY 
		rowClusterBarW = colourBarWidthX
		rowClusterBarH = rowDendrogramH
		
		colDendrogramX = rowClusterBarX + rowClusterBarW + marginX
		colDendrogramY = rowDendrogramY + rowDendrogramH + marginY + colourBarWidthY + marginY
		colDendrogramW = cellSizeXPer*len(colHeaders)
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
		legendX = rowDendrogramX
		legendY = 1.0 - paddingY - legendHeight - yLabelBounds.height
		legendW = rowDendrogramW
		legendH = legendHeight

		# plot dendrograms
		axisRowDendrogram = self.fig.add_axes([rowDendrogramX, rowDendrogramY, rowDendrogramW, rowDendrogramH], frame_on=False)
		ind1, leafIndex1 = self.plotDendrogram(matrix, axisRowDendrogram, self.clusteringThreshold, 'right', bPlot = self.bShowLeftDendrogram)

		axisColDendrogram = self.fig.add_axes([colDendrogramX, colDendrogramY, colDendrogramW, colDendrogramH], frame_on=False)
		ind2, leafIndex2 = self.plotDendrogram(matrix.T, axisColDendrogram, self.clusteringThreshold, 'top', bPlot = self.bShowTopDendrogram)

		# *** Handle mouse events
		xCell = []
		yCell = []
		tooltips = []
		for x in xrange(0, len(colHeaders)):
			for y in xrange(0, len(rowHeaders)):
				xCell.append(x)
				yCell.append(y)
				
				tooltip = rowHeaders[leafIndex1[y]] + ', ' + colHeaders[leafIndex2[x]]  + '\n'
				
				if self.fieldToPlot == "Number of sequences":
					tooltip += '%d' % (matrix[leafIndex1[y]][leafIndex2[x]])
				else:
					tooltip += '%.3f' % (matrix[leafIndex1[y]][leafIndex2[x]]) + '%'
				tooltips.append(tooltip)
			
		self.plotEventHandler =	PlotEventHandler(xCell, yCell, tooltips, 0.4, 0.4)
		self.mouseEventCallback(self.plotEventHandler)

		# plot column clustering bars
		sampleColourMap = []

		for i in leafIndex2:
			groupName = profile.getSampleGroup(colHeaders[i])
			sampleColourMap.append(groupColours[groupName])
		
		sampleColourMap = mpl.colors.ListedColormap(sampleColourMap)
		matrix = matrix[:,leafIndex2]
		ind2 = ind2[:,leafIndex2] 
		
		axc = self.fig.add_axes([colClusterBarX, colClusterBarY, colClusterBarW, colClusterBarH])  # axes for column side colorbar
		dc = numpy.array(numpy.arange(len(leafIndex2)), dtype=int)
		dc.shape = (1,len(leafIndex2)) 
		axc.matshow(dc, aspect='auto', origin='lower', cmap=sampleColourMap)
		axc.set_xticks([]) 
		axc.set_yticks([])

		# plot row clustering bars
		matrix = matrix[leafIndex1,:]
		ind1 = ind1[leafIndex1,:]
		
		axr = self.fig.add_axes([rowClusterBarX, rowClusterBarY, rowClusterBarW, rowClusterBarH]) 
		dr = numpy.array(ind1, dtype=int)
		dr.shape = (len(ind1),1)
		axr.matshow(dr, aspect='auto', origin='lower', cmap=self.discreteColourMap)
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
		axisHeatmap.matshow(matrix, origin='lower', cmap = self.matrixColourmap, norm = norm)
		axisHeatmap.set_xticks([])
		axisHeatmap.set_yticks([])
		
		# row and column labels
		labelOffset = 0.5*(yLabelBounds.height / cellSizeYPer)
		for i in xrange(0, len(rowHeaders)):
			axisHeatmap.text(matrix.shape[1] - 0.5, i - labelOffset, '  ' + rowHeaders[leafIndex1[i]], horizontalalignment="left")

		labelOffset = 0.5*(xLabelBounds.width / cellSizeXPer)
		for i in xrange(0, len(colHeaders)):
			axisHeatmap.text(i - labelOffset, -0.5, '  ' + colHeaders[leafIndex2[i]], rotation = 270, verticalalignment="top")

		# plot colour map legend
		axisColourMap = self.fig.add_axes([legendX, legendY, legendW, legendH], frame_on=False)  # axes for colorbar
		colourBar = mpl.colorbar.ColorbarBase(axisColourMap, cmap=self.matrixColourmap, norm=norm, orientation='horizontal')
		
		if self.fieldToPlot == "Number of sequences":
			axisColourMap.set_title("# sequences")
		else:
			axisColourMap.set_title("abundance (%)")
			
		colourBar.set_ticks([minValue, 0.5*(maxValue-minValue) + minValue, maxValue])
		colourBar.set_ticklabels(['%.1f' % minValue, '%.1f' % (0.5*(maxValue-minValue) + minValue), '%.1f' % maxValue])
		
		# plot column and row lines
		for i in xrange(0, len(rowHeaders)):
			axisHeatmap.plot([-0.5, len(colHeaders)-0.5], [i-0.5,i-0.5], color='white', linestyle='-', linewidth=1.5)
			
		for i in xrange(0, len(colHeaders)):
			axisHeatmap.plot([i-0.5, i-0.5], [-0.5, len(rowHeaders)-0.5], color='white', linestyle='-', linewidth=1.5)
		
		# plot legend
		if self.legendPos != -1:
			groupRecs = []
			groups = []
			for group, colour in groupColours.iteritems():
				groupRecs.append(Rectangle((0, 0), 1, 1, fc=colour))
				groups.append(group)

			legend = self.fig.legend(groupRecs, groups, loc=self.legendPos, ncol=int(len(groups)/2 + 0.5))
			legend.get_frame().set_linewidth(0)
					
		self.updateGeometry()
		self.draw()

	def configure(self, profile, statsResults):
		configDlg = ConfigureDialog(Ui_HeatmapPlotDialog)
		
		configDlg.ui.cboFieldToPlot.setCurrentIndex(configDlg.ui.cboFieldToPlot.findText(self.fieldToPlot))

		configDlg.ui.spinFigWidth.setValue(self.figWidth)
		configDlg.ui.spinFigHeight.setValue(self.figHeight)
		
		configDlg.ui.cboDendrogramMethod.setCurrentIndex(configDlg.ui.cboDendrogramMethod.findText(self.dendrogramMethod))
		
		configDlg.ui.chkShowTopDendrogram.setChecked(self.bShowTopDendrogram)
		configDlg.ui.chkShowLeftDendrogram.setChecked(self.bShowLeftDendrogram)
		
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
			
		configDlg.ui.spinClusteringThreshold.setValue(self.clusteringThreshold)
		configDlg.ui.spinDendrogramWidth.setValue(self.dendrogramWidth)
		configDlg.ui.spinDendrogramHeight.setValue(self.dendrogramHeight)
				
		if configDlg.exec_() == QtGui.QDialog.Accepted:
			self.fieldToPlot = str(configDlg.ui.cboFieldToPlot.currentText())
			
			self.figWidth = configDlg.ui.spinFigWidth.value()
			self.figHeight = configDlg.ui.spinFigHeight.value()
			
			self.dendrogramMethod = str(configDlg.ui.cboDendrogramMethod.currentText())
			
			self.bShowTopDendrogram = configDlg.ui.chkShowTopDendrogram.isChecked()
			self.bShowLeftDendrogram = configDlg.ui.chkShowLeftDendrogram.isChecked()
			
			self.colourmap = str(configDlg.ui.cboColourMap.currentText())
			
			# legend position			
			if configDlg.ui.radioLegendPosUpperRight.isChecked() == True:
				self.legendPos = 1
			elif configDlg.ui.radioLegendPosLowerRight.isChecked() == True:
				self.legendPos = 4
			elif configDlg.ui.radioLegendPosUpperLeft.isChecked() == True:
				self.legendPos = 2
			elif configDlg.ui.radioLegendPosLowerLeft.isChecked() == True:
				self.legendPos = 3
			else:
				self.legendPos = -1
				
			self.clusteringThreshold = configDlg.ui.spinClusteringThreshold.value()
			
			self.dendrogramWidth = configDlg.ui.spinDendrogramWidth.value()
			self.dendrogramHeight = configDlg.ui.spinDendrogramHeight.value()
				
			self.settings.setValue('multiple group: ' + self.name + '/field to plot', self.fieldToPlot)
			self.settings.setValue('multiple group: ' + self.name + '/width', self.figWidth)
			self.settings.setValue('multiple group: ' + self.name + '/height', self.figHeight)
			self.settings.setValue('multiple group: ' + self.name + '/dendrogram method', self.dendrogramMethod)
			self.settings.setValue('multiple group: ' + self.name + '/show top dendrogram', self.bShowTopDendrogram)
			self.settings.setValue('multiple group: ' + self.name + '/show left dendrogram', self.bShowLeftDendrogram)
			self.settings.setValue('multiple group: ' + self.name + '/colourmap', self.colourmap)
			self.settings.setValue('multiple group: ' + self.name + '/legend position', self.legendPos)
			self.settings.setValue('multiple group: ' + self.name + '/clustering threshold', self.clusteringThreshold)
			self.settings.setValue('multiple group: ' + self.name + '/dendrogram height', self.dendrogramHeight)
			self.settings.setValue('multiple group: ' + self.name + '/dendrogram width', self.dendrogramWidth)

			self.plot(profile, statsResults)
					
if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(HeatmapPlot)
	testWindow.show()
	sys.exit(app.exec_())