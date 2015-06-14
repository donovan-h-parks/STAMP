#=======================================================================
# Author: Donovan Parks
#
# PCA plot for two groups.
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
#=======================================================================

import sys
import math

from PyQt4 import QtGui

from stamp.plugins.groups.AbstractGroupPlotPlugin import AbstractGroupPlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.groups.plots.configGUI.pcaPlotUI import Ui_PcaPlotDialog

from stamp.plugins.PlotEventHandler import MultiPlotEventHandler

from stamp.metagenomics.PCA import pca

from matplotlib.patches import Rectangle

from numpy.linalg import LinAlgError

class pcaPlot(AbstractGroupPlotPlugin):
	'''
	Profile scatter plot.
	'''
	def __init__(self, preferences, parent=None):
		AbstractGroupPlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences

		self.name = 'PCA plot'
		self.type = 'Exploratory'
		
		self.bPlotFeaturesIndividually = False
		
		self.settings = preferences['Settings']
		self.figWidth = self.settings.value('group: ' + self.name + '/width', 7.0).toDouble()[0]
		self.figHeight = self.settings.value('group: ' + self.name + '/height', 6.0).toDouble()[0]
		self.bFixedPixelsPerUnitDistance = self.settings.value('group: ' + self.name + '/fixed pixels per unit distance', True).toBool()
		self.markerSize = self.settings.value('group: ' + self.name + '/marker size', 30).toInt()[0]
		self.bRotateLabels = self.settings.value('group: ' + self.name + '/rotate pc3 labels', True).toBool()
		self.bShowPC1vsPC3 = self.settings.value('group: ' + self.name + '/showPC1vsPC3', True).toBool()
		self.bShowPC3vsPC2 = self.settings.value('group: ' + self.name + '/showPC3vsPC2', True).toBool()
		self.legendPos = self.settings.value('group: ' + self.name + '/legend position', -1).toInt()[0]
		self.bUniqueShapes = self.settings.value('group: ' + self.name + '/unique shapes', True).toBool()
		
	def mirrorProperties(self, plotToCopy):
		self.name = plotToCopy.name
		self.figWidth = plotToCopy.figWidth
		self.figHeight = plotToCopy.figHeight
		self.bFixedPixelsPerUnitDistance = plotToCopy.bFixedPixelsPerUnitDistance
		
		self.bPlotFeaturesIndividually = False
		
		self.markerSize = plotToCopy.markerSize
		
		self.bRotateLabels = plotToCopy.bRotateLabels
		
		self.bShowPC1vsPC3 = plotToCopy.bShowPC1vsPC3
		self.bShowPC3vsPC2 = plotToCopy.bShowPC3vsPC2
		
		self.legendPos = plotToCopy.legendPos
		
		self.bUniqueShapes = plotToCopy.bUniqueShapes
		
	def plot(self, profile, statsResults):
		if len(profile.profileDict) <= 0 or (len(profile.samplesInGroup1) + len(profile.samplesInGroup2)) <= 2:
			self.emptyAxis('There are 2 or fewer samples.')
			return

		# *** Colour of plot elements
		axesColour = str(self.preferences['Axes colour'].name())
		group1Colour = str(self.preferences['Group colours'][profile.groupName1].name())
		group2Colour = str(self.preferences['Group colours'][profile.groupName2].name())
		
		# *** Calculate PCA of group of samples
		featureMatrix = profile.getFeatureMatrix()
		try:
			embedding, capturedVar = pca(featureMatrix)
		except LinAlgError:
			self.emptyAxis('Linear algebra error likely due to a singular matrix (i.e., degenerate data).\nTry plotting at a different level in the hierarchy.')
			return
		except:
			self.emptyAxis('Unknown error when performing PCA analysis.')
			return
			
		if math.isnan(capturedVar[0]):
			self.emptyAxis('Degenerate PCA plot.\nTry plotting at a different level in the hierarchy.')
			return
			
		if len(embedding[0]) == 1:
			pc1 = embedding[:,0]
			pc2 = embedding[:,0]
			pc3 = embedding[:,0]
		elif len(embedding[0]) == 2:
			pc1 = embedding[:,0]
			pc2 = embedding[:,1]
			pc3 = embedding[:,1]
		else:
			pc1 = embedding[:,0]
			pc2 = embedding[:,1]
			pc3 = embedding[:,2]
		
		if len(capturedVar) <= 1 or capturedVar[1] <= 0.001:
			self.emptyAxis('Degenerate PCA plot')
			return
			
		bPlot2D = False
		if len(capturedVar) == 2:
			bPlot2D = True

		# *** Set figure size
		self.fig.clear()
		
		axesExpandPercentage = 0.1
		
		border = 0.4 # inches
		plotSpacing = 0.15
		xLabelOffset = 0.5
		yLabelOffset = 0.2
		
		xBorderFigSpace = border / self.figWidth
		xPlotSpacingFigSpace = plotSpacing / self.figWidth
		xLabelOffsetFigSpace = xLabelOffset/self.figWidth
		
		if (bPlot2D == False) and self.bShowPC1vsPC3 and self.bShowPC3vsPC2: # all three plots
			if self.bFixedPixelsPerUnitDistance:
				pc1Percentage = (max(pc1)*axesExpandPercentage - min(pc1)*axesExpandPercentage) / ((max(pc1)*axesExpandPercentage - min(pc1)*axesExpandPercentage) + (max(pc3)*axesExpandPercentage-min(pc3)*axesExpandPercentage))
				pc1AxisLengthX = pc1Percentage*(self.figWidth - xLabelOffset - 2*border - plotSpacing)
				pc3AxisLengthX = (1.0-pc1Percentage)*(self.figWidth - xLabelOffset - 2*border - plotSpacing)
				pc2AxisLengthY = pc1AxisLengthX * ((max(pc2)*axesExpandPercentage - min(pc2)*axesExpandPercentage) / (max(pc1)*axesExpandPercentage-min(pc1)*axesExpandPercentage))
				pc3AxisLengthY = pc1AxisLengthX * ((max(pc3)*axesExpandPercentage - min(pc3)*axesExpandPercentage) / (max(pc1)*axesExpandPercentage-min(pc1)*axesExpandPercentage))
				figHeight = pc2AxisLengthY + pc3AxisLengthY + yLabelOffset + 2*border + plotSpacing
			else:
				pc1AxisLengthX = 0.5*(self.figWidth - xLabelOffset - 2*border - plotSpacing)
				pc3AxisLengthX = pc1AxisLengthX
				pc2AxisLengthY = 0.5*(self.figHeight - yLabelOffset - 2*border - plotSpacing)
				pc3AxisLengthY = pc2AxisLengthY
				figHeight = self.figHeight
			
			if math.isnan(figHeight) or math.isnan(pc1AxisLengthX) or math.isnan(pc2AxisLengthY) or (pc2AxisLengthY == 0):
				self.emptyAxis('Degenerate PCA plot')
				return
				
			self.fig.set_size_inches(self.figWidth, figHeight)

			yBorderFigSpace = border / figHeight
			yPlotSpacingFigSpace = plotSpacing / figHeight
			yLabelOffsetFigSpace = yLabelOffset/figHeight
			
			axesScatter1 = self.fig.add_axes([xBorderFigSpace + xLabelOffsetFigSpace, yBorderFigSpace + yLabelOffsetFigSpace + pc3AxisLengthY/figHeight + yPlotSpacingFigSpace,
																					pc1AxisLengthX/self.figWidth, pc2AxisLengthY/figHeight])
			axesScatter2 = self.fig.add_axes([xBorderFigSpace + xLabelOffsetFigSpace + pc1AxisLengthX/self.figWidth + xPlotSpacingFigSpace, yBorderFigSpace + yLabelOffsetFigSpace + pc3AxisLengthY/figHeight + yPlotSpacingFigSpace,
																					pc3AxisLengthX/self.figWidth, pc2AxisLengthY/figHeight])
			axesScatter3 = self.fig.add_axes([xBorderFigSpace + xLabelOffsetFigSpace, yBorderFigSpace + yLabelOffsetFigSpace,
																					pc1AxisLengthX/self.figWidth, pc3AxisLengthY/figHeight])
		elif (bPlot2D == False) and self.bShowPC1vsPC3: # vertical plots
			if self.bFixedPixelsPerUnitDistance:
				pc1AxisLengthX = self.figWidth - xLabelOffset - 2*border
				pc2AxisLengthY = pc1AxisLengthX * ((max(pc2)*axesExpandPercentage - min(pc2)*axesExpandPercentage) / (max(pc1)*axesExpandPercentage-min(pc1)*axesExpandPercentage))
				pc3AxisLengthY = pc1AxisLengthX * ((max(pc3)*axesExpandPercentage - min(pc3)*axesExpandPercentage) / (max(pc1)*axesExpandPercentage-min(pc1)*axesExpandPercentage))
				figHeight = pc2AxisLengthY + pc3AxisLengthY + yLabelOffset + 2*border + plotSpacing
			else:
				pc1AxisLengthX = self.figWidth - xLabelOffset - 2*border
				pc2AxisLengthY = 0.5*(self.figHeight - yLabelOffset - 2*border - plotSpacing)
				pc3AxisLengthY = pc2AxisLengthY
				figHeight = self.figHeight
			
			if math.isnan(figHeight) or math.isnan(pc1AxisLengthX) or math.isnan(pc2AxisLengthY) or (pc3AxisLengthY == 0):
				self.emptyAxis('Degenerate PCA plot')
				return
				
			self.fig.set_size_inches(self.figWidth, figHeight)

			yBorderFigSpace = border / figHeight
			yPlotSpacingFigSpace = plotSpacing / figHeight
			yLabelOffsetFigSpace = yLabelOffset/figHeight
			
			axesScatter1 = self.fig.add_axes([xBorderFigSpace + xLabelOffsetFigSpace, yBorderFigSpace + yLabelOffsetFigSpace + pc3AxisLengthY/figHeight + yPlotSpacingFigSpace,
																					pc1AxisLengthX/self.figWidth, pc2AxisLengthY/figHeight])
			axesScatter3 = self.fig.add_axes([xBorderFigSpace + xLabelOffsetFigSpace, yBorderFigSpace + yLabelOffsetFigSpace,
																					pc1AxisLengthX/self.figWidth, pc3AxisLengthY/figHeight])
		
		elif (bPlot2D == False) and self.bShowPC3vsPC2: # horizontal plots
			if self.bFixedPixelsPerUnitDistance:
				pc1Percentage = (max(pc1)*axesExpandPercentage - min(pc1)*axesExpandPercentage) / ((max(pc1)*axesExpandPercentage - min(pc1)*axesExpandPercentage) + (max(pc3)*axesExpandPercentage-min(pc3)*axesExpandPercentage))
				pc1AxisLengthX = pc1Percentage*(self.figWidth - xLabelOffset - 2*border - plotSpacing)
				pc3AxisLengthX = (1.0-pc1Percentage)*(self.figWidth - xLabelOffset - 2*border - plotSpacing)
				pc2AxisLengthY = pc1AxisLengthX * ((max(pc2)*axesExpandPercentage - min(pc2)*axesExpandPercentage) / (max(pc1)*axesExpandPercentage-min(pc1)*axesExpandPercentage))
				figHeight = pc2AxisLengthY + yLabelOffset + 2*border
			else:
				pc1AxisLengthX = 0.5*(self.figWidth - xLabelOffset - 2*border - plotSpacing)
				pc3AxisLengthX = pc1AxisLengthX
				pc2AxisLengthY = 0.5*(self.figHeight - yLabelOffset - 2*border - plotSpacing)
				figHeight = self.figHeight
			
			if math.isnan(figHeight) or math.isnan(pc1AxisLengthX) or math.isnan(pc2AxisLengthY):
				self.emptyAxis('Degenerate PCA plot')
				return
				
			self.fig.set_size_inches(self.figWidth, figHeight)

			yBorderFigSpace = border / figHeight
			yPlotSpacingFigSpace = plotSpacing / figHeight
			yLabelOffsetFigSpace = yLabelOffset/figHeight
			
			axesScatter1 = self.fig.add_axes([xBorderFigSpace + xLabelOffsetFigSpace, yBorderFigSpace + yLabelOffsetFigSpace,
																					pc1AxisLengthX/self.figWidth, pc2AxisLengthY/figHeight])
			axesScatter2 = self.fig.add_axes([xBorderFigSpace + xLabelOffsetFigSpace + pc1AxisLengthX/self.figWidth + xPlotSpacingFigSpace, yBorderFigSpace + yLabelOffsetFigSpace,
																					pc3AxisLengthX/self.figWidth, pc2AxisLengthY/figHeight])

		else: # just PC1 vs. PC2
			if self.bFixedPixelsPerUnitDistance:
				pc1AxisLengthX = self.figWidth - xLabelOffset - 2*border
				pc2AxisLengthY = pc1AxisLengthX * ((max(pc2)*axesExpandPercentage - min(pc2)*axesExpandPercentage) / (max(pc1)*axesExpandPercentage-min(pc1)*axesExpandPercentage))
				figHeight = pc2AxisLengthY + yLabelOffset + 2*border
			else:
				pc1AxisLengthX = self.figWidth - xLabelOffset - 2*border
				pc2AxisLengthY = self.figHeight - yLabelOffset - 2*border
				figHeight = self.figHeight
				
			self.fig.set_size_inches(self.figWidth, figHeight)
				
			yBorderFigSpace = border / figHeight
			yLabelOffsetFigSpace = yLabelOffset/figHeight
			
			axesScatter1 = self.fig.add_axes([xBorderFigSpace + xLabelOffsetFigSpace, yBorderFigSpace + yLabelOffsetFigSpace,
																					pc1AxisLengthX/self.figWidth, pc2AxisLengthY/figHeight])
				
		# *** Plot data
		
		# set visual properties of all points
		if self.bUniqueShapes:
			markers = ['o', 's']
		else:
			markers = ['o', 'o']
			
		colours = []
		numGroup1Samples = len(profile.samplesInGroup1)
		for _ in xrange(0, numGroup1Samples):
			colours.append(group1Colour)
		for _ in xrange(0, len(profile.samplesInGroup2)):
			colours.append(group2Colour)

		# scatter plots
		scatterPlotAxes = [axesScatter1]
		
		axesScatter1.scatter(pc1[0:numGroup1Samples], pc2[0:numGroup1Samples], s=self.markerSize, zorder=5, c=colours[0:numGroup1Samples], marker = markers[0])
		axesScatter1.scatter(pc1[numGroup1Samples:], pc2[numGroup1Samples:], s=self.markerSize, zorder=5, c=colours[numGroup1Samples:], marker = markers[1])
		
		xOffset = (max(pc1) - min(pc1))*axesExpandPercentage
		axesScatter1.set_xlim([min(pc1)-xOffset, max(pc1)+xOffset])
		yOffset = (max(pc2) - min(pc2))*axesExpandPercentage
		axesScatter1.set_ylim([min(pc2)-yOffset, max(pc2)+yOffset])
		
		axesScatter1.set_ylabel('PC2 (' + '%.1f' % (capturedVar[1]*100) + '%)')
		
		xData = [pc1]
		yData = [pc2]
		if (bPlot2D == False) and self.bShowPC3vsPC2:
			try:
				axesScatter2.scatter(pc3[0:numGroup1Samples], pc2[0:numGroup1Samples], s=self.markerSize, zorder=5, c=colours[0:numGroup1Samples], marker = markers[0])
				axesScatter2.scatter(pc3[numGroup1Samples:], pc2[numGroup1Samples:], s=self.markerSize, zorder=5, c=colours[numGroup1Samples:], marker = markers[1])
			except:
				# failed to plot PC3, but continue on since the plot of PC1 vs. PC2 can be informative
				pass
			
			xOffset = (max(pc3) - min(pc3))*axesExpandPercentage
			axesScatter2.set_xlim([min(pc3)-xOffset, max(pc3)+xOffset])
			yOffset = (max(pc2) - min(pc2))*axesExpandPercentage
			axesScatter2.set_ylim([min(pc2)-yOffset, max(pc2)+yOffset])
			axesScatter2.set_yticklabels([])
			
			axesScatter2.set_xlabel('PC3 (' + '%.1f' % (capturedVar[2]*100) + '%)')
			if self.bRotateLabels:
				for label in axesScatter2.get_xticklabels():
					label.set_rotation(45)
				
			scatterPlotAxes.append(axesScatter2)
			xData.append(pc3)
			yData.append(pc2)
			
		if (bPlot2D == False) and self.bShowPC1vsPC3:
			try:
				axesScatter3.scatter(pc1[0:numGroup1Samples], pc3[0:numGroup1Samples], s=self.markerSize, zorder=5, c=colours[0:numGroup1Samples], marker = markers[0])
				axesScatter3.scatter(pc1[numGroup1Samples:], pc3[numGroup1Samples:], s=self.markerSize, zorder=5, c=colours[numGroup1Samples:], marker = markers[1])
			except:
				# failed to plot PC3, but continue on since the plot of PC1 vs. PC2 can be informative
				pass
			
			xOffset = (max(pc1) - min(pc1))*axesExpandPercentage
			axesScatter3.set_xlim([min(pc1)-xOffset, max(pc1)+xOffset])
			yOffset = (max(pc3) - min(pc3))*axesExpandPercentage
			axesScatter3.set_ylim([min(pc3)-yOffset, max(pc3)+yOffset])
			
			axesScatter1.set_xticklabels([])
			axesScatter3.set_xlabel('PC1 (' + '%.1f' % (capturedVar[0]*100) + '%)')
			axesScatter3.set_ylabel('PC3 (' + '%.1f' % (capturedVar[2]*100) + '%)')
			
			scatterPlotAxes.append(axesScatter3)
			xData.append(pc1)
			yData.append(pc3)
			
		else:
			axesScatter1.set_xlabel('PC1 (' + '%.1f' % (capturedVar[0]*100) + '%)')
			
		# plot x=0 and y=0 lines
		for axes in scatterPlotAxes:
			axes.axhline(color=axesColour, linestyle='dashed', zorder = 1)
			axes.axvline(color=axesColour, linestyle='dashed', zorder = 1)

		# *** Prettify scatter plot
		for axes in scatterPlotAxes:
			for a in axes.yaxis.majorTicks:
				a.tick1On=True
				a.tick2On=False
					
			for a in axes.xaxis.majorTicks:
				a.tick1On=True
				a.tick2On=False
				
			for line in axes.yaxis.get_ticklines(): 
				line.set_color(axesColour)
					
			for line in axes.xaxis.get_ticklines(): 
				line.set_color(axesColour)
				
			for loc, spine in axes.spines.iteritems():
				if loc in ['right','top']:
					spine.set_color('none') 
				else:
					spine.set_color(axesColour)
					
		# *** Handle mouse events
		tooltips = []
		for sampleName in profile.samplesInGroup1:
			tooltip = profile.groupName1 + ': ' + sampleName
			tooltips.append(tooltip)
			
		for sampleName in profile.samplesInGroup2:
			tooltip = profile.groupName2 + ': ' + sampleName
			tooltips.append(tooltip)
			
		self.plotEventHandler =	MultiPlotEventHandler(xData, yData, scatterPlotAxes, tooltips)
		self.mouseEventCallback(self.plotEventHandler)
		
		# *** Legend
		if self.legendPos != -1:
			legend1 = Rectangle((0, 0), 1, 1, fc=group1Colour)
			legend2 = Rectangle((0, 0), 1, 1, fc=group2Colour)
			legend = self.fig.legend([legend1, legend2], (profile.groupName1, profile.groupName2), loc=self.legendPos, ncol=2)
			legend.get_frame().set_linewidth(0)

		self.updateGeometry()
		self.draw()

	def configure(self, profile, statsResults):
		configDlg = ConfigureDialog(Ui_PcaPlotDialog)

		configDlg.ui.spinFigWidth.setValue(self.figWidth)
		configDlg.ui.spinFigHeight.setValue(self.figHeight)
		
		configDlg.ui.chkFixedPixelsPerUnitDistance.setChecked(self.bFixedPixelsPerUnitDistance)
		
		configDlg.ui.spinFigHeight.setDisabled(configDlg.ui.chkFixedPixelsPerUnitDistance.isChecked())
		
		configDlg.ui.spinMarkerSize.setValue(self.markerSize)
		
		configDlg.ui.chkRotateLabels.setChecked(self.bRotateLabels)
		
		configDlg.ui.chkPC1vsPC3.setChecked(self.bShowPC1vsPC3)
		configDlg.ui.chkPC3vsPC2.setChecked(self.bShowPC3vsPC2)
		
		if self.legendPos == 2:
			configDlg.ui.radioLegendPosUpperLeft.setChecked(True)
		elif self.legendPos == 3:
			configDlg.ui.radioLegendPosLowerLeft.setChecked(True)
		elif self.legendPos == 4:
			configDlg.ui.radioLegendPosLowerRight.setChecked(True)
		elif self.legendPos == 9:
			configDlg.ui.radioLegendPosUpperCentre.setChecked(True)
		elif self.legendPos == 1:
			configDlg.ui.radioLegendPosUpperRight.setChecked(True)
		else:
			configDlg.ui.radioLegendPosNone.setChecked(True)
			
		configDlg.ui.chkUniqueShapes.setChecked(self.bUniqueShapes)

		if configDlg.exec_() == QtGui.QDialog.Accepted:	 
			self.figWidth = configDlg.ui.spinFigWidth.value()
			self.figHeight = configDlg.ui.spinFigHeight.value()
			
			self.bFixedPixelsPerUnitDistance = configDlg.ui.chkFixedPixelsPerUnitDistance.isChecked()
			
			self.markerSize = configDlg.ui.spinMarkerSize.value()
			
			self.bRotateLabels = configDlg.ui.chkRotateLabels.isChecked()
			
			self.bShowPC1vsPC3 = configDlg.ui.chkPC1vsPC3.isChecked()
			self.bShowPC3vsPC2 = configDlg.ui.chkPC3vsPC2.isChecked()
			
			# legend position
			if configDlg.ui.radioLegendPosUpperLeft.isChecked() == True:
				self.legendPos = 2
			elif configDlg.ui.radioLegendPosLowerLeft.isChecked() == True:
				self.legendPos = 3
			elif configDlg.ui.radioLegendPosUpperCentre.isChecked() == True:
				self.legendPos = 9
			elif configDlg.ui.radioLegendPosLowerRight.isChecked() == True:
				self.legendPos = 4
			elif configDlg.ui.radioLegendPosUpperRight.isChecked() == True:
				self.legendPos = 1
			else:
				self.legendPos = -1
				
			self.bUniqueShapes = configDlg.ui.chkUniqueShapes.isChecked()
			
			self.settings.setValue('group: ' + self.name + '/width', self.figWidth)
			self.settings.setValue('group: ' + self.name + '/height', self.figHeight)
			self.settings.setValue('group: ' + self.name + '/fixed pixels per unit distance', self.bFixedPixelsPerUnitDistance)
			self.settings.setValue('group: ' + self.name + '/marker size', self.markerSize)
			self.settings.setValue('group: ' + self.name + '/rotate pc3 labels', self.bRotateLabels)
			self.settings.setValue('group: ' + self.name + '/showPC1vsPC3', self.bShowPC1vsPC3)
			self.settings.setValue('group: ' + self.name + '/showPC3vsPC2', self.bShowPC3vsPC2)
			self.settings.setValue('group: ' + self.name + '/legend position', self.legendPos)
			self.settings.setValue('group: ' + self.name + '/unique shapes', self.bUniqueShapes)

			self.plot(profile, statsResults)
					
if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(ProfileScatterPlot)
	testWindow.show()
	sys.exit(app.exec_())