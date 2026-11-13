#=======================================================================
# Author: Donovan Parks
#
# Loading plot plugins. Render plots on the GUI. Save plots to file.
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

from PyQt4 import QtCore

import os
import sys
import platform
from stamp.metagenomics.DirectoryHelper import runningExecutable, getMainDir
from stamp.GUI.plotDlg import PlotDlg

class PlotsManager:
	def __init__(self, cboPlots, plotScrollArea, defaultPlot):
		self.currentPlot = None
		self.currentPlotClass = None
		self.plotClassDict = {}
		self.plotDict = {}
		
		self.defaultPlot = defaultPlot
		
		self.cboPlots = cboPlots
		self.plotScrollArea = plotScrollArea
		
	def loadPlots(self, preferences, pluginFolder):
		os.chdir(getMainDir())
		
		pluginModulePath = pluginFolder.replace('/', '.')

		if runningExecutable():
			if platform.system() == 'Windows':
				# windows plugin folder
				pluginFolder = 'library/' + pluginFolder
			else:
				# os x plugin folder
				pluginFolder = './lib/python2.6/site-packages/' + pluginFolder
		else:		
			pluginFolder = os.path.join(os.path.split(os.path.realpath(__file__))[0], '..', '..', pluginFolder)
		
		for filename in os.listdir(pluginFolder):
			if os.path.isdir(os.path.join (pluginFolder, filename)):
				continue

			extension = filename[filename.rfind('.')+1:len(filename)]	
			if extension == 'py' and filename != '__init__.py':
				pluginModule = filename[0:filename.rfind('.')]
				theModule = __import__(pluginModulePath + pluginModule, fromlist='*')
				theClass = getattr(theModule, pluginModule)
				plot = theClass(preferences)
				
				self.plotClassDict[plot.name] = theClass
				self.plotDict[plot.name] = plot
			
		exploratoryPlots = []
		statisticalPlots = []
		for key in self.plotDict:
			if self.plotDict[key].type == 'Exploratory':
				exploratoryPlots.append(key)
			else:
				statisticalPlots.append(key)
				
		exploratoryPlots.sort(lambda a,b:cmp(a.upper(), b.upper()))
		statisticalPlots.sort(lambda a,b:cmp(a.upper(), b.upper()))
			
		for plotName in exploratoryPlots:
			self.cboPlots.addItem(plotName)
			
		self.cboPlots.insertSeparator(self.cboPlots.count())
			
		for plotName in statisticalPlots:
			self.cboPlots.addItem(plotName)
				
		self.display(self.defaultPlot, None, None)
		self.cboPlots.setCurrentIndex(self.cboPlots.findText(self.defaultPlot)) 
		
	def display(self, plotName, profile, statsResults):
		# remove current plot widget
		if self.currentPlot != None:
			widget = self.plotScrollArea.takeWidget()
			widget.setParent(None)
			del widget
		
		# add new plot widget
		self.currentPlotClass = self.plotClassDict[plotName]
		self.currentPlot = self.plotDict[plotName]
		if profile != None:
			self.currentPlot.plot(profile, statsResults)
		else:
			self.currentPlot.emptyAxis()
		self.plotScrollArea.setWidget(self.currentPlot)
		
	def checkFlags(self):
		return self.plotDict[str(self.cboPlots.currentText())]
		
	def update(self, profile, statsResults):
		self.display(str(self.cboPlots.currentText()), profile, statsResults)
		
	def reset(self, preferences):
		for plotName in self.plotClassDict:
			theClass = self.plotClassDict[plotName]
			self.plotDict[plotName] = theClass(preferences)
		
	def configure(self, profile, statsResults):
		self.currentPlot.configure(profile, statsResults)
		
	def save(self, file, dpi = 300):
		self.currentPlot.savePlot(str(file), dpi)
		
	def sendToNewWindow(self, mainWindow, profile, statsResults):
		plotDlg = PlotDlg(mainWindow)
		plotDlg.setWindowTitle(self.cboPlots.currentText())
		plotDlg.setObjectName("groupLegendDlg");
		plotDlg.setVisible(True)
		mainWindow.addDockWidget(QtCore.Qt.RightDockWidgetArea, plotDlg)
		plotDlg.setFloating(True)
		
		newPlotWindow = self.currentPlotClass(self.currentPlot.preferences)
		newPlotWindow.mirrorProperties(self.currentPlot)
		newPlotWindow.plot(profile, statsResults)
		
		w, h = newPlotWindow.get_width_height()
		plotDlg.setMaximumSize(w, h)
		if h > 800:
			h = 800
		plotDlg.resize(w, h)
		
		plotDlg.addPlot(newPlotWindow)
