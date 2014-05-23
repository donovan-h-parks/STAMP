#=======================================================================
# Author: Donovan Parks
#
# Bar plot for two groups.
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

from PyQt4 import QtGui, QtCore
import numpy as np

from stamp.plugins.groups.AbstractGroupPlotPlugin import AbstractGroupPlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.groups.plots.configGUI.BarPlotUI import Ui_BarPlotDialog

from stamp.metagenomics.stats.CI.WilsonCI import WilsonCI

class BarPlot(AbstractGroupPlotPlugin):
	'''
	Bar plot for two groups.
	'''
	def __init__(self, preferences, parent=None):
		AbstractGroupPlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences

		self.name = 'Bar plot'
		self.type = 'Exploratory'

		self.settings = preferences['Settings']
		self.figColWidth = self.settings.value('group: ' + self.name + '/column width', 0.2).toDouble()[0]
		self.figHeight = self.settings.value('group: ' + self.name + '/height', 6.0).toDouble()[0]
		self.fieldToPlot = self.settings.value('group: ' + self.name + '/field to plot', 'Proportion of sequences (%)').toString()
		self.bShowAverages = self.settings.value('group: ' + self.name + '/show averages', True).toBool()
		self.legendPos = self.settings.value('group: ' + self.name + '/legend position', -1).toInt()[0]
		self.bShowPvalue = self.settings.value('group: ' + self.name + '/show p-value', True).toBool()

	def mirrorProperties(self, plotToCopy):
		super(BarPlot, self).mirrorProperties(plotToCopy)
		
		self.figColWidth = plotToCopy.figColWidth
		self.figHeight = plotToCopy.figHeight
		
		self.fieldToPlot = plotToCopy.fieldToPlot
		self.legendPos = plotToCopy.legendPos
		self.bShowAverage = plotToCopy.bShowAverages
		
		self.bShowPvalue = plotToCopy.bShowPvalue
		
	def plot(self, profile, statsResults):
		if len(profile.profileDict) <= 0 or self.preferences['Selected group feature'] == '' or len(profile.samplesInGroup1) == 0 or len(profile.samplesInGroup2) == 0:
			self.emptyAxis()
			return
				
		# *** Colour of plot elements
		axesColour = str(self.preferences['Axes colour'].name())
		group1Colour = str(self.preferences['Group colours'][profile.groupName1].name())
		group2Colour = str(self.preferences['Group colours'][profile.groupName2].name())
		
		# *** Get data for each group
		feature = self.preferences['Selected group feature']
		if self.fieldToPlot == "Number of sequences":
			data1, data2 = profile.getFeatureCounts(feature)
		else: # Proportion of sequences (%)
			data1, data2 = profile.getFeatureProportions(feature)
		
		sampleNames = profile.samplesInGroup1 + profile.samplesInGroup2

		# *** Find longest label
		bTruncate = False
		if self.preferences['Truncate feature names']:
			length = self.preferences['Length of truncated feature names']
			bTruncate = True
		
		longestLabelLen = 0
		longestLabel = ''
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
		xLabelBounds, yLabelBounds = self.labelExtents([longestLabel], 8, 90, [max(max(data1), max(data2))], 8, 0)
		padding = 0.25 # inches
		newFigWidth = figWidth * (1.0+yLabelBounds.width) + 2*padding
		
		self.fig.set_size_inches(figWidth * (1.0+yLabelBounds.width) + 2*padding, figHeight)

		xOffsetFigSpace = (yLabelBounds.width*figWidth)/newFigWidth + padding/newFigWidth
		yOffsetFigSpace = xLabelBounds.height + padding/figHeight
		axesBar = self.fig.add_axes([xOffsetFigSpace, yOffsetFigSpace,
																		1.0 - xOffsetFigSpace - padding/newFigWidth, 1.0 - yOffsetFigSpace - padding/figHeight])

		# *** Plot data
		colWidth = self.figColWidth
		barWidth = (colWidth*0.9) / 2
		
		rects1 = axesBar.bar(np.arange(len(data1))*colWidth, data1, width=barWidth, color=group1Colour, zorder=5)
		rects2 = axesBar.bar(np.arange(len(data2))*colWidth + len(data1)*colWidth, data2, width=barWidth, color=group2Colour, zorder=5)

		axesBar.set_xticks(np.arange(len(sampleNames))*colWidth + barWidth)
		axesBar.set_xlim([-0.25*colWidth, len(sampleNames)*colWidth - 0.25*colWidth])
		axesBar.set_xticklabels(sampleNames)
		
		# *** Plot average lines
		if self.bShowAverages:
			avgGroup1 = float(sum(data1)) / len(data1)
			avgGroup2 = float(sum(data2)) / len(data2)
			
			axesBar.plot([-0.25*colWidth, len(data1)*colWidth - 0.25*colWidth], [avgGroup1, avgGroup1], color=group1Colour, linestyle='-', zorder=1)
			axesBar.plot([len(data1)*colWidth - 0.25*colWidth, len(sampleNames)*colWidth - 0.25*colWidth], [avgGroup2, avgGroup2], color=group2Colour, linestyle='-', zorder=1)
			
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
			legend = axesBar.legend([rects1[0], rects2[0]], (profile.groupName1, profile.groupName2), loc=self.legendPos)
			legend.get_frame().set_linewidth(0)

		for label in axesBar.get_xticklabels():
			label.set_rotation(90)

		for a in axesBar.yaxis.majorTicks:
			a.tick1On=False
			a.tick2On=False
				
		for a in axesBar.xaxis.majorTicks:
			a.tick1On=False
			a.tick2On=False
			
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
				
			self.settings.setValue('group: ' + self.name + '/column width', self.figColWidth)
			self.settings.setValue('group: ' + self.name + '/height', self.figHeight)
			self.settings.setValue('group: ' + self.name + '/field to plot', self.fieldToPlot)
			self.settings.setValue('group: ' + self.name + '/show averages', self.bShowAverages)
			self.settings.setValue('group: ' + self.name + '/legend position', self.legendPos)
			self.settings.setValue('group: ' + self.name + '/show p-value', self.bShowPvalue)
			
			self.plot(profile, statsResults)
					
if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(ProfileBarPlots)
	testWindow.show()
	sys.exit(app.exec_())


				