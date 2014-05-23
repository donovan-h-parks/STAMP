#=======================================================================
# Author: Donovan Parks
#
# p-value histogram plot.
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

from mpl_toolkits.axes_grid.inset_locator import inset_axes

from stamp.plugins.samples.AbstractSamplePlotPlugin import AbstractSamplePlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.samples.plots.configGUI.pValueHistogramUI import Ui_pValueHistogramDialog

class pValueHistogram(AbstractSamplePlotPlugin):
	'''
	p-value histogram plot.
	'''
	def __init__(self, preferences, parent=None):
		AbstractSamplePlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences
		
		self.name = 'p-value histogram'
		self.type = 'Statistical'
		
		self.settings = preferences['Settings']		
		self.figWidth = self.settings.value(self.name + '/width', 7.0).toDouble()[0]
		self.figHeight = self.settings.value(self.name + '/height', 7.0).toDouble()[0]
		self.fieldToPlot = self.settings.value(self.name + '/field to plot', 'p-values (corrected)').toString()
		self.yAxisLogScale = self.settings.value(self.name + '/histogram log scale', False).toBool()
		self.binWidth = self.settings.value(self.name + '/histogram bin width', 0.01).toDouble()[0]
		self.bShowInset = self.settings.value(self.name + '/show inset', True).toBool()
		self.insetWidth = self.settings.value(self.name + '/inset width %', 60.0).toDouble()[0]
		self.insetHeight = self.settings.value(self.name + '/inset height %', 60.0).toDouble()[0]
		self.insetLogScale = self.settings.value(self.name + '/inset log scale', False).toBool()
		self.insetBinWidth = self.settings.value(self.name + '/inset bin width', 0.002).toDouble()[0]
		self.xLimit = self.settings.value(self.name + '/inset x-axis limit', 0.05).toDouble()[0]

	def mirrorProperties(self, plotToCopy):
		self.name = plotToCopy.name
		self.figWidth = plotToCopy.figWidth
		self.figHeight = plotToCopy.figHeight
		self.yAxisLogScale = plotToCopy.yAxisLogScale
		self.fieldToPlot = plotToCopy.fieldToPlot
		
		self.bShowInset = plotToCopy.bShowInset
		self.insetWidth = plotToCopy.insetWidth
		self.insetHeight = plotToCopy.insetHeight
		self.insetBinWidth = plotToCopy.insetBinWidth
		self.xLimit = plotToCopy.xLimit
		self.insetLogScale = plotToCopy.insetLogScale
		
	def plot(self, profile, statsResults):
		if len(statsResults.activeData) <= 0:
			self.emptyAxis()			
			return
			
		axesColour = str(self.preferences['Axes colour'].name())
		
		# *** Get data to plot 
		if self.fieldToPlot == 'p-values':
			data = statsResults.getColumn('pValues')
			xLabel = 'p-value'
		elif self.fieldToPlot == 'p-values (corrected)':
			data = statsResults.getColumn('pValuesCorrected')
			xLabel = 'p-value (corrected)'
		
		# *** Set size of figure
		self.fig.clear()
		self.fig.set_size_inches(self.figWidth, self.figHeight) 
		heightBottomLabels = 0.4	# inches
		widthSideLabel = 0.5			# inches 
		padding = 0.2						 # inches
		axesHist = self.fig.add_axes([widthSideLabel/self.figWidth,heightBottomLabels/self.figHeight,\
																		1.0-(widthSideLabel+padding)/self.figWidth,\
																		1.0-(heightBottomLabels+padding)/self.figHeight])
		
		# *** Histogram plot 
		bins = [0.0]
		binWidth = self.binWidth
		binEnd = binWidth
		while binEnd <= 1.0:
			bins.append(binEnd)
			binEnd += binWidth
			

		n, bins, patch = axesHist.hist(data, bins=bins, log=self.yAxisLogScale,color=(0.5,0.5,0.5))	
		axesHist.set_xlabel(xLabel)
		axesHist.set_ylabel('Number of features')
			

		# *** Prettify plot	 
		for a in axesHist.yaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
				
		for a in axesHist.xaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
			
		for line in axesHist.yaxis.get_ticklines(): 
			line.set_color(axesColour)
				
		for line in axesHist.xaxis.get_ticklines(): 
			line.set_color(axesColour)
			
		for loc, spine in axesHist.spines.iteritems():
			if loc in ['right','top']:
				spine.set_color('none') 
			else:
				spine.set_color(axesColour)
			
		# *** Plot inset		
		if self.bShowInset:
			bins = [0.0]
			binWidth = self.insetBinWidth
			binEnd = binWidth
			while binEnd <= self.xLimit:
				bins.append(binEnd)
				binEnd += binWidth
				
			widthStr = str(self.insetWidth) + '%'
			heightStr = str(self.insetHeight) + '%'
			axins = inset_axes(axesHist, width=widthStr, height=heightStr, loc=1)
			filteredData = [d for d in data if d <= self.xLimit]
			if filteredData:
				n, bins, patch = axins.hist(filteredData, bins=bins, log=self.insetLogScale, color=(0.5,0.5,0.5))	
			axins.set_xlim(0, self.xLimit)
			
			# *** Prettify inset	 
			for a in axins.yaxis.majorTicks:
				a.tick1On=True
				a.tick2On=False
					
			for a in axins.xaxis.majorTicks:
				a.tick1On=True
				a.tick2On=False
				
			for line in axins.yaxis.get_ticklines(): 
				line.set_color(axesColour)
				
			for line in axins.xaxis.get_ticklines(): 
				line.set_color(axesColour)
				
			for loc, spine in axins.spines.iteritems():
				if loc in ['right','top']:
					spine.set_color('none') 
				else:
						spine.set_color(axesColour)
				
		self.updateGeometry()			 
		self.draw()
	
	def configure(self, profile, statsResults):
		self.statsResults = statsResults
		
		self.configDlg = ConfigureDialog(Ui_pValueHistogramDialog)
		
		self.connect(self.configDlg.ui.btnXmax, QtCore.SIGNAL('clicked()'), self.setXaxisMax)
		
		self.configDlg.ui.cboFieldToPlot.setCurrentIndex(self.configDlg.ui.cboFieldToPlot.findText(self.fieldToPlot))
		
		self.configDlg.ui.spinFigWidth.setValue(self.figWidth)
		self.configDlg.ui.spinFigHeight.setValue(self.figHeight)
		
		self.configDlg.ui.spinBinWidth.setValue(self.binWidth)		 
		self.configDlg.ui.chkLogScale.setChecked(self.yAxisLogScale)
		
		self.configDlg.ui.chkShowInset.setChecked(self.bShowInset)
		self.configDlg.ui.spinInsetWidth.setValue(self.insetWidth)
		self.configDlg.ui.spinInsetHeight.setValue(self.insetHeight)
		self.configDlg.ui.spinInsetBinWidth.setValue(self.insetBinWidth)
		self.configDlg.ui.spinXlimit.setValue(self.xLimit)
		self.configDlg.ui.chkInsetLogScale.setChecked(self.insetLogScale)
		
		if self.configDlg.exec_() == QtGui.QDialog.Accepted:		 
			self.figWidth = self.configDlg.ui.spinFigWidth.value()
			self.figHeight = self.configDlg.ui.spinFigHeight.value()

			self.binWidth = self.configDlg.ui.spinBinWidth.value()
			self.yAxisLogScale = self.configDlg.ui.chkLogScale.isChecked()
			
			self.fieldToPlot = self.configDlg.ui.cboFieldToPlot.currentText()
			
			self.bShowInset = self.configDlg.ui.chkShowInset.isChecked()
			self.insetWidth = self.configDlg.ui.spinInsetWidth.value()
			self.insetHeight = self.configDlg.ui.spinInsetHeight.value()
			self.insetBinWidth = self.configDlg.ui.spinInsetBinWidth.value()
			self.xLimit = self.configDlg.ui.spinXlimit.value()
			self.insetLogScale = self.configDlg.ui.chkInsetLogScale.isChecked()
			
			self.settings.setValue(self.name + '/width', self.figWidth)
			self.settings.setValue(self.name + '/height', self.figHeight)
			self.settings.setValue(self.name + '/field to plot', self.fieldToPlot)
			self.settings.setValue(self.name + '/histogram log scale', self.yAxisLogScale)
			self.settings.setValue(self.name + '/histogram bin width', self.binWidth)
			self.settings.setValue(self.name + '/show inset', self.bShowInset)
			self.settings.setValue(self.name + '/inset width %', self.insetWidth)
			self.settings.setValue(self.name + '/inset height %', self.insetHeight)
			self.settings.setValue(self.name + '/inset log scale', self.insetLogScale)
			self.settings.setValue(self.name + '/inset bin width', self.insetBinWidth)
			self.settings.setValue(self.name + '/inset x-axis limit', self.xLimit)
			
			self.plot(profile, statsResults)
			
	def setXaxisMax(self):
		# *** Get data to plot 
		if self.configDlg.ui.cboFieldToPlot.currentText() == 'p-values':
			data = self.statsResults.getColumn('pValues')
		else:
			data = self.statsResults.getColumn('pValuesCorrected')
			
		self.configDlg.ui.spinXlimit.setValue(max(data))

if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(pValueHistogram)
	testWindow.show()
	sys.exit(app.exec_())


				