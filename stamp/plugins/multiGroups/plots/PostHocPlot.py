#=======================================================================
# Author: Donovan Parks
#
# Post-hoc plot.
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

from PyQt4 import QtGui, QtCore

import sys
import math
import operator
import numpy as np

from matplotlib.font_manager import FontProperties
from mpl_toolkits.axes_grid import make_axes_locatable, Size

from stamp.plugins.multiGroups.AbstractMultiGroupPlotPlugin import AbstractMultiGroupPlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.multiGroups.plots.configGUI.PostHocPlotUI import Ui_PostHocPlotDialog
from stamp.metagenomics import TableHelper

class PostHocPlot(AbstractMultiGroupPlotPlugin):
	'''
	Post-hoc plot.
	'''	 
	def __init__(self, preferences, parent=None):
		AbstractMultiGroupPlotPlugin.__init__(self, preferences, parent)
		
		self.name = 'Post-hoc plot'
		self.type = 'Statistical'

		self.bRunPostHocTest = True
		
		self.settings = preferences['Settings']
		self.figWidth = self.settings.value('multiple group: ' + self.name + '/width', 7.0).toDouble()[0]
		self.figHeightPerRow = self.settings.value('multiple group: ' + self.name + '/row height', 0.2).toDouble()[0]
		self.sortingField = self.settings.value('multiple group: ' + self.name + '/field', 'p-values').toString()
		self.bShowBarPlot = self.settings.value('multiple group: ' + self.name + '/sequences subplot', True).toBool()
		self.bShowPValueLabels = self.settings.value('multiple group: ' + self.name + '/p-value labels', True).toBool()
		self.bCustomLimits = self.settings.value('multiple group: ' + self.name + '/use custom limits', False).toBool()
		self.minX = self.settings.value('multiple group: ' + self.name + '/minimum', 0.0).toDouble()[0]
		self.maxX = self.settings.value('multiple group: ' + self.name + '/maximum', 1.0).toDouble()[0]
		self.markerSize = self.settings.value('multiple group: ' + self.name + '/marker size', 30).toInt()[0]
		self.bShowStdDev = self.settings.value('multiple group: ' + self.name + '/show std. dev.', False).toBool()
		self.endCapSize = self.settings.value('multiple group: ' + self.name + '/end cap size', 0.0).toInt()[0]
		self.bPvalueFilter = self.settings.value('multiple group: ' + self.name + '/p-value filter', True).toBool()
		
	def mirrorProperties(self, plotToCopy):
		self.name = plotToCopy.name
		
		self.figWidth = plotToCopy.figWidth
		self.figHeightPerRow = plotToCopy.figHeightPerRow
		
		self.sortingField = plotToCopy.sortingField
		
		self.bShowBarPlot = plotToCopy.bShowBarPlot
		self.bShowPValueLabels = plotToCopy.bShowPValueLabels
		
		self.bCustomLimits = plotToCopy.bCustomLimits
		self.minX = plotToCopy.minX
		self.maxX = plotToCopy.maxX
		
		self.markerSize = plotToCopy.markerSize
		self.bShowStdDev = plotToCopy.bShowStdDev
		self.endCapSize = plotToCopy.endCapSize
		
		self.bPvalueFilter = plotToCopy.bPvalueFilter
	
	def plot(self, profile, statsResults):
		# *** Check if there is sufficient data to generate the plot
		if len(statsResults.postHocResults.pValues) <= 0:
			self.emptyAxis('No post-hoc test results')
			return

		if len(statsResults.postHocResults.pValues) > 200:
			QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.ArrowCursor))
			reply = QtGui.QMessageBox.question(self, 'Continue?', 'Plots contains ' + str(len(statsResults.postHocResults.pValues)) + ' rows. ' +
																		'It may take several seconds to generate this plot. We recommend filtering the results first.' + 
																		'Do you wish to continue?', QtGui.QMessageBox.Yes, QtGui.QMessageBox.No)
			QtGui.QApplication.instance().restoreOverrideCursor()
			if reply == QtGui.QMessageBox.No:
				self.emptyAxis('Too many rows.')	
				return
				
		# *** Set plot properties
		axesColour = str(self.preferences['Axes colour'].name())
		highlightColor = (0.9, 0.9, 0.9)
		
		# Apply p-value filter
		labels = []
		pValues = []
		effectSizes = []
		lowerCIs = []
		upperCIs = []
		if self.bPvalueFilter:
			for i in xrange(0, len(statsResults.postHocResults.labels)):
				# get numeric p-value
				if isinstance(statsResults.postHocResults.pValues[i], str):
					pValueSplit = statsResults.postHocResults.pValues[i].split(' ')
					if pValueSplit[0][0] == '<':
						pValue = float(pValueSplit[1]) - 1e-6
					else:
						pValue = 1.0
				else:
					pValue = statsResults.postHocResults.pValues[i]
			
				# check if p-value should be filtered
				if pValue <= statsResults.postHocResults.alpha:
					labels.append(statsResults.postHocResults.labels[i])
					pValues.append(statsResults.postHocResults.pValues[i])
					effectSizes.append(statsResults.postHocResults.effectSizes[i])
					lowerCIs.append(statsResults.postHocResults.lowerCIs[i])
					upperCIs.append(statsResults.postHocResults.upperCIs[i])
		else:
			labels = list(statsResults.postHocResults.labels)
			pValues = list(statsResults.postHocResults.pValues)
			effectSizes = list(statsResults.postHocResults.effectSizes)
			lowerCIs = list(statsResults.postHocResults.lowerCIs)
			upperCIs = list(statsResults.postHocResults.upperCIs)
			
		if len(labels) == 0:
			self.emptyAxis('No rows above nominal level.')
			return
			
		# *** Determine dominant group for each contrast (i.e., row).
		#  Adjust labels and effect sizes to reflect the dominant group.
		for i in xrange(0, len(effectSizes)):
			labelSplit = labels[i].split(':')
			if effectSizes[i] > 0.0:
				lowerCIs[i] = effectSizes[i] - lowerCIs[i]
				upperCIs[i] = upperCIs[i] - effectSizes[i]
			else:
				labels[i] = labelSplit[1].strip() + ' : ' + labelSplit[0].strip()
				lowerCIs[i] = effectSizes[i] - lowerCIs[i]
				upperCIs[i] = upperCIs[i] - effectSizes[i]
				effectSizes[i] = -effectSizes[i]
				
		# *** Sort data
		data = zip(labels, pValues, effectSizes, lowerCIs, upperCIs)

		if self.sortingField == 'p-values':
			data = sorted(data, key = operator.itemgetter(1), reverse = True)
		elif self.sortingField == 'Effect sizes':
			data = sorted(data, key = operator.itemgetter(2))
		elif self.sortingField == 'Group labels':
			data = sorted(data, key = lambda row: row[0].lower(), reverse = True)

		labels, pValues, effectSizes, lowerCIs, upperCIs = zip(*data)
		labels = list(labels)
		pValues = list(pValues)
		effectSizes = list(effectSizes)
		lowerCIs = list(lowerCIs)
		upperCIs = list(upperCIs)
		
		# *** Make list of which group is dominant in each contrast.
		dominantGroup = {}
		for i in xrange(0, len(effectSizes)):
			labelSplit = labels[i].split(':')
			groupName = labelSplit[0].strip()

			if groupName in dominantGroup:
				dominantGroup[groupName][0].append(effectSizes[i])
				dominantGroup[groupName][1].append(i)
			else:
				dominantGroup[groupName] = [[effectSizes[i]],[i]]

		# *** Create p-value labels
		pValueTitle = 'p-value'
		pValueLabels = []
		for pValue in pValues:
			if isinstance(pValue, str):
				pValueSplit = pValue.split(' ')
				if pValue[0] == '<':
					pValueLabels.append(r'$<$' + pValueSplit[1])
				else:
					pValueLabels.append(r'$\geq$' + pValueSplit[1])
			else:
				pValueLabels.append(statsResults.getPValueStr(pValue))

		# *** Truncate labels
		adjustedLabels = list(labels)
		if self.preferences['Truncate feature names']:
			length = self.preferences['Length of truncated feature names']
			
			for i in xrange(0, len(labels)):
				if len(labels[i]) > length+3:
					adjustedLabels[i] = labels[i][0:length] + '...'
				
		# *** Set figure size
		plotHeight = self.figHeightPerRow*len(adjustedLabels) 
		self.imageWidth = self.figWidth
		self.imageHeight = plotHeight	+ 0.65	 # 0.65 inches for bottom and top labels
		if self.imageWidth > 256 or self.imageHeight > 256:
				QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.ArrowCursor))
				self.emptyAxis()	
				reply = QtGui.QMessageBox.question(self, 'Excessively large plot', 'The resulting plot is too large to display.')
				QtGui.QApplication.instance().restoreOverrideCursor()
				return
		
		self.fig.set_size_inches(self.imageWidth, self.imageHeight)	
				
		# *** Determine width of y-axis labels
		yLabelBounds = self.yLabelExtents(adjustedLabels, 8)
		
		# *** Size plots which comprise the extended errorbar plot
		self.fig.clear()
		
		heightBottomLabels = 0.4	# inches

		spacingBetweenPlots = 0.25	# inches
		widthNumSeqPlot = 1.25	# inches
		if self.bShowBarPlot == False:
			widthNumSeqPlot = 0.0
			spacingBetweenPlots = 0.0
		
		widthPvalueLabels = 0.75	# inches
		if self.bShowPValueLabels == False:
			widthPvalueLabels = 0.1
				 
		yPlotOffsetFigSpace = heightBottomLabels / self.imageHeight 
		heightPlotFigSpace = plotHeight / self.imageHeight
			 
		xPlotOffsetFigSpace = yLabelBounds.width + 0.1 / self.imageWidth
		pValueLabelWidthFigSpace =	widthPvalueLabels / self.imageWidth
		widthPlotFigSpace = 1.0 - pValueLabelWidthFigSpace - xPlotOffsetFigSpace
		
		widthErrorBarPlot = widthPlotFigSpace*self.imageWidth - widthNumSeqPlot - spacingBetweenPlots
				
		axInitAxis = self.fig.add_axes([xPlotOffsetFigSpace,yPlotOffsetFigSpace,widthPlotFigSpace,heightPlotFigSpace])
		divider = make_axes_locatable(axInitAxis)
		divider.get_vertical()[0] = Size.Fixed(len(labels)*self.figHeightPerRow)

		self.fig.text(0.0,1.0,self.preferences['Selected multiple group feature'], va='top', ha='left')
		
		if self.bShowBarPlot == True:
			divider.get_horizontal()[0] = Size.Fixed(widthNumSeqPlot)
			axErrorbar = divider.new_horizontal(widthErrorBarPlot, pad=spacingBetweenPlots, sharey=axInitAxis)
			self.fig.add_axes(axErrorbar)
		else:
			divider.get_horizontal()[0] = Size.Fixed(widthErrorBarPlot)
			axErrorbar = axInitAxis
				
		# *** Plot of sequences for each subsystem
		if self.bShowBarPlot == True:
			axNumSeq = axInitAxis
			
			# get relative frequency and standard deviation of each contrast
			maxPercentage = 0
			for i in xrange(0, len(labels)):
				splitLabel = labels[i].split(':')
				groupName1 = splitLabel[0].strip()
				groupName2 = splitLabel[1].strip()
				
				colour1 = str(self.preferences['Group colours'][groupName1].name())
				colour2 = str(self.preferences['Group colours'][groupName2].name())

				meanRelFreq1 = statsResults.getDataFromTable(statsResults.postHocResults.feature, groupName1 + ': mean rel. freq. (%)')
				meanRelFreq2 = statsResults.getDataFromTable(statsResults.postHocResults.feature, groupName2 + ': mean rel. freq. (%)')
			
				if self.bShowStdDev:
					stdDev1 = statsResults.getDataFromTable(statsResults.postHocResults.feature, groupName1 + ': std. dev. (%)')
					stdDev2 = statsResults.getDataFromTable(statsResults.postHocResults.feature, groupName2 + ': std. dev. (%)')
					endCapSize = self.endCapSize
				else:
					stdDev1 = 0
					stdDev2 = 0
					endCapSize = 0
					
				if meanRelFreq1 + stdDev1 > maxPercentage:
					maxPercentage = meanRelFreq1 + stdDev1
					
				if meanRelFreq2 + stdDev2 > maxPercentage:
					maxPercentage = meanRelFreq2 + stdDev2

				axNumSeq.barh(i+0.0, meanRelFreq1, height = 0.3, xerr=stdDev1, color=colour1, ecolor='black', capsize=endCapSize)
				axNumSeq.barh(i-0.3, meanRelFreq2, height = 0.3, xerr=stdDev2, color=colour2, ecolor='black', capsize=endCapSize)
				
			for value in np.arange(-0.5, len(labels)-1, 2):
				axNumSeq.axhspan(value, value+1, facecolor=highlightColor, edgecolor='none', zorder=-1)
			
			axNumSeq.set_xlabel('Mean proportion (%)')
			axNumSeq.set_xticks([0, maxPercentage])
			axNumSeq.set_xlim([0, maxPercentage*1.05])
			maxPercentageStr = '%.1f' % maxPercentage
			axNumSeq.set_xticklabels(['0.0', maxPercentageStr])
			
			axNumSeq.set_yticks(np.arange(len(labels)))
			axNumSeq.set_yticklabels(adjustedLabels)
			axNumSeq.set_ylim([-1, len(labels)])
					
			for a in axNumSeq.yaxis.majorTicks:
				a.tick1On=False
				a.tick2On=False
					
			for a in axNumSeq.xaxis.majorTicks:
				a.tick1On=True
				a.tick2On=False
				
			for line in axNumSeq.yaxis.get_ticklines(): 
				line.set_color(axesColour)
				
			for line in axNumSeq.xaxis.get_ticklines(): 
				line.set_color(axesColour)
					
			for loc, spine in axNumSeq.spines.iteritems():
				if loc in ['left', 'right','top']:
					spine.set_color('none') 
				else:
					spine.set_color(axesColour)
						
		# *** Plot confidence intervals for each subsystem
		lastAxes = axErrorbar
		markerSize = math.sqrt(float(self.markerSize))
		axErrorbar.errorbar(effectSizes, np.arange(len(labels)), xerr=[lowerCIs,upperCIs], fmt='o', ms=markerSize, mfc='black', mec='black', ecolor='black', zorder=10)
		for groupName in dominantGroup:
			colour = str(self.preferences['Group colours'][groupName].name())
			effectSizes = dominantGroup[groupName][0]
			indices = dominantGroup[groupName][1]
			axErrorbar.plot(effectSizes, indices, ls='', marker='o', ms=markerSize, mfc=colour, mec='black', zorder=100)
			
		axErrorbar.vlines(0, -1, len(labels), linestyle='dashed', color=axesColour)
		
		for value in np.arange(-0.5, len(labels)-1, 2):
			axErrorbar.axhspan(value, value+1, facecolor=highlightColor,edgecolor='none',zorder=1)

		ciTitle = ('%.3g' % ((1.0-statsResults.postHocResults.alpha)*100)) + '% confidence intervals'
		axErrorbar.set_title(ciTitle) 
		axErrorbar.set_xlabel('Difference in mean proportions (%)')
		
		if self.bCustomLimits:
			axErrorbar.set_xlim([self.minX, self.maxX])
		else:
			self.minX, self.maxX = axErrorbar.get_xlim()
 
		if self.bShowBarPlot == False:
			axErrorbar.set_yticks(np.arange(len(labels)))
			axErrorbar.set_yticklabels(labels)
			axErrorbar.set_ylim([-1, len(labels)])
		else:
			for label in axErrorbar.get_yticklabels():
				label.set_visible(False)
				
			for a in axErrorbar.yaxis.majorTicks:
				a.set_visible(False)
				
		for a in axErrorbar.xaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
				
		for a in axErrorbar.yaxis.majorTicks:
			a.tick1On=False
			a.tick2On=False
			
		for line in axErrorbar.yaxis.get_ticklines(): 
			line.set_visible(False)
				
		for line in axErrorbar.xaxis.get_ticklines(): 
			line.set_color(axesColour)

		for loc, spine in axErrorbar.spines.iteritems():
			if loc in ['left','right','top']:
				spine.set_color('none') 
			else:
				spine.set_color(axesColour)
						
		# *** Show p-values on right of last plot
		if self.bShowPValueLabels == True:
			axRight = lastAxes.twinx()
			axRight.set_yticks(np.arange(len(pValueLabels)))
			axRight.set_yticklabels(pValueLabels)
			axRight.set_ylim([-1, len(pValueLabels)])
			axRight.set_ylabel(pValueTitle)
			
			for a in axRight.yaxis.majorTicks:
				a.tick1On=False
				a.tick2On=False
				
			for loc, spine in axRight.spines.iteritems():
				spine.set_color('none') 

		self.updateGeometry()
		self.draw()

	def configure(self, profile, statsResults):
		self.configDlg = ConfigureDialog(Ui_PostHocPlotDialog)
		
		# set enabled state of controls
		self.configDlg.ui.chkShowStdDev.setChecked(self.bShowBarPlot)
		self.configDlg.ui.spinEndCapSize.setValue(self.bShowBarPlot)
		self.configDlg.ui.spinMinimumX.setEnabled(self.bCustomLimits)
		self.configDlg.ui.spinMaximumX.setEnabled(self.bCustomLimits)

		# set current value of controls
		self.configDlg.ui.cboSortingField.setCurrentIndex(self.configDlg.ui.cboSortingField.findText(self.sortingField))
		
		self.configDlg.ui.spinFigWidth.setValue(self.figWidth)
		self.configDlg.ui.spinFigRowHeight.setValue(self.figHeightPerRow)
		
		self.configDlg.ui.chkShowBarPlot.setChecked(self.bShowBarPlot)
		self.configDlg.ui.chkPValueLabels.setChecked(self.bShowPValueLabels)

		self.configDlg.ui.chkCustomLimits.setChecked(self.bCustomLimits)
		self.configDlg.ui.spinMinimumX.setValue(self.minX)
		self.configDlg.ui.spinMaximumX.setValue(self.maxX)
		
		self.configDlg.ui.spinMarkerSize.setValue(self.markerSize)

		self.configDlg.ui.chkShowStdDev.setChecked(self.bShowStdDev)
		self.configDlg.ui.spinEndCapSize.setValue(self.endCapSize)
		
		self.configDlg.ui.chkFilterPvalue.setChecked(self.bPvalueFilter)
		
		if self.configDlg.exec_() == QtGui.QDialog.Accepted:
			QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))
			
			self.sortingField = str(self.configDlg.ui.cboSortingField.currentText())
			
			self.figWidth = self.configDlg.ui.spinFigWidth.value()
			self.figHeightPerRow = self.configDlg.ui.spinFigRowHeight.value()
			
			self.bShowBarPlot = self.configDlg.ui.chkShowBarPlot.isChecked()
			self.bShowPValueLabels = self.configDlg.ui.chkPValueLabels.isChecked()
			
			self.bCustomLimits = self.configDlg.ui.chkCustomLimits.isChecked()
			self.minX = self.configDlg.ui.spinMinimumX.value()
			self.maxX = self.configDlg.ui.spinMaximumX.value()
			
			self.markerSize = self.configDlg.ui.spinMarkerSize.value()
			
			self.bShowStdDev = self.configDlg.ui.chkShowStdDev.isChecked()
			self.endCapSize = self.configDlg.ui.spinEndCapSize.value()
			
			self.bPvalueFilter = self.configDlg.ui.chkFilterPvalue.isChecked()
		
			self.settings.setValue('multiple group: ' + self.name + '/width', self.figWidth)
			self.settings.setValue('multiple group: ' + self.name + '/row height', self.figHeightPerRow)
			self.settings.setValue('multiple group: ' + self.name + '/field', self.sortingField)
			self.settings.setValue('multiple group: ' + self.name + '/sequences subplot', self.bShowBarPlot)
			self.settings.setValue('multiple group: ' + self.name + '/p-value labels', self.bShowPValueLabels)
			self.settings.setValue('multiple group: ' + self.name + '/use custom limits', self.bCustomLimits)
			self.settings.setValue('multiple group: ' + self.name + '/minimum', self.minX)
			self.settings.setValue('multiple group: ' + self.name + '/maximum', self.maxX)
			self.settings.setValue('multiple group: ' + self.name + '/marker size', self.markerSize)
			self.settings.setValue('multiple group: ' + self.name + '/show std. dev.', self.bShowStdDev)
			self.settings.setValue('multiple group: ' + self.name + '/end cap size', self.endCapSize)
			self.settings.setValue('multiple group: ' + self.name + '/p-value filter', self.bPvalueFilter)

			self.plot(profile, statsResults)
			
			QtGui.QApplication.instance().restoreOverrideCursor()

if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(ExtendedErrorBar)
	testWindow.show()
	sys.exit(app.exec_())
