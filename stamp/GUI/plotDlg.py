#=======================================================================
# Author: Donovan Parks
#
# Dock widget containing customizable group legend.
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

from PyQt6 import QtCore, QtGui, QtWidgets
from stamp.GUI.plotDlgUI import Ui_PlotDlg

class PlotDlg(QtWidgets.QDockWidget):
	def __init__(self, parent=None, info=None):
		QtWidgets.QWidget.__init__(self, parent)
		
		self.ui = Ui_PlotDlg()
		self.ui.setupUi(self)
		
		self.topLevelChanged.connect(self.onTopLevelChanged)
		self.dockLocationChanged.connect(self.onDockLocationChanged)
		
		self.plot = None

	def addPlot(self, plot):
		self.ui.scrollArea.setWidget(plot)
		self.plot = plot
		
	def onTopLevelChanged(self, bFloating):
		if self.plot == None:
			return
			
		if bFloating:
			w, h = self.plot.get_width_height()
			self.setMaximumSize(w, h)
			if h > 800:
				h = 800
			self.resize(w, h)
		
	def onDockLocationChanged(self):
		self.setMaximumSize(10000, 10000)

if __name__ == "__main__": 
	pass