#=======================================================================
# Author: Donovan Parks
#
# Dialog box used to load profile data and group metadata.
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

from PyQt4 import QtGui, QtCore
from loadDataDlgUI import Ui_loadDataDlg

class LoadDataDlg(QtGui.QDialog):
	def __init__(self, preferences, parent=None, info=None):
		QtGui.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_loadDataDlg()
		self.ui.setupUi(self)

		self.centerWindow()
		
		self.preferences = preferences
		
		# connect signals to slots
		self.connect(self.ui.tbProfileFile, QtCore.SIGNAL("clicked()"), self.openProfileFile)
		self.connect(self.ui.tbMetadataFile, QtCore.SIGNAL("clicked()"), self.openMetadataFile)
		
	def openProfileFile(self):
		profileFile = QtGui.QFileDialog.getOpenFileName(self, 'Open profile', self.preferences['Last directory'], 'STAMP profile file (*.spf *.tsv *.txt);;All files (*.*)')
		if profileFile != '':
			self.preferences['Last directory'] = profileFile[0:profileFile.lastIndexOf('/')]
			self.ui.txtProfileFile.setText(profileFile)
			
	def openMetadataFile(self):
		metadataFile = QtGui.QFileDialog.getOpenFileName(self, 'Open group metadata', self.preferences['Last directory'], 'STAMP group metadata file (*.met *.tsv *.txt);;All files (*.*)')
		if metadataFile != '':
			self.preferences['Last directory'] = metadataFile[0:metadataFile.lastIndexOf('/')]
			self.ui.txtMetadataFile.setText(metadataFile)
		
	def centerWindow(self):
		screen = QtGui.QDesktopWidget().screenGeometry()
		size =	self.geometry()
		self.move((screen.width()-size.width())/2, (screen.height()-size.height())/2)
		
	def getProfileFile(self):
		return self.ui.txtProfileFile.text()
		
	def getMetadataFile(self):
		return self.ui.txtMetadataFile.text()
	
if __name__ == "__main__": 
	pass