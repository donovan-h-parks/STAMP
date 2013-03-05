#=======================================================================
# Author: Donovan Parks
#
# Dialog box used to set program preferences.
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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.	If not, see <http://www.gnu.org/licenses/>.
#=======================================================================

from PyQt4 import QtGui, QtCore
from preferencesUI import Ui_preferencesDlg

import math

class PreferencesDlg(QtGui.QDialog):
	def __init__(self, parent=None, info=None):
		QtGui.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_preferencesDlg()
		self.ui.setupUi(self)

		self.centerWindow()
		
		self.tuncFeatureNameChanged()
		
		# connect signals to slots
		self.connect(self.ui.chkTruncateFeatureNames, QtCore.SIGNAL('toggled(bool)'), self.tuncFeatureNameChanged)
		self.connect(self.ui.btnOK, QtCore.SIGNAL("clicked()"), self.accept)
		self.connect(self.ui.btnAxesColour, QtCore.SIGNAL("clicked()"), self.setAxesColour)
		self.connect(self.ui.btnAllOtherSamplesColour, QtCore.SIGNAL("clicked()"), self.setAllOtherSamplesColour)
		
	def centerWindow(self):
		screen = QtGui.QDesktopWidget().screenGeometry()
		size =	self.geometry()
		self.move((screen.width()-size.width())/2, (screen.height()-size.height())/2)
				
	def tuncFeatureNameChanged(self):
		self.ui.spinFeatureNameLength.setEnabled(self.ui.chkTruncateFeatureNames.isChecked())

	def getAxesColour(self):
		return self.axesColour

	def setAxesButtonColour(self, colour):
		self.axesColour = colour
		colourStr = str(colour.red()) + ',' + str(colour.green()) + ',' + str(colour.blue())
		self.ui.btnAxesColour.setStyleSheet('* { background-color: rgb(' + colourStr + ') }')

	def setAxesColour(self):
		colour = QtGui.QColorDialog.getColor(self.axesColour, self, 'Axis colour')

		if colour.isValid():
			self.axesColour = colour
			colourStr = str(colour.red()) + ',' + str(colour.green()) + ',' + str(colour.blue())
			self.ui.btnAxesColour.setStyleSheet('* { background-color: rgb(' + colourStr + ') }')
			
	def getAllOtherSamplesColour(self):
		return self.allOtherSamplesColour
		
	def setAllOtherSamplesButtonColour(self, colour):
		self.allOtherSamplesColour = colour
		colourStr = str(colour.red()) + ',' + str(colour.green()) + ',' + str(colour.blue())
		self.ui.btnAllOtherSamplesColour.setStyleSheet('* { background-color: rgb(' + colourStr + ') }')

	def setAllOtherSamplesColour(self):
		colour = QtGui.QColorDialog.getColor(self.allOtherSamplesColour, self, 'All other samples colour')

		if colour.isValid():
			self.allOtherSamplesColour = colour
			colourStr = str(colour.red()) + ',' + str(colour.green()) + ',' + str(colour.blue())
			self.ui.btnAllOtherSamplesColour.setStyleSheet('* { background-color: rgb(' + colourStr + ') }')
		
	def setMinimumReportedPValue(self, exponent):
		self.ui.spinMinPvalue.setValue(-exponent)
		
	def getMinimumReportedPValue(self):
		exponent = self.ui.spinMinPvalue.value()
		return -exponent
if __name__ == "__main__": 
	pass