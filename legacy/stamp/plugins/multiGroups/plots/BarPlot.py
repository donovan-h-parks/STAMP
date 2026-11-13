#=======================================================================
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
#======================================================================='''

import sys

from PyQt4 import QtGui, QtCore
import numpy as np

from stamp.plugins.multiGroups.AbstractMultiGroupPlotPlugin import AbstractMultiGroupPlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.multiGroups.plots.configGUI.BarPlotUI import Ui_BarPlotDialog

from stamp.metagenomics.stats.CI.WilsonCI import WilsonCI

class BarPlot(AbstractMultiGroupPlotPlugin):
	'''
	Bar plot for multiple groups.
	'''
	def __init__(self, preferences, parent=None):
		AbstractMultiGroupPlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences
	 
		self.name = 'Bar plot'
		self.type = 'Exploratory'
		
		self.settings = preferences['Settings']
		self.figColWidth = self.settings.value('multiple group: ' + self.name + '/column width', 0.2).toDouble()[0]
		self.figHeight = self.settings.value('multiple group: ' + self.name + '/height', 6.0).toDouble()[0]
		self.fieldToPlot = self.settings.value('multiple group: ' + self.name + '/field to plot', 'Proportion of sequences (%)').toString()
		self.bShowAverages = self.settings.value('multiple group: ' + self.name + '/show averages', True).toBool()
		self.legendPos = self.settings.value('multiple group: ' + self.name + '/legend position', -1).toInt()[0]
		self.bShowPvalue = self.settings.value('multiple group: ' + self.name + '/show p-value', True).toBool()

	def mirrorProperties(self, plotToCopy):
		super(BarPlot, self).mirrorProperties(plotToCopy)
		
		self.figColWidth = plotToCopy.figColWidth
		self.figHeight = plotToCopy.figHeight
		
		self.fieldToPlot = plotToCopy.fieldToPlot
		self.legendPos = plotToCopy.legendPos
		self.bShowAverage = plotToCopy.bShowAverages
		
		self.bShowPvalue = plotToCopy.bShowPvalue
		
	def plot(self, profile, statsResults):
		if len(profile.profileDict) <= 0 or self.preferences['Selected multiple group feature'] == '':
			self.emptyAxis()
			return
			
		axesColour = str(self.preferences['Axes colour'].name())
		
		# *** Get data for each group
		feature = self.preferences['Selected multiple group feature']
		if self.fieldToPlot == "Number of sequences":
			data = profile.getActiveFeatureCounts(feature)
		else: # Proportion of sequences (%)
			data = profile.getActiveFeatureProportions(feature)
			
		if data == []:
			self.emptyAxis()
			return
		
		sampleNames = []
		for i in xrange(0, len(profile.activeSamplesInGroups )):
			sampleNames += profile.activeSamplesInGroups[i]

		# *** Find longest label
		bTruncate = False
		if self.preferences['Truncate feature names']:
			length = self.preferences['Length of truncated feature names']
			bTruncate = True
		
		longestLabelLen = 0
		for i in xrange(0, len(sampleNames)):
			if bTruncate and len(sampleNames[i]) > length+3:
				sampleNames[i] = sampleNames[i][0:length] + '...'
				
			if len(sampleNames[i]) > longestLabelLen:
				longestLabelLen = len(sampleNames[i])
				longestLabel = sampleNames[i]

		# *** Set figure size
		self.fig.clear()
		figWidth = self.figColWidth*len(sampleNames)
		figHeight = self.figHeight
		if figWidth > 256 or figHeight > 256:
				QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.ArrowCursor))
				self.emptyAxis()	
				QtGui.QMessageBox.question(self, 'Excessively large plot', 'The resulting plot is too large to display.')
				QtGui.QApplication.instance().restoreOverrideCursor()
				return

		self.fig.set_size_inches(figWidth, figHeight)
		
		maxValue = 0
		for i in xrange(0, len(data)):
			curMax = max(data[i])
			if curMax > maxValue:
				maxValue = curMax
		maxValueStr = '%.2f' % maxValue
		
		xLabelBounds, yLabelBounds = self.labelExtents([longestLabel], 8, 90, [maxValueStr], 8, 0)
		padding = 0.3 # inches
		newFigWidth = figWidth * (1.0+yLabelBounds.width) + 2*padding
		
		self.fig.set_size_inches(figWidth * (1.0+yLabelBounds.width) + 2*padding, figHeight)

		xOffsetFigSpace = (yLabelBounds.width*figWidth)/newFigWidth + padding/newFigWidth
		yOffsetFigSpace = xLabelBounds.height + padding/figHeight
		axesBar = self.fig.add_axes([xOffsetFigSpace, yOffsetFigSpace,
																		1.0 - xOffsetFigSpace - padding/newFigWidth, 1.0 - yOffsetFigSpace - padding/figHeight])

		# *** Plot data
		colours = []
		for groupName in profile.activeGroupNames:
			colours.append(str(self.preferences['Group colours'][groupName].name()))

		colWidth = self.figColWidth
		barWidth = (colWidth*0.9) / 2
		
		theRects = []
		start = 0
		for i in xrange(0, len(data)):
			rects = axesBar.bar(np.arange(len(data[i]))*colWidth + start*colWidth, data[i], width=barWidth, color=colours[i], zorder=5)
			theRects.append(rects[0])
			start += len(data[i])

		axesBar.set_xticks(np.arange(len(sampleNames))*colWidth + barWidth)
		axesBar.set_xlim([-0.25*colWidth, len(sampleNames)*colWidth - 0.25*colWidth])
		axesBar.set_xticklabels(sampleNames)
		
		# *** Plot average lines
		if self.bShowAverages:
			start = 0
			for i in xrange(0, len(data)):
				avgGroup = float(sum(data[i])) / len(data[i])
				axesBar.plot([start*colWidth - 0.25*colWidth, (start + len(data[i]))*colWidth - 0.25*colWidth], [avgGroup, avgGroup], color=colours[i], linestyle='-', zorder=1)
				start += len(data[i])
				
		# *** P-value label
		if self.bShowPvalue and statsResults.profile != None:
			pValueStr = statsResults.getFeatureStatisticAsStr(feature, 'pValuesCorrected')
			axesBar.text(1.0, 1.0, r'$p$ = ' + pValueStr, horizontalalignment='right', verticalalignment='bottom', transform=axesBar.transAxes)

		# *** Prettify plot
		if self.preferences['Truncate feature names']:
			length = self.preferences['Length of truncated feature names']
			if len(feature) > length+3:
					feature = feature[0:length] + '...'
					
		axesBar.set_title(feature)
		axesBar.set_ylabel(self.fieldToPlot)
		
		if self.legendPos != -1:
			legend = axesBar.legend(theRects, profile.activeGroupNames, loc=self.legendPos)
			legend.get_frame().set_linewidth(0)

		for label in axesBar.get_xticklabels():
			label.set_rotation(90)

		for a in axesBar.yaxis.majorTicks:
			a.tick1On=False
			a.tick2On=False
				
		for a in axesBar.xaxis.majorTicks:
			a.tick1On=False
			a.tick2On=False
			
		for line in axesBar.yaxis.get_ticklines(): 
			line.set_color(axesColour)
		
		for line in axesBar.xaxis.get_ticklines(): 
			line.set_color(axesColour)
			
		for loc, spine in axesBar.spines.iteritems():
			if loc in ['right','top']:
				spine.set_color('none') 
			else:
				spine.set_color(axesColour)

		self.updateGeometry()
		self.draw()

	def configure(self, profile, statsResults):
		configDlg = ConfigureDialog(Ui_BarPlotDialog)
		
		configDlg.ui.cboFieldToPlot.setCurrentIndex(configDlg.ui.cboFieldToPlot.findText(self.fieldToPlot))
				
		configDlg.ui.spinFigColWidth.setValue(self.figColWidth)
		configDlg.ui.spinFigHeight.setValue(self.figHeight)
		
		configDlg.ui.chkShowAverage.setChecked(self.bShowAverages)

		# legend position
		if self.legendPos == 0:
			configDlg.ui.radioLegendPosBest.setDown(True)
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
		
		if configDlg.exec_() == QtGui.QDialog.Accepted:	 
			self.fieldToPlot = str(configDlg.ui.cboFieldToPlot.currentText())
			
			self.figColWidth = configDlg.ui.spinFigColWidth.value()
			self.figHeight = configDlg.ui.spinFigHeight.value()
			
			self.bShowAverages = configDlg.ui.chkShowAverage.isChecked()
			
			# legend position			
			if configDlg.ui.radioLegendPosBest.isChecked() == True:
				self.legendPos = 0
			elif configDlg.ui.radioLegendPosUpperRight.isChecked() == True:
				self.legendPos = 1
			elif configDlg.ui.radioLegendPosCentreRight.isChecked() == True:
				self.legendPos = 7
			elif configDlg.ui.radioLegendPosLowerRight.isChecked() == True:
				self.legendPos = 4
			elif configDlg.ui.radioLegendPosUpperLeft.isChecked() == True:
				self.legendPos = 2
			elif configDlg.ui.radioLegendPosCentreLeft.isChecked() == True:
				self.legendPos = 6
			elif configDlg.ui.radioLegendPosLowerLeft.isChecked() == True:
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
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(ProfileBarPlots)
	testWindow.show()
	sys.exit(app.exec_())


				