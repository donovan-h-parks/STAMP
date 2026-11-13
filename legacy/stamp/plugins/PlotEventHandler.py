#=======================================================================
# Author: Donovan Parks
#
# Handle mouse events for plots.
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

import math
from PyQt4 import QtGui

class PlotEventHandler:
	def __init__(self, xData, yData, toolTips, xtol=None, ytol=None):
		self.data = zip(xData, yData, toolTips)
		
		if xtol == None:
			self.xtol = (max(xData) - min(xData)) / 50
		else:
			self.xtol = xtol
			
		if ytol == None:
			self.ytol = (max(yData) - min(yData)) / 50
		else:
			self.ytol = ytol
	
	def distance(self, x1, x2, y1, y2):
		return( (x1 - x2)**2 + (y1 - y2)**2 )

	def __call__(self, event):
		clickX = event.xdata
		clickY = event.ydata

		toolTips = []
		for x,y,tip in self.data:
			if event.xdata != None and event.ydata != None:
				if (clickX - self.xtol < x < clickX + self.xtol) and (clickY - self.ytol < y < clickY + self.ytol):
					toolTips.append( (self.distance(x, clickX, y, clickY), tip) )
				
		if len(toolTips) > 0:
			toolTips.sort()
			distance, tip = toolTips[0]
			msgBox = QtGui.QMessageBox()
			
			icon = QtGui.QIcon()
			icon.addPixmap(QtGui.QPixmap(":/icons/icons/programIcon.png"), QtGui.QIcon.Normal, QtGui.QIcon.Off)
			msgBox.setWindowIcon(icon)
			
			msgBox.setWindowTitle('Tooltip')
			msgBox.setText(tip)
			msgBox.exec_()
			
class MultiPlotEventHandler:
	def __init__(self, xData, yData, axes, toolTips):
		self.data = []
		for i in xrange(0, len(xData)):
			self.data.append(zip(xData[i], yData[i], toolTips))
		
		self.xtol = []
		self.ytol = []
		for i in xrange(0, len(xData)):
			self.xtol.append((max(xData[i]) - min(xData[i])) / 50)
			self.ytol.append((max(yData[i]) - min(yData[i])) / 50)
			
		self.axes = axes
	
	def distance(self, x1, x2, y1, y2):
		return( math.sqrt( (x1 - x2)**2 + (y1 - y2)**2 ) )

	def __call__(self, event):
		clickX = event.xdata
		clickY = event.ydata

		toolTips = []
		if event.xdata != None and event.ydata != None:
			for i in xrange(0, len(self.data)):
				if event.inaxes == self.axes[i]:
					for x,y,tip in self.data[i]:
						if (clickX - self.xtol[i] < x < clickX + self.xtol[i]) and (clickY - self.ytol[i] < y < clickY + self.ytol[i]):
							toolTips.append( (self.distance(x, clickX, y, clickY), tip) )
				
		if len(toolTips) > 0:
			toolTips.sort()
			distance, tip = toolTips[0]
			msgBox = QtGui.QMessageBox()
			
			icon = QtGui.QIcon()
			icon.addPixmap(QtGui.QPixmap(":/icons/icons/programIcon.png"), QtGui.QIcon.Normal, QtGui.QIcon.Off)
			msgBox.setWindowIcon(icon)
			
			msgBox.setWindowTitle('Tooltip')
			msgBox.setText(tip)
			msgBox.exec_()
