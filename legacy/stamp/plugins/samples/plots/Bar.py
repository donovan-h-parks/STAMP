#=======================================================================
# Author: Donovan Parks
#
# Bar plot of different statistics.
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

from matplotlib.ticker import ScalarFormatter

from stamp.plugins.samples.AbstractSamplePlotPlugin import AbstractSamplePlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.samples.plots.configGUI.barUI import Ui_BarConfigDialog
from stamp.metagenomics import TableHelper

class Bar(AbstractSamplePlotPlugin):
	'''
	Bar plot.
	'''
	def __init__(self, preferences, parent=None):
		AbstractSamplePlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences

		self.name = 'Bar plot'
		self.type = 'Statistical'
		
		self.settings = preferences['Settings']
		self.figWidth = self.settings.value(self.name + '/width', 7.0).toDouble()[0]
		self.figHeightPerRow = self.settings.value(self.name +  '/row height', 0.2).toDouble()[0]
		self.fieldToPlot = self.settings.value(self.name +  '/field to plot', 'Proportion of sequences (%)').toString()
		self.legendPos = self.settings.value(self.name +  '/legend position', 0).toInt()[0]
		self.bSortFeatures = self.settings.value(self.name +  '/sort values', True).toBool()
		
	def mirrorProperties(self, plotToCopy):
		self.name = plotToCopy.name
		self.figWidth = plotToCopy.figWidth
		self.figHeightPerRow = plotToCopy.figHeightPerRow
		self.fieldToPlot = plotToCopy.fieldToPlot
		self.legendPos = plotToCopy.legendPos
		self.bSortFeatures = plotToCopy.bSortFeatures
		
	def plot(self, profile, statsResults):
		if len(statsResults.activeData) <= 0:
			self.emptyAxis()
			return
		
		features = statsResults.getColumn('Features')
		if len(features) > 200:
			QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.ArrowCursor))
			reply = QtGui.QMessageBox.question(self, 'Continue?', 'Profile contains ' + str(len(features)) + ' features. ' +
																		'It may take several seconds to generate this plot. We recommend filtering your profile first. ' + 
																		'Do you wish to continue?', QtGui.QMessageBox.Yes, QtGui.QMessageBox.No)
			QtGui.QApplication.instance().restoreOverrideCursor()
			if reply == QtGui.QMessageBox.No:
				self.emptyAxis()
				return

		# *** Colour of plot elements
		axesColour = str(self.preferences['Axes colour'].name())
		profile1Colour = str(self.preferences['Sample 1 colour'].name())
		profile2Colour = str(self.preferences['Sample 2 colour'].name())
		
		# *** Set sample names
		bLogScale = False
		xLabel = self.fieldToPlot
		
		# *** Create lists for each quantity of interest
		if self.fieldToPlot == 'Number of sequences':
			if self.bSortFeatures:
				statsResults.activeData = TableHelper.SortTable(statsResults.activeData,\
																												[statsResults.dataHeadings['Seq1']], True)
			field1 = statsResults.getColumn('Seq1')
			field2 = statsResults.getColumn('Seq2')
			

		elif self.fieldToPlot == 'Number of parental sequences':
			if self.bSortFeatures:
				statsResults.activeData = TableHelper.SortTable(statsResults.activeData,\
																												[statsResults.dataHeadings['ParentalSeq1']], True)
			field1 = statsResults.getColumn('ParentalSeq1')
			field2 = statsResults.getColumn('ParentalSeq2')
			
		elif self.fieldToPlot == 'Proportion of sequences (%)':
			if self.bSortFeatures:
				statsResults.activeData = TableHelper.SortTable(statsResults.activeData,\
																												[statsResults.dataHeadings['RelFreq1']], True)
			field1 = statsResults.getColumn('RelFreq1')
			field2 = statsResults.getColumn('RelFreq2')
			
		elif self.fieldToPlot == 'p-values':
			if self.bSortFeatures:
				statsResults.activeData = TableHelper.SortTable(statsResults.activeData,\
																												[statsResults.dataHeadings['pValues']], False)
			field1 = statsResults.getColumn('pValues')
			field2 = None
			
		elif self.fieldToPlot == 'p-values (corrected)':
			if self.bSortFeatures:
				statsResults.activeData = TableHelper.SortTable(statsResults.activeData,\
																												[statsResults.dataHeadings['pValuesCorrected']], False)
			field1 = statsResults.getColumn('pValuesCorrected')
			field2 = None
						
		elif self.fieldToPlot == 'Effect size':
			if self.bSortFeatures:
				statsResults.activeData = TableHelper.SortTable(statsResults.activeData,\
																												[statsResults.dataHeadings['EffectSize']], True, True,
																												statsResults.confIntervMethod.bRatio)
			field1 = statsResults.getColumn('EffectSize')
			field2 = None
			
			bLogScale = statsResults.confIntervMethod.bRatio
			xLabel = statsResults.confIntervMethod.plotLabel
			
		features = statsResults.getColumn('Features')	# get sorted feature labels

		# *** Truncate feature labels
		highlightedFeatures = list(self.preferences['Highlighted sample features'])
		if self.preferences['Truncate feature names']:
			length = self.preferences['Length of truncated feature names']
			
			for i in xrange(0, len(features)):
				if len(features[i]) > length+3:
					features[i] = features[i][0:length] + '...'

			for i in xrange(0, len(highlightedFeatures)):
				if len(highlightedFeatures[i]) > length+3:
					highlightedFeatures[i] = highlightedFeatures[i][0:length] + '...'
						
		# *** Check that there is at least one significant feature
		if len(features) <= 0:
			self.emptyAxis('No significant features')			
			return

		# *** Set figure size
		padding = 0.2							 #inches
		heightBottomLabels = 0.4		# inches
		
		imageHeight = len(features)*self.figHeightPerRow + padding + heightBottomLabels
		self.fig.set_size_inches(self.figWidth, imageHeight)	
							
		yPlotOffsetFigSpace = heightBottomLabels / imageHeight 
		heightPlotFigSpace = 1.0 - yPlotOffsetFigSpace - padding / imageHeight
			 
		yLabelBounds = self.yLabelExtents(features, 8)
		xPlotOffsetFigSpace = yLabelBounds.width + 0.1 / self.figWidth
		widthPlotFigSpace = 1.0 - xPlotOffsetFigSpace - padding / self.figWidth
		
		axesBar = self.fig.add_axes([xPlotOffsetFigSpace,yPlotOffsetFigSpace,widthPlotFigSpace,heightPlotFigSpace])
		
		# *** Plot data
		barHeight = 0.35 
		
		if bLogScale:
			field1 = np.log10(field1)
			xLabel = 'log(' + xLabel + ')'
			if field2 != None:
				field2 = np.log10(field2)
		
		if field2 == None:
			rects1 = axesBar.barh(np.arange(len(features)), field1, height=barHeight)	
			axesBar.set_yticks(np.arange(len(features)) + 0.5*barHeight)		
			axesBar.set_ylim([0, len(features)-1.0 + barHeight + 0.1])			
		elif field2 != None:
			rects2 = axesBar.barh(np.arange(len(features)), field2, height=barHeight, color=profile2Colour)	
			rects1 = axesBar.barh(np.arange(len(features))+barHeight, field1, height=barHeight, color=profile1Colour)
			axesBar.set_yticks(np.arange(len(features)) + barHeight)		
			axesBar.set_ylim([0, len(features)-1.0 + 2*barHeight + 0.1])			 
				
		axesBar.set_yticklabels(features)	
		axesBar.set_xlabel(xLabel)
		
		scalarFormatter = ScalarFormatter(useMathText=False)
		scalarFormatter.set_scientific(True)
		scalarFormatter.set_powerlimits((-3,4))
		axesBar.xaxis.set_major_formatter(scalarFormatter)

		# *** Prettify plot
		if self.legendPos != -1 and field2 != None:
			legend = axesBar.legend([rects1[0], rects2[0]], (profile.sampleNames[0], profile.sampleNames[1]), loc=self.legendPos)
			legend.get_frame().set_linewidth(0)
				
		for label in axesBar.get_yticklabels():
			if label.get_text() in highlightedFeatures:
					label.set_color('red')
			
		for a in axesBar.yaxis.majorTicks:
			a.tick1On=False
			a.tick2On=False
				
		for a in axesBar.xaxis.majorTicks:
			a.tick1On=True
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
		self.statsResults = statsResults
		
		configDlg = ConfigureDialog(Ui_BarConfigDialog)
		
		configDlg.ui.cboFieldToPlot.clear()
		configDlg.ui.cboFieldToPlot.addItem('Effect size')
		configDlg.ui.cboFieldToPlot.addItem('Number of sequences')
		configDlg.ui.cboFieldToPlot.addItem('Number of parental sequences')
		configDlg.ui.cboFieldToPlot.addItem('p-values')
		configDlg.ui.cboFieldToPlot.addItem('p-values (corrected)')
		configDlg.ui.cboFieldToPlot.addItem('Proportion of sequences (%)')
		
		configDlg.ui.cboFieldToPlot.setCurrentIndex(configDlg.ui.cboFieldToPlot.findText(self.fieldToPlot))
		
		configDlg.ui.chkSort.setChecked(self.bSortFeatures)
		
		configDlg.ui.spinFigWidth.setValue(self.figWidth)
		configDlg.ui.spinFigRowHeight.setValue(self.figHeightPerRow)
		
		# legend position
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
		
		if configDlg.exec_() == QtGui.QDialog.Accepted:
			self.fieldToPlot = str(configDlg.ui.cboFieldToPlot.currentText())
			self.bSortFeatures = configDlg.ui.chkSort.isChecked()
			self.figWidth = configDlg.ui.spinFigWidth.value()
			self.figHeightPerRow = configDlg.ui.spinFigRowHeight.value()
			
			# legend position
			self.bShowLegend = True
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
				
			self.settings.setValue(self.name + '/width', self.figWidth)
			self.settings.setValue(self.name + '/row height', self.figHeightPerRow)
			self.settings.setValue(self.name + '/field to plot', self.fieldToPlot)
			self.settings.setValue(self.name + '/legend position', self.legendPos)
			self.settings.setValue(self.name + '/sort values', self.bSortFeatures)

			self.plot(profile, statsResults)
					
if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(Bar)
	testWindow.show()
	sys.exit(app.exec_())


				