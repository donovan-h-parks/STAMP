#=======================================================================
# Author: Donovan Parks
#
# Sequence histogram plot.
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

from PyQt4 import QtCore, QtGui

from stamp.plugins.samples.AbstractSamplePlotPlugin import AbstractSamplePlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.samples.plots.configGUI.seqHistogramUI import Ui_SeqHistogramDialog

class SeqHistogram(AbstractSamplePlotPlugin):
	'''
	Sequence histogram plot.
	'''
	def __init__(self, preferences, parent=None):
		AbstractSamplePlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences
		
		self.name = 'Sequence histogram'
		self.type = 'Exploratory'
		
		self.settings = preferences['Settings']
		self.figWidth = self.settings.value(self.name + '/width', 7.0).toDouble()[0]
		self.figHeight = self.settings.value(self.name + '/height', 7.0).toDouble()[0]
		self.bCustomBinWidth = self.settings.value(self.name + '/custom bin width', False).toBool()
		self.binWidth = self.settings.value(self.name + '/bin width', 100.0).toDouble()[0]
		self.yAxisLogScale = self.settings.value(self.name + '/log scale', False).toBool()
		self.bCustomXaxis = self.settings.value(self.name + '/custom x-axis extents', False).toBool()
		self.xLimitLeft = self.settings.value(self.name + '/min value', 0.0).toDouble()[0]
		self.xLimitRight = self.settings.value(self.name + '/max value', 1.0).toDouble()[0]
		self.legendPos = self.settings.value(self.name + '/legend position', 0).toInt()[0]
		
	def mirrorProperties(self, plotToCopy):
		self.name = plotToCopy.name
		self.figWidth = plotToCopy.figWidth
		self.figHeight = plotToCopy.figHeight

		self.bCustomBinWidth = plotToCopy.bCustomBinWidth
		self.binWidth = plotToCopy.binWidth
		self.yAxisLogScale = plotToCopy.yAxisLogScale
		
		self.bCustomXaxis = plotToCopy.bCustomXaxis
		self.xLimitLeft = plotToCopy.xLimitLeft
		self.xLimitRight = plotToCopy.xLimitRight
		
		self.legendPos = plotToCopy.legendPos
		
	def plot(self, profile, statsResults):
		if len(profile.profileDict) <= 0:
			self.emptyAxis()
			return
		
		# *** Colour of plot elements
		axesColour = str(self.preferences['Axes colour'].name())
		profile1Colour = str(self.preferences['Sample 1 colour'].name())
		profile2Colour = str(self.preferences['Sample 2 colour'].name()) 
		
		# *** Get sequence counts
		seqs1 = profile.getSequenceCounts(0)
		seqs2 = profile.getSequenceCounts(1)
		
		# *** Set x-axis limit
		self.xMin = min(min(seqs1),min(seqs2))
		if self.xLimitLeft == None:
			self.xLimitLeft = self.xMin
			
		self.xMax = max(max(seqs1),max(seqs2))
		if self.xLimitRight == None:
			self.xLimitRight = self.xMax
			
		# Set bin width
		if not self.bCustomBinWidth:
			self.binWidth = (self.xMax - self.xMin) / 40
		
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
		bins = [0]
		binEnd = self.binWidth
		while binEnd <= self.xMax:
			bins.append(binEnd)
			binEnd += self.binWidth
		bins.append(binEnd)

		n, b, patches = axesHist.hist([seqs1, seqs2], bins=bins, log=self.yAxisLogScale)
		for patch in patches[0]:
			patch.set_facecolor(profile1Colour)
		for patch in patches[1]:
			patch.set_facecolor(profile2Colour)

		if self.bCustomXaxis:
			axesHist.set_xlim(self.xLimitLeft, self.xLimitRight)

		axesHist.set_xlabel('Sequences')
		axesHist.set_ylabel('Number of features')
		
		# *** Prettify plot
		if self.legendPos != -1:
			legend = axesHist.legend([patches[0][0], patches[1][0]], (profile.sampleNames[0], profile.sampleNames[1]), loc=self.legendPos)
			legend.get_frame().set_linewidth(0)
			
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
		
		self.updateGeometry()			 
		self.draw()
	
	def configure(self, profile, statsResults):	
		self.profile = profile
		
		self.configDlg = ConfigureDialog(Ui_SeqHistogramDialog)
		
		self.connect(self.configDlg.ui.chkCustomBinWidth, QtCore.SIGNAL('toggled(bool)'), self.changeCustomBinWidth)
		self.connect(self.configDlg.ui.chkCustomXaxis, QtCore.SIGNAL('toggled(bool)'), self.changeCustomXaxis)
		self.connect(self.configDlg.ui.btnXmin, QtCore.SIGNAL('clicked()'), self.setXaxisMin)
		self.connect(self.configDlg.ui.btnXmax, QtCore.SIGNAL('clicked()'), self.setXaxisMax)

		self.configDlg.ui.spinFigWidth.setValue(self.figWidth)
		self.configDlg.ui.spinFigHeight.setValue(self.figHeight)
		
		self.configDlg.ui.chkCustomBinWidth.setChecked(self.bCustomBinWidth)
		self.configDlg.ui.spinBinWidth.setValue(self.binWidth)		 
		self.configDlg.ui.chkLogScale.setChecked(self.yAxisLogScale)
		
		self.configDlg.ui.chkCustomXaxis.setChecked(self.bCustomXaxis)
		self.configDlg.ui.spinXmin.setValue(self.xLimitLeft)
		self.configDlg.ui.spinXmax.setValue(self.xLimitRight)
		
		self.changeCustomBinWidth()
		self.changeCustomXaxis()
		
		# legend position
		if self.legendPos == 0:
			self.configDlg.ui.radioLegendPosBest.setDown(True)
		elif self.legendPos == 1:
			self.configDlg.ui.radioLegendPosUpperRight.setChecked(True)
		elif self.legendPos == 7:
			self.configDlg.ui.radioLegendPosCentreRight.setChecked(True)
		elif self.legendPos == 4:
			self.configDlg.ui.radioLegendPosLowerRight.setChecked(True)
		elif self.legendPos == 2:
			self.configDlg.ui.radioLegendPosUpperLeft.setChecked(True)
		elif self.legendPos == 6:
			self.configDlg.ui.radioLegendPosCentreLeft.setChecked(True)
		elif self.legendPos == 3:
			self.configDlg.ui.radioLegendPosLowerLeft.setChecked(True)
		else:
			self.configDlg.ui.radioLegendPosNone.setChecked(True)
		
		if self.configDlg.exec_() == QtGui.QDialog.Accepted:
			self.figWidth = self.configDlg.ui.spinFigWidth.value()
			self.figHeight = self.configDlg.ui.spinFigHeight.value()

			self.bCustomBinWidth = self.configDlg.ui.chkCustomBinWidth.isChecked()
			self.binWidth = self.configDlg.ui.spinBinWidth.value()
			self.yAxisLogScale = self.configDlg.ui.chkLogScale.isChecked()
			
			self.bCustomXaxis = self.configDlg.ui.chkCustomXaxis.isChecked()
			self.xLimitLeft = self.configDlg.ui.spinXmin.value()
			self.xLimitRight = self.configDlg.ui.spinXmax.value()
			
			# legend position			
			if self.configDlg.ui.radioLegendPosBest.isChecked() == True:
				self.legendPos = 0
			elif self.configDlg.ui.radioLegendPosUpperRight.isChecked() == True:
				self.legendPos = 1
			elif self.configDlg.ui.radioLegendPosCentreRight.isChecked() == True:
				self.legendPos = 7
			elif self.configDlg.ui.radioLegendPosLowerRight.isChecked() == True:
				self.legendPos = 4
			elif self.configDlg.ui.radioLegendPosUpperLeft.isChecked() == True:
				self.legendPos = 2
			elif self.configDlg.ui.radioLegendPosCentreLeft.isChecked() == True:
				self.legendPos = 6
			elif self.configDlg.ui.radioLegendPosLowerLeft.isChecked() == True:
				self.legendPos = 3
			else:
				self.legendPos = -1
			
			self.settings.setValue(self.name + '/width', self.figWidth)
			self.settings.setValue(self.name + '/height', self.figHeight)
			self.settings.setValue(self.name + '/custom bin width', self.bCustomBinWidth)
			self.settings.setValue(self.name + '/bin width', self.binWidth)
			self.settings.setValue(self.name + '/log scale', self.yAxisLogScale)
			self.settings.setValue(self.name + '/custom x-axis extents', self.bCustomXaxis)
			self.settings.setValue(self.name + '/min value', self.xLimitLeft)
			self.settings.setValue(self.name + '/max value', self.xLimitRight)
			self.settings.setValue(self.name + '/legend position', self.legendPos)
	
			self.plot(profile, statsResults)
			
	def changeCustomBinWidth(self):
		self.configDlg.ui.spinBinWidth.setEnabled(self.configDlg.ui.chkCustomBinWidth.isChecked())	 
		
	def changeCustomXaxis(self):
		self.configDlg.ui.spinXmin.setEnabled(self.configDlg.ui.chkCustomXaxis.isChecked())
		self.configDlg.ui.spinXmax.setEnabled(self.configDlg.ui.chkCustomXaxis.isChecked())
			
	def setXaxisMin(self):
		seqs1 = self.profile.getSequenceCounts(0)
		seqs2 = self.profile.getSequenceCounts(1)
		self.configDlg.ui.spinXmin.setValue(min(min(seqs1), min(seqs2)))
			
	def setXaxisMax(self):
		seqs1 = self.profile.getSequenceCounts(0)
		seqs2 = self.profile.getSequenceCounts(1)
		self.configDlg.ui.spinXmax.setValue(max(max(seqs1), max(seqs2)))

if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(SeqHistogram)
	testWindow.show()
	sys.exit(app.exec_())


				