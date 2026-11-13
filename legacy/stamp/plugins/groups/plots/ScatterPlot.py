#=======================================================================
# Author: Donovan Parks
#
# Scatter plot.
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

from stamp.plugins.groups.AbstractGroupPlotPlugin import AbstractGroupPlotPlugin, TestWindow, ConfigureDialog
from stamp.plugins.groups.plots.configGUI.scatterPlotUI import Ui_ScatterPlotDialog

from stamp.plugins.PlotEventHandler import PlotEventHandler

from matplotlib.ticker import NullFormatter

from numpy import mean, std
from scipy.stats import linregress
from scipy.stats.mstats import mquantiles

class ScatterPlot(AbstractGroupPlotPlugin):
	'''
	Scatter plot.
	'''
	def __init__(self, preferences, parent=None):
		AbstractGroupPlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences

		self.name = 'Scatter plot'
		self.type = 'Exploratory'
		self.bSupportsHighlight = True
		self.bPlotFeaturesIndividually = False
		
		self.settings = preferences['Settings']
		self.figWidth = self.settings.value('group: ' + self.name + '/width', 7.0).toDouble()[0]
		self.figHeight = self.settings.value('group: ' + self.name + '/height', 7.0).toDouble()[0]
		self.spreadMethod = self.settings.value('group: ' + self.name + '/spread method', '2nd and 98th percentile').toString()
		self.numBins = self.settings.value('group: ' + self.name + '/bins', 30).toInt()[0]
		self.histogramSize = self.settings.value('group: ' + self.name + '/bin size', 0.5).toDouble()[0]
		self.bShowHistograms = self.settings.value('group: ' + self.name + '/show histograms', True).toBool()
		self.markerSize = self.settings.value('group: ' + self.name + '/marker size', 20).toInt()[0]
		self.bShowR2 = self.settings.value('group: ' + self.name + '/show R2', True).toBool()
		
	def mirrorProperties(self, plotToCopy):
		self.name = plotToCopy.name
		self.figWidth = plotToCopy.figWidth
		self.figHeight = plotToCopy.figHeight
		self.spreadMethod = plotToCopy.spreadMethod
		self.numBins = plotToCopy.numBins
		self.histogramSize = plotToCopy.histogramSize
		self.bShowHistograms = plotToCopy.bShowHistograms
		self.markerSize = plotToCopy.markerSize
		
	def plot(self, profile, statsResults):
		if len(profile.profileDict) <= 0:
			self.emptyAxis()
			return

		if len(profile.profileDict) > 10000:
			QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.ArrowCursor))
			reply = QtGui.QMessageBox.question(self, 'Continue?', 'Profile contains ' + str(len(profile.profileDict)) + ' features. ' +
																		'It may take several seconds to generate this plot. Exploring the data at a higher hierarchy level is recommended. ' + 
																		'Do you wish to continue?', QtGui.QMessageBox.Yes, QtGui.QMessageBox.No)
			QtGui.QApplication.instance().restoreOverrideCursor()
			if reply == QtGui.QMessageBox.No:
				self.emptyAxis()	
				return
						
		# *** Colour of plot elements
		axesColour = str(self.preferences['Axes colour'].name())
		group1Colour = str(self.preferences['Group colours'][profile.groupName1].name())
		group2Colour = str(self.preferences['Group colours'][profile.groupName2].name())
		
		# *** Set sample names
		self.groupName1 = profile.groupName1
		self.groupName2 = profile.groupName2
						
		# *** Create lists for each quantity of interest and calculate spread of data
		groupData1, groupData2 = profile.getFeatureProportionsAll()
		features = profile.getFeatures()
		
		field1 = []
		field2 = []
		xSpread = []
		ySpread = []
		for i in xrange(0, len(groupData1)):
			mean1 = mean(groupData1[i])
			mean2 = mean(groupData2[i])
			
			field1.append(mean1)
			field2.append(mean2)
			
			if self.spreadMethod == 'standard deviation':
				xSpread.append([max(mean1 - std(groupData1[i]), 0), min(mean1 + std(groupData1[i]),100)])
				ySpread.append([max(mean2 - std(groupData2[i]), 0), min(mean2 + std(groupData2[i]),100)])
			elif self.spreadMethod == '2 * standard deviation':
				xSpread.append([max(mean1 - 2*std(groupData1[i]), 0), min(mean1 + 2*std(groupData1[i]),100)])
				ySpread.append([max(mean2 - 2*std(groupData2[i]), 0), min(mean2 + 2*std(groupData2[i]),100)])
			elif self.spreadMethod == '25th and 75th percentile':
				spread1 = mquantiles(groupData1[i], prob=[0.25,0.75])
				spread2 = mquantiles(groupData2[i], prob=[0.25,0.75])
				xSpread.append([max(spread1[0], 0), min(spread1[1],100)])
				ySpread.append([max(spread2[0], 0), min(spread2[1],100)])
			elif self.spreadMethod == '9th and 91st percentile':
				spread1 = mquantiles(groupData1[i], prob=[0.09,0.91])
				spread2 = mquantiles(groupData2[i], prob=[0.09,0.91])
				xSpread.append([max(spread1[0], 0), min(spread1[1],100)])
				ySpread.append([max(spread2[0], 0), min(spread2[1],100)])
			elif self.spreadMethod == '2nd and 98th percentile':
				spread1 = mquantiles(groupData1[i], prob=[0.02,0.98])
				spread2 = mquantiles(groupData2[i], prob=[0.02,0.98])
				xSpread.append([max(spread1[0], 0), min(spread1[1],100)])
				ySpread.append([max(spread2[0], 0), min(spread2[1],100)])
			elif self.spreadMethod == 'minimum and maximum':
				xSpread.append([max(groupData1[i]), min(groupData1[i])])
				ySpread.append([max(groupData2[i]), min(groupData2[i])])
				
		# *** Set figure size
		self.fig.clear()
		self.fig.set_size_inches(self.figWidth, self.figHeight)	
		
		if self.bShowHistograms:
				histogramSizeX = self.histogramSize /self.figWidth
				histogramSizeY = self.histogramSize /self.figHeight
		else:
				histogramSizeX = 0.0
				histogramSizeY = 0.0
	
		padding = 0.1	# inches
		xOffsetFigSpace = (0.4 + padding)/self.figWidth
		yOffsetFigSpace = (0.3 + padding)/self.figHeight
		axesScatter = self.fig.add_axes([xOffsetFigSpace, yOffsetFigSpace,
																		1.0 - xOffsetFigSpace - histogramSizeX - (2*padding)/self.figWidth, 1.0 - yOffsetFigSpace - histogramSizeY - (2*padding)/self.figHeight])

		if self.bShowHistograms:
				axesTopHistogram = self.fig.add_axes([xOffsetFigSpace, 1.0 - histogramSizeY - padding/self.figHeight,
																		1.0 - xOffsetFigSpace - histogramSizeX - (2*padding)/self.figWidth, histogramSizeY])
		
				axesRightHistogram = self.fig.add_axes([1.0 - histogramSizeX - padding/self.figWidth, yOffsetFigSpace,
																		histogramSizeX, 1.0 - yOffsetFigSpace - histogramSizeY - (2*padding)/self.figHeight])
		
		# *** Handle mouse events
		tooltips = []
		for i in xrange(0, len(field1)):
			tooltip = features[i] + '\n\n'
			tooltip += (self.groupName1 + ' mean proportion: %.3f' % field1[i]) + '\n' 
			tooltip += (self.groupName2 + ' mean proportion: %.3f' % field2[i]) + '\n\n' 
			tooltip += 'Difference between mean proportions (%): ' + ('%.3f' % (field1[i] - field2[i])) + '\n'
			
			if field2[i] != 0:
				tooltip += 'Ratio of mean proportions: %.3f' % (field1[i]/field2[i])
			else:
				tooltip += 'Ratio of mean proportions: undefined'
			
			if statsResults.profile != None:
				pValue = statsResults.getFeatureStatisticAsStr(features[i], 'pValues')
				pValueCorrected = statsResults.getFeatureStatisticAsStr(features[i], 'pValuesCorrected')
				tooltip += '\n\n'
				tooltip += 'p-value: ' + pValue + '\n'
				tooltip += 'Corrected p-value: ' + pValueCorrected
				
			tooltips.append(tooltip)
			
		self.plotEventHandler =	PlotEventHandler(field1, field2, tooltips)
		
		self.mouseEventCallback(self.plotEventHandler)
		
		# *** Calculate R^2 value
		slope, intercept, r_value, p_value, std_err = linregress(field1, field2)
		
		# *** Plot data
		
		# set visual properties of all points
		colours = []
		highlightedField1 = []
		highlightedField2 = []
		highlighColours = []
		for i in xrange(0, len(field1)):
			if field1[i] > field2[i]:
				colours.append(group1Colour)
			else:
				colours.append(group2Colour)
				
			if features[i] in self.preferences['Highlighted group features']:
				highlightedField1.append(field1[i])
				highlightedField2.append(field2[i])
				highlighColours.append(colours[i])
		
		# scatter plot
		axesScatter.scatter(field1, field2, c=colours, s=self.markerSize, zorder=5)
		if len(highlightedField1) > 0:
			axesScatter.scatter(highlightedField1, highlightedField2, c=highlighColours, s=self.markerSize, edgecolors = 'red', linewidth = 2, zorder=10)
		
		# plot CIs
		if self.spreadMethod != 'None':
			xlist = []
			ylist = []
			for i in xrange(0, len(field1)):
				# horizontal CIs
				xlist.append(xSpread[i][0])
				xlist.append(xSpread[i][1])
				xlist.append(None)
				ylist.append(field2[i])
				ylist.append(field2[i])
				ylist.append(None)
				
				# vertical CIs
				xlist.append(field1[i])
				xlist.append(field1[i])
				xlist.append(None)
				ylist.append(ySpread[i][0])
				ylist.append(ySpread[i][1])
				ylist.append(None)

			axesScatter.plot(xlist, ylist, '-', color='gray', antialiased=False)
			
		# plot y=x line
		maxProportion = max(max(field1),max(field2))*1.05
		axesScatter.plot([0,maxProportion],[0,maxProportion], color=axesColour, linestyle='dashed', marker='', zorder = 1)
		
		axesScatter.set_xlabel(self.groupName1 + ' (%)')
		axesScatter.set_ylabel(self.groupName2 + ' (%)')
		
		if self.bShowR2:
			axesScatter.text(0.02, 0.98, r'R$^2$ = ' + ('%0.3f' % r_value**2), horizontalalignment='left', verticalalignment='top', transform=axesScatter.transAxes)
				
		axesScatter.set_xlim(0, maxProportion)
		axesScatter.set_ylim(0, maxProportion)
		
		# *** Prettify scatter plot
		for line in axesScatter.yaxis.get_ticklines(): 
			line.set_color(axesColour)
				
		for line in axesScatter.xaxis.get_ticklines(): 
			line.set_color(axesColour)
			
		for loc, spine in axesScatter.spines.iteritems():
			spine.set_color(axesColour)

		# plot histograms
		if not self.bShowHistograms:
			for a in axesScatter.yaxis.majorTicks:
					a.tick1On=True
					a.tick2On=False
				
			for a in axesScatter.xaxis.majorTicks:
					a.tick1On=True
					a.tick2On=False
					
			for line in axesScatter.yaxis.get_ticklines(): 
				line.set_color(axesColour)
			
			for line in axesScatter.xaxis.get_ticklines(): 
				line.set_color(axesColour)

			for loc, spine in axesScatter.spines.iteritems():
					if loc in ['right','top']:
							spine.set_color('none')
					else:
						spine.set_color(axesColour)
			
		else: # show histograms 
				# plot top histogram
				axesTopHistogram.xaxis.set_major_formatter(NullFormatter())
				pdf, bins, patches = axesTopHistogram.hist(field1, bins = self.numBins, facecolor = group1Colour)
				axesTopHistogram.set_xlim(axesScatter.get_xlim())
				axesTopHistogram.set_yticks([0, max(pdf)])
				axesTopHistogram.set_ylim([0, max(pdf)*1.05])

				# plot right histogram
				axesRightHistogram.yaxis.set_major_formatter(NullFormatter())
				pdf, bins, patches = axesRightHistogram.hist(field2, bins = self.numBins, orientation='horizontal', facecolor = group2Colour)
				axesRightHistogram.set_ylim(axesScatter.get_ylim())
				axesRightHistogram.set_xticks([0, max(pdf)])
				axesRightHistogram.set_xlim([0, max(pdf)*1.05])

				# *** Prettify histogram plot
				for a in axesTopHistogram.yaxis.majorTicks:
						a.tick1On=True
						a.tick2On=False
					
				for a in axesTopHistogram.xaxis.majorTicks:
						a.tick1On=True
						a.tick2On=False
						
				for line in axesTopHistogram.yaxis.get_ticklines(): 
					line.set_color(axesColour)
				
				for line in axesTopHistogram.xaxis.get_ticklines(): 
					line.set_color(axesColour)

				for loc, spine in axesTopHistogram.spines.iteritems():
						if loc in ['right','top']:
								spine.set_color('none')
						else:
							spine.set_color(axesColour)

				for a in axesRightHistogram.yaxis.majorTicks:
						a.tick1On=True
						a.tick2On=False
					
				for a in axesRightHistogram.xaxis.majorTicks:
						a.tick1On=True
						a.tick2On=False
						
				for line in axesRightHistogram.yaxis.get_ticklines(): 
					line.set_color(axesColour)
				
				for line in axesRightHistogram.xaxis.get_ticklines(): 
					line.set_color(axesColour)

				for loc, spine in axesRightHistogram.spines.iteritems():
						if loc in ['right','top']:
								spine.set_color('none') 
						else:
							spine.set_color(axesColour)

		self.updateGeometry()
		self.draw()

	def configure(self, profile, statsResults):
		configDlg = ConfigureDialog(Ui_ScatterPlotDialog)
		
		configDlg.ui.spinNumBins.setEnabled(self.bShowHistograms)
		configDlg.ui.spinHistogramSize.setEnabled(self.bShowHistograms)

		configDlg.ui.spinFigWidth.setValue(self.figWidth)
		configDlg.ui.spinFigHeight.setValue(self.figHeight)
		
		configDlg.ui.cboSpreadMethod.setCurrentIndex(configDlg.ui.cboSpreadMethod.findText(self.spreadMethod))

		configDlg.ui.spinNumBins.setValue(self.numBins)
		configDlg.ui.spinHistogramSize.setValue(self.histogramSize)
		configDlg.ui.chkShowHistogram.setChecked(self.bShowHistograms)
		
		configDlg.ui.spinMarkerSize.setValue(self.markerSize)
		
		configDlg.ui.chkShowR2.setChecked(self.bShowR2)
				
		if configDlg.exec_() == QtGui.QDialog.Accepted:
			self.figWidth = configDlg.ui.spinFigWidth.value()
			self.figHeight = configDlg.ui.spinFigHeight.value()
			
			self.spreadMethod = configDlg.ui.cboSpreadMethod.currentText()
			
			self.numBins = configDlg.ui.spinNumBins.value()
			self.histogramSize = configDlg.ui.spinHistogramSize.value()
			self.bShowHistograms = configDlg.ui.chkShowHistogram.isChecked()
			
			self.markerSize = configDlg.ui.spinMarkerSize.value()
			
			self.bShowR2 = configDlg.ui.chkShowR2.isChecked()
			
			self.settings.setValue('group: ' + self.name + '/width', self.figWidth)
			self.settings.setValue('group: ' + self.name + '/height', self.figHeight)
			self.settings.setValue('group: ' + self.name + '/spread method', self.spreadMethod)
			self.settings.setValue('group: ' + self.name + '/bin', self.numBins)
			self.settings.setValue('group: ' + self.name + '/bin size', self.histogramSize)
			self.settings.setValue('group: ' + self.name + '/show histograms', self.bShowHistograms)
			self.settings.setValue('group: ' + self.name + '/marker size', self.markerSize)
			self.settings.setValue('group: ' + self.name + '/show R2', self.bShowR2)

			self.plot(profile, statsResults)
					
if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(ScatterPlot)
	testWindow.show()
	sys.exit(app.exec_())