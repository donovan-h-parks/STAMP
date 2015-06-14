#=======================================================================
# Author: Donovan Parks
#
# Profile bar plot.
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

from stamp.plugins.samples.AbstractSamplePlotPlugin import AbstractSamplePlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.samples.plots.configGUI.profileBarPlotUI import Ui_ProfileBarPlotDialog

from stamp.metagenomics.stats.CI.WilsonCI import WilsonCI

from matplotlib.artist import setp

class ProfileBarPlots(AbstractSamplePlotPlugin):
	'''
	Profile bar plots.
	'''

	def __init__(self, preferences, parent=None):
		AbstractSamplePlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences
	 
		self.name = 'Profile bar plot'
		self.type = 'Exploratory'

		self.settings = preferences['Settings']
		self.figColWidth = self.settings.value(self.name + '/column width', 0.25).toDouble()[0]
		self.figHeight = self.settings.value(self.name + '/height', 6.0).toDouble()[0]
		self.fieldToPlot = self.settings.value(self.name + '/field to plot', 'Proportion of sequences (%)').toString()
		self.legendPos = self.settings.value(self.name + '/legend position', 0).toInt()[0]
		self.bShowCIs = self.settings.value(self.name + '/show cis', True).toBool()
		self.endCapSize = self.settings.value(self.name + '/end cap size', 0).toInt()[0]
		self.numFeaturesToShow = self.settings.value(self.name + '/features to show', 50).toInt()[0]
		self.barWidth = self.settings.value(self.name + '/bar width (%)', 80).toDouble()[0]
		self.bShowPvalue = self.settings.value(self.name + '/show p-value', True).toBool()
		self.pValueThreshold = self.settings.value(self.name + '/p-value threshold', 0.05).toDouble()[0]
		self.bOnlyActiveFeatures = self.settings.value(self.name + '/only active features', True).toBool()

	def mirrorProperties(self, plotToCopy):
		self.name = plotToCopy.name
		self.figColWidth = plotToCopy.figColWidth
		self.figHeight = plotToCopy.figHeight
		self.fieldToPlot = plotToCopy.fieldToPlot
		self.legendPos = plotToCopy.legendPos
		self.bShowCIs = plotToCopy.bShowCIs
		self.endCapSize = plotToCopy.endCapSize
		self.numFeaturesToShow = plotToCopy.numFeaturesToShow
		self.bShowPvalue = plotToCopy.bShowPvalue
		self.pValueThreshold = plotToCopy.pValueThreshold
		self.bOnlyActiveFeatures = plotToCopy.bOnlyActiveFeatures
		
	def plot(self, profile, statsResults):
		if len(profile.profileDict) <= 0:
			self.emptyAxis()
			return
				
		# *** Colour of plot elements
		axesColour = str(self.preferences['Axes colour'].name())
		profile1Colour = str(self.preferences['Sample 1 colour'].name())
		profile2Colour = str(self.preferences['Sample 2 colour'].name())
			
		# *** Determine most abundant features
		features = []
		field1 = []
		field2 = []
		parentField1 = []
		parentField2 = []
		
		if self.bOnlyActiveFeatures:
			activeFeatures = set([d[0] for d in statsResults.activeData])

		tables = profile.getLabeledTables()
		for table in tables:
			feature, seq1, seq2, parentSeq1, parentSeq2 = table
			
			if self.bOnlyActiveFeatures and feature not in activeFeatures:
				continue
			
			field1.append(seq1)
			field2.append(seq2)
			parentField1.append(parentSeq1)
			parentField2.append(parentSeq2)
			features.append(feature)

		fields = zip(field1, field2, parentField1, parentField2, features)
		fields.sort(reverse = True)
		field1, field2, parentField1, parentField2, features = zip(*fields)
		
		if len(field1) > self.numFeaturesToShow:
			field1 = list(field1[0:self.numFeaturesToShow])
			field2 = list(field2[0:self.numFeaturesToShow])
			parentField1 = parentField1[0:self.numFeaturesToShow]
			parentField2 = parentField2[0:self.numFeaturesToShow]
			features = list(features[0:self.numFeaturesToShow])
		else:
			field1 = list(field1)
			field2 = list(field2)
			features = list(features)

		# *** Create lists for each quantity of interest and calculate CIs
		wilsonCI = WilsonCI()
		confInter1 = []
		confInter2 = []
		
		if self.fieldToPlot == 'Number of sequences':
			for i in xrange(0, len(field1)):
				if self.bShowCIs:
					lowerCI, upperCI, p = wilsonCI.run(field1[i], parentField1[i], 0.95, 1.96)
					confInter1.append(max((p - lowerCI)*parentField1[i], 0))
					
					lowerCI, upperCI, p = wilsonCI.run(field2[i], parentField2[i], 0.95, 1.96)
					confInter2.append(max((p - lowerCI)*parentField2[i], 0))
				else:
					confInter1.append(0)
					confInter2.append(0)
				
		elif self.fieldToPlot == 'Proportion of sequences (%)':
			for i in xrange(0, len(field1)):
				if self.bShowCIs:
					lowerCI, upperCI, p = wilsonCI.run(field1[i], parentField1[i], 0.95, 1.96)
					confInter1.append(max((p - lowerCI)*100, 0))
					
					lowerCI, upperCI, p = wilsonCI.run(field2[i], parentField2[i], 0.95, 1.96)
					confInter2.append(max((p - lowerCI)*100, 0))
				else:
					confInter1.append(0)
					confInter2.append(0)
					
				field1[i] = float(field1[i])*100 / max(parentField1[i],1)
				field2[i] = float(field2[i])*100 / max(parentField2[i],1)

				
		# *** Truncate feature labels
		highlightedFeatures = list(self.preferences['Highlighted sample features'])
		
		truncatedNames = list(features)
		if self.preferences['Truncate feature names']:
			length = self.preferences['Length of truncated feature names']
						
			for i in xrange(0, len(truncatedNames)):
				if len(truncatedNames[i]) > length+3:
					truncatedNames[i] = truncatedNames[i][0:length] + '...'
					
			for i in xrange(0, len(highlightedFeatures)):
				if len(highlightedFeatures[i]) > length+3:
					highlightedFeatures[i] = highlightedFeatures[i][0:length] + '...'
					
		# *** Find longest label
		longestLabelLen = 0
		for i in xrange(0, len(truncatedNames)):
			if len(truncatedNames[i]) > longestLabelLen:
				longestLabelLen = len(truncatedNames[i])
				longestLabel = truncatedNames[i]
					
		# *** Set figure size
		self.fig.clear()
		figWidth = self.figColWidth*len(features)
		figHeight = self.figHeight
		if figWidth > 256 or figHeight > 256:
				QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.ArrowCursor))
				self.emptyAxis()	
				QtGui.QMessageBox.question(self, 'Excessively large plot', 'The resulting plot is too large to display.')
				QtGui.QApplication.instance().restoreOverrideCursor()
				return

		self.fig.set_size_inches(figWidth, figHeight)
		xLabelBounds, yLabelBounds = self.labelExtents([longestLabel], 8, 90, ['%.0f' % max(max(field1), max(field2))], 8, 0)
		yLabelOffset = 0.3 
		padding = 0.15 # inches
		
		newFigWidth = figWidth + yLabelBounds.width / figWidth  + 2*padding + yLabelOffset
		self.fig.set_size_inches(newFigWidth, figHeight)

		xOffsetFigSpace = yLabelBounds.width + padding/newFigWidth + yLabelOffset / newFigWidth
		yOffsetFigSpace = xLabelBounds.height + padding/figHeight
		axesBar = self.fig.add_axes([xOffsetFigSpace, yOffsetFigSpace, 
										1.0 - xOffsetFigSpace - padding/newFigWidth, 
										1.0 - yOffsetFigSpace - padding/figHeight])

		# *** Plot data
		colWidth = self.figColWidth
		barWidth = (colWidth*(self.barWidth/100.0)) / 2
		
		if self.bShowCIs == True:
			rects1 = axesBar.bar(np.arange(len(features))*colWidth, field1, width=barWidth, color=profile1Colour, yerr=confInter1, ecolor='black', capsize=self.endCapSize)	
			rects2 = axesBar.bar(np.arange(len(features))*colWidth + barWidth, field2, width=barWidth, color=profile2Colour, yerr=confInter2, ecolor='black', capsize=self.endCapSize)
		else:
			rects1 = axesBar.bar(np.arange(len(features))*colWidth, field1, width=barWidth, color=profile1Colour)
			rects2 = axesBar.bar(np.arange(len(features))*colWidth + barWidth, field2, width=barWidth, color=profile2Colour)

		axesBar.set_xticks(np.arange(len(features))*colWidth + barWidth)
		axesBar.set_xlim([0, (len(features)-1.0)*colWidth + 2*barWidth + 0.1])
		axesBar.set_ylim(0, axesBar.get_ylim()[1])
		axesBar.set_xticklabels(truncatedNames, size=8)
		axesBar.set_ylabel(self.fieldToPlot, fontsize=8)
		
		# *** Mark significant features
		if self.bShowPvalue and statsResults.profile != None:
			offset = axesBar.get_ylim()[1]*0.02
			
			x = []
			y = []
			for i in xrange(0, len(features)):
				pValue = float(statsResults.getFeatureStatistic(features[i], 'pValuesCorrected'))
				if pValue <= self.pValueThreshold:
					x.append(i*colWidth + barWidth)
					y.append(max(field1[i], field2[i]) + offset)
					
			axesBar.plot(x, y, color='k', linestyle='', marker='*', markeredgecolor='k', ms = 3)
		
		# *** Prettify plot
		if self.legendPos != -1:
			legend = axesBar.legend([rects1[0], rects2[0]], (profile.sampleNames[0], profile.sampleNames[1]), loc=self.legendPos)
			legend.get_frame().set_linewidth(0)
		
		for label in axesBar.get_xticklabels():
			label.set_rotation(90)
			if label.get_text() in highlightedFeatures:
					label.set_color('red')

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
		configDlg = ConfigureDialog(Ui_ProfileBarPlotDialog)
		
		# set enabled state of widgets
		configDlg.ui.spinEndCapSize.setEnabled(self.bShowCIs)
		configDlg.ui.spinPvalueThreshold.setEnabled(self.bShowPvalue)
		
		# set current values
		configDlg.ui.cboFieldToPlot.setCurrentIndex(configDlg.ui.cboFieldToPlot.findText(self.fieldToPlot))

		configDlg.ui.spinFigColWidth.setValue(self.figColWidth)
		configDlg.ui.spinFigHeight.setValue(self.figHeight)
		
		configDlg.ui.chkShowCIs.setChecked(self.bShowCIs)
		configDlg.ui.spinEndCapSize.setValue(self.endCapSize)
		
		configDlg.ui.spinFeaturesToShow.setValue(self.numFeaturesToShow)
		
		configDlg.ui.chkOnlyActiveFeatures.setChecked(self.bOnlyActiveFeatures)
		
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
			
		configDlg.ui.spinBarWidth.setValue(self.barWidth)
		
		configDlg.ui.chkShowPvalue.setChecked(self.bShowPvalue)
		configDlg.ui.spinPvalueThreshold.setValue(self.pValueThreshold)
		
		if configDlg.exec_() == QtGui.QDialog.Accepted:	 
			self.fieldToPlot = str(configDlg.ui.cboFieldToPlot.currentText())
			
			self.figColWidth = configDlg.ui.spinFigColWidth.value()
			self.figHeight = configDlg.ui.spinFigHeight.value()
			
			self.bShowCIs = configDlg.ui.chkShowCIs.isChecked()
			self.endCapSize = configDlg.ui.spinEndCapSize.value()
			
			self.numFeaturesToShow = configDlg.ui.spinFeaturesToShow.value()
			
			self.bOnlyActiveFeatures = configDlg.ui.chkOnlyActiveFeatures.isChecked()
			
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
				
			self.barWidth = configDlg.ui.spinBarWidth.value()
			
			self.bShowPvalue = configDlg.ui.chkShowPvalue.isChecked()
			self.pValueThreshold = configDlg.ui.spinPvalueThreshold.value()
				
			self.settings.setValue(self.name + '/column width', self.figColWidth)
			self.settings.setValue(self.name + '/height', self.figHeight)
			self.settings.setValue(self.name + '/field to plot', self.fieldToPlot)
			self.settings.setValue(self.name + '/legend position', self.legendPos)
			self.settings.setValue(self.name + '/show cis', self.bShowCIs)
			self.settings.setValue(self.name + '/end cap size', self.endCapSize)
			self.settings.setValue(self.name + '/features to show', self.numFeaturesToShow)
			self.settings.setValue(self.name + '/bar width (%)', self.barWidth)
			self.settings.setValue(self.name + '/show p-value', self.bShowPvalue)
			self.settings.setValue(self.name + '/p-value threshold', self.pValueThreshold)
			self.settings.setValue(self.name + '/only active features', self.bOnlyActiveFeatures)
			
			self.plot(profile, statsResults)
					
if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(ProfileBarPlots)
	testWindow.show()
	sys.exit(app.exec_())


				