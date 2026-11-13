#=======================================================================
# Author: Donovan Parks
#
# Box plot for two groups.
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
from stamp.plugins.groups.plots.configGUI.BoxPlotUI import Ui_BoxPlotDialog

from matplotlib.patches import Polygon

from matplotlib.artist import setp

import numpy as np

class BoxPlot(AbstractGroupPlotPlugin):
	'''
	Box plot for two groups.
	'''
	def __init__(self, preferences, parent=None):
		AbstractGroupPlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences
	 
		self.name = 'Box plot'
		self.type = 'Exploratory'
		
		self.settings = preferences['Settings']
		self.figWidth = self.settings.value('group: ' + self.name + '/width', 7.0).toDouble()[0]
		self.figHeight = self.settings.value('group: ' + self.name + '/height', 7.0).toDouble()[0]
		self.fieldToPlot = self.settings.value('group: ' + self.name + '/field to plot', 'Proportion of sequences (%)').toString()
		self.bShowAverages = self.settings.value('group: ' + self.name + '/show averages', True).toBool()
		self.bShowPvalue = self.settings.value('group: ' + self.name + '/show p-value', True).toBool()

	def mirrorProperties(self, plotToCopy):
		super(BoxPlot, self).mirrorProperties(plotToCopy)
		
		self.figWidth = plotToCopy.figWidth
		self.figHeight = plotToCopy.figHeight
		
		self.fieldToPlot = plotToCopy.fieldToPlot
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
		
		# *** Set figure size
		self.fig.clear()
		self.fig.set_size_inches(self.figWidth, self.figHeight)	
		
		padding = 0.25					 # inches
		xOffsetFigSpace = (0.4 + padding)/self.figWidth
		yOffsetFigSpace = (0.3 + padding)/self.figHeight
		axesBoxPlot = self.fig.add_axes([xOffsetFigSpace, yOffsetFigSpace,
																		1.0 - xOffsetFigSpace - (2*padding)/self.figWidth, 1.0 - yOffsetFigSpace - (2*padding)/self.figHeight])
			
		# box plot
		data = [data1, data2]
		bp = axesBoxPlot.boxplot(data, notch=0, sym='k+', vert=1, whis=1.5)
		setp(bp['boxes'], color='black')
		setp(bp['whiskers'], color='black',linestyle='-')
		setp(bp['medians'], color='black')
		
		# fill boxes with desired colors
		colours = [group1Colour, group2Colour]

		for i in xrange(0, len(data)):
			# get box coordinates
			box = bp['boxes'][i]
			boxCoords = zip(box.get_xdata()[0:5],box.get_ydata()[0:5])
			
			# colour in box
			boxPolygon = Polygon(boxCoords, facecolor=colours[i])
			axesBoxPlot.add_patch(boxPolygon)
			
			# draw the median lines back over what we just filled in
			med = bp['medians'][i]
			axesBoxPlot.plot(med.get_xdata()[0:2], med.get_ydata()[0:2], 'k')
		
		# mark average
		if self.bShowAverages:
			for i in xrange(0,2):
				med = bp['medians'][i]
				axesBoxPlot.plot([np.average(med.get_xdata())], [np.average(data[i])], color='w', marker='*', markeredgecolor='k')
				
		# *** P-value label
		if self.bShowPvalue and statsResults.profile != None:
			pValueStr = statsResults.getFeatureStatisticAsStr(feature, 'pValuesCorrected')
			axesBoxPlot.text(1.0, 1.0, r'$p$ = ' + pValueStr, horizontalalignment='right', verticalalignment='bottom', transform=axesBoxPlot.transAxes)
		
		# *** Prettify scatter plot
		if self.preferences['Truncate feature names']:
			length = self.preferences['Length of truncated feature names']
			if len(feature) > length+3:
					feature = feature[0:length] + '...'
					
		axesBoxPlot.set_title(feature)
		axesBoxPlot.set_ylabel(self.fieldToPlot)
		axesBoxPlot.set_xticklabels([profile.groupName1, profile.groupName2])
			
		for a in axesBoxPlot.yaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
				
		for a in axesBoxPlot.xaxis.majorTicks:
			a.tick1On=True
			a.tick2On=False
			
		for line in axesBoxPlot.yaxis.get_ticklines(): 
			line.set_color(axesColour)
				
		for line in axesBoxPlot.xaxis.get_ticklines(): 
			line.set_color(axesColour)
			
		for loc, spine in axesBoxPlot.spines.iteritems():
			if loc in ['right','top']:
				spine.set_color('none') 
			else:
				spine.set_color(axesColour)
					
		#axesBoxPlot.yaxis.grid(True, linestyle='-', which='major', color='lightgrey', alpha=0.5)
		#axesBoxPlot.set_axisbelow(True)
		
		self.updateGeometry()
		self.draw()

	def configure(self, profile, statsResults):
		configDlg = ConfigureDialog(Ui_BoxPlotDialog)
		
		configDlg.ui.cboFieldToPlot.setCurrentIndex(configDlg.ui.cboFieldToPlot.findText(self.fieldToPlot))

		configDlg.ui.spinFigWidth.setValue(self.figWidth)
		configDlg.ui.spinFigHeight.setValue(self.figHeight)
		
		configDlg.ui.chkShowAverage.setChecked(self.bShowAverages)
		
		configDlg.ui.chkShowPvalue.setChecked(self.bShowPvalue)
				
		if configDlg.exec_() == QtGui.QDialog.Accepted:
			self.fieldToPlot = str(configDlg.ui.cboFieldToPlot.currentText())
			
			self.figWidth = configDlg.ui.spinFigWidth.value()
			self.figHeight = configDlg.ui.spinFigHeight.value()
			
			self.bShowAverages = configDlg.ui.chkShowAverage.isChecked()
			
			self.bShowPvalue = configDlg.ui.chkShowPvalue.isChecked()
			
			self.settings.setValue('group: ' + self.name + '/column width', self.figWidth)
			self.settings.setValue('group: ' + self.name + '/height', self.figHeight)
			self.settings.setValue('group: ' + self.name + '/field to plot', self.fieldToPlot)
			self.settings.setValue('group: ' + self.name + '/show averages', self.bShowAverages)
			self.settings.setValue('group: ' + self.name + '/show p-value', self.bShowPvalue)
			
			self.plot(profile, statsResults)
					
if __name__ == "__main__": 
	app = QtGui.QApplication(sys.argv)
	testWindow = TestWindow(ProfileScatterPlot)
	testWindow.show()
	sys.exit(app.exec_())