#=======================================================================
# Author: Donovan Parks
#
# Simple exploratory scatter plot.
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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.	See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.	If not, see <http://www.gnu.org/licenses/>.
#=======================================================================

import sys

from plugins.samples.AbstractSamplePlotPlugin import AbstractSamplePlotPlugin, TestWindow, ConfigureDialog


class MyScatterPlot(AbstractSamplePlotPlugin):
	'''
	Simple exploratory scatter plot.
	'''
	def __init__(self, preferences, parent=None):
		AbstractSamplePlotPlugin.__init__(self, preferences, parent)
		self.preferences = preferences

		self.name = 'My scatter plot'

		self.figWidth = 7.0
		self.figHeight = 7.0
		
		self.sampleName1 = ''
		self.sampleName2 = ''
		
	def plot(self, profile, statsResults):
		# Colour of plot elements
		profile1Colour = str(self.preferences['Sample 1 colour'].name())
		profile2Colour = str(self.preferences['Sample 2 colour'].name())
		
		# Set sample names
		if self.sampleName1 == '' and self.sampleName2 == '':
			self.sampleName1 = statsResults.profile.sampleNames[0]
			self.sampleName2 = statsResults.profile.sampleNames[1]
				
		# Get data to plot		
		field1 = statsResults.getColumn('RelFreq1')
		field2 = statsResults.getColumn('RelFreq2')
					
		# Set figure size
		self.fig.clear()
		self.fig.set_size_inches(self.figWidth, self.figHeight)	
		axesScatter = self.fig.add_subplot(111)
		
		# Set visual properties of all points
		colours = []
		for i in xrange(0, len(field1)):
			if field1[i] > field2[i]:
				colours.append(profile1Colour)
			else:
				colours.append(profile2Colour)
					 
		# Create scatter plot
		axesScatter.scatter(field1, field2, c=colours)

		# Update plot
		self.updateGeometry()
		self.draw()
