#=======================================================================
# Author: Donovan Parks
#
# Set of four plots useful for investigating the effects of a multiple comparison
# correct method (applicable to both familywise error or false discory rate methods).
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

from PyQt4 import QtCore, QtGui
import numpy as np

from stamp.plugins.samples.AbstractSamplePlotPlugin import AbstractSamplePlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.samples.plots.configGUI.multCompCorrectionUI import Ui_MultCompCorrectionDialog

class MultCompCorrectionPlots(AbstractSamplePlotPlugin):
	'''
	Sequence histogram plot.
	'''
	def __init__(self, preferences, parent=None):
		AbstractSamplePlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences
		
		self.name = 'Multiple comparison plots'
		self.type = 'Statistical'
		
		self.settings = preferences['Settings']		
		self.figWidth = self.settings.value(self.name + '/width', 7.0).toDouble()[0]
		self.figHeight = self.settings.value(self.name + '/height', 3.5).toDouble()[0]
		self.yAxisLogScale = self.settings.value(self.name + '/histogram log scale', False).toBool()
		self.binWidth = self.settings.value(self.name + '/bin width', 0.01).toDouble()[0]
		self.xLimitFig1 = self.settings.value(self.name + '/histogram x-axis limit', 0.1).toDouble()[0]
		self.xLimitFig2 = self.settings.value(self.name + '/correction plot x-axis limit', 1.0).toDouble()[0]
		self.xLimitFig3 = self.settings.value(self.name + '/significant features x-axis limit', 0.1).toDouble()[0]
		
		self.xMax = 0

	def mirrorProperties(self, plotToCopy):
		self.name = plotToCopy.name
		self.figWidth = plotToCopy.figWidth
		self.figHeight = plotToCopy.figHeight
		
		self.yAxisLogScale = plotToCopy.yAxisLogScale
		self.binWidth = plotToCopy.binWidth
				
		self.xLimitFig1 = plotToCopy.xLimitFig1
		self.xLimitFig2 = plotToCopy.xLimitFig2
		self.xLimitFig3 = plotToCopy.xLimitFig3

		self.xMax = plotToCopy.xMax
			 
	def plot(self, profile, statsResults):
		if len(statsResults.activeData) <= 0:
			self.emptyAxis()			
			return
			
		axesColour = str(self.preferences['Axes colour'].name())
		
		# *** Get p-values and effect size
		rawValues = statsResults.getColumn('pValues')
		correctedValues = statsResults.getColumn('pValuesCorrected')
		
		# *** Get labels
		if statsResults.multCompCorrection.method == 'False discovery rate':
			xLabel = 'q-value'
			yLabel = 'p-value'
		else:
			xLabel = 'corrected p-value'
			yLabel = 'raw p-value'
		
		# *** Set size of figure
		self.fig.clear()
		self.fig.set_size_inches(self.figWidth, self.figHeight) 
		
		self.fig.subplots_adjust(wspace = 0.4)
		self.fig.subplots_adjust(bottom = 0.15)
		self.fig.subplots_adjust(top = 0.95)
		self.fig.subplots_adjust(left = 0.07)
		self.fig.subplots_adjust(right = 0.97)
				
		# Sort p-values
		pValues = zip(correctedValues, rawValues)
		pValues.sort()
		correctedValues = [pValue[0] for pValue in pValues]
		rawValues = [pValue[1] for pValue in pValues]
		
		self.xMax = max(correctedValues)
		
		# Plot effect size vs. corrected p-value
		ax1 = self.fig.add_subplot(1,3,1)
		
		bins = [0.0]
		binWidth = self.binWidth
		binEnd = binWidth
		while binEnd <= self.xLimitFig1:
			bins.append(binEnd)
			binEnd += binWidth
			
		x = [value for value in correctedValues if value <= self.xLimitFig1]
		if x:
			n, bins, patch = ax1.hist(x, bins=bins, log=self.yAxisLogScale, color=(0.5,0.5,0.5))
		ax1.set_xlabel(xLabel)
		ax1.set_ylabel('Number of features')
		ax1.set_xlim(0, self.xLimitFig1)
		
		for label in ax1.get_xticklabels():
			label.set_rotation(90)
			
		for a in ax1.yaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
				
		for a in ax1.xaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
			
		for line in ax1.yaxis.get_ticklines(): 
			line.set_color(axesColour)
				
		for line in ax1.xaxis.get_ticklines(): 
			line.set_color(axesColour)
			
		for loc, spine in ax1.spines.iteritems():
			if loc in ['right','top']:
				spine.set_color('none') 
			else:
				spine.set_color(axesColour)
					
		# Plot corrected p-values vs raw p-values
		ax2 = self.fig.add_subplot(1,3,2)
		
		x = [value for value in correctedValues if value <= self.xLimitFig2]
		y = rawValues[0:len(x)]
		
		ax2.plot(x,y,'o', markersize=4, mfc=(0.5,0.5,0.5))
		ax2.set_xlabel(xLabel)
		ax2.set_ylabel(yLabel)
		ax2.set_xlim(0, self.xLimitFig2)
		
		for label in ax2.get_xticklabels():
			label.set_rotation(90)
			
		for a in ax2.yaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
				
		for a in ax2.xaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
			
		for line in ax2.yaxis.get_ticklines(): 
			line.set_color(axesColour)
				
		for line in ax2.xaxis.get_ticklines(): 
			line.set_color(axesColour)
			
		for loc, spine in ax2.spines.iteritems():
			if loc in ['right','top']:
				spine.set_color('none') 
			else:
				spine.set_color(axesColour)
		
		# Plot corrected p-values vs. number of significant features		
		ax3 = self.fig.add_subplot(1,3,3)
		
		x = [value for value in correctedValues if value <= self.xLimitFig3]
		x = correctedValues[0:len(x)+1] # add in one extra point so the plot fills the entire x-axis
		
		ax3.plot(x, np.arange(len(x)),'k-')
		ax3.set_xlabel(xLabel)
		ax3.set_ylabel('# significant features')
		ax3.set_xlim(0, self.xLimitFig3)
		
		for label in ax3.get_xticklabels():
			label.set_rotation(90)
			
		for a in ax3.yaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
				
		for a in ax3.xaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
			
		for line in ax3.yaxis.get_ticklines(): 
			line.set_color(axesColour)
				
		for line in ax3.xaxis.get_ticklines(): 
			line.set_color(axesColour)
			
		for loc, spine in ax3.spines.iteritems():
			if loc in ['right','top']:
				spine.set_color('none') 
			else:
				spine.set_color(axesColour)
								
		self.updateGeometry()			 
		self.draw()
	
	def configure(self, profile, statsResults):			 
		self.configDlg = ConfigureDialog(Ui_MultCompCorrectionDialog)
		
		self.connect(self.configDlg.ui.btnXmaxFig1, QtCore.SIGNAL('clicked()'), self.setXaxisMax1)
		self.connect(self.configDlg.ui.btnXmaxFig2, QtCore.SIGNAL('clicked()'), self.setXaxisMax2)
		self.connect(self.configDlg.ui.btnXmaxFig3, QtCore.SIGNAL('clicked()'), self.setXaxisMax3)
		
		self.configDlg.ui.spinFigWidth.setValue(self.figWidth)
		self.configDlg.ui.spinFigHeight.setValue(self.figHeight)
		
		self.configDlg.ui.spinBinWidth.setValue(self.binWidth)		 
		self.configDlg.ui.chkLogScale.setChecked(self.yAxisLogScale)
		
		self.configDlg.ui.spinXlimitFig1.setValue(self.xLimitFig1)		
		self.configDlg.ui.spinXlimitFig2.setValue(self.xLimitFig2)		
		self.configDlg.ui.spinXlimitFig3.setValue(self.xLimitFig3)
				
		if self.configDlg.exec_() == QtGui.QDialog.Accepted:					
			self.figWidth = self.configDlg.ui.spinFigWidth.value()
			self.figHeight = self.configDlg.ui.spinFigHeight.value()

			self.binWidth = self.configDlg.ui.spinBinWidth.value()
			self.yAxisLogScale = self.configDlg.ui.chkLogScale.isChecked()
			
			self.xLimitFig1 = self.configDlg.ui.spinXlimitFig1.value()
			self.xLimitFig2 = self.configDlg.ui.spinXlimitFig2.value()			
			self.xLimitFig3 = self.configDlg.ui.spinXlimitFig3.value()
			
			self.settings.setValue(self.name + '/width', self.figWidth)
			self.settings.setValue(self.name + '/height', self.figHeight)
			self.settings.setValue(self.name + '/histogram log scale', self.yAxisLogScale)
			self.settings.setValue(self.name + '/bin width', self.binWidth)
			self.settings.setValue(self.name + '/histogram x-axis limit', self.xLimitFig1)
			self.settings.setValue(self.name + '/correction plot x-axis limit', self.xLimitFig2)
			self.settings.setValue(self.name + '/significant features x-axis limit', self.xLimitFig3)
			
			self.plot(profile, statsResults)
			
	def setXaxisMax1(self):
		self.configDlg.ui.spinXlimitFig1.setValue(self.xMax)
		
	def setXaxisMax2(self):
		self.configDlg.ui.spinXlimitFig2.setValue(self.xMax)
		
	def setXaxisMax3(self):
		self.configDlg.ui.spinXlimitFig3.setValue(self.xMax)

if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(MultCompCorrectionPlots)
	testWindow.show()
	sys.exit(app.exec_())


				