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

from PyQt6 import QtCore, QtGui, QtWidgets
from stamp.GUI.loadDataDlgUI import Ui_loadDataDlg

class LoadDataDlg(QtWidgets.QDialog):
	def __init__(self, preferences, parent=None, info=None):
		QtWidgets.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_loadDataDlg()
		self.ui.setupUi(self)

		self.centerWindow()
		
		self.preferences = preferences
		
		# connect signals to slots
		self.ui.tbProfileFile.clicked.connect(self.openProfileFile)
		self.ui.tbMetadataFile.clicked.connect(self.openMetadataFile)
		
	def openProfileFile(self):
		profileFile = QtWidgets.QFileDialog.getOpenFileName(self, 'Open profile', self.preferences['Last directory'], 'STAMP profile file (*.spf *.tsv *.txt);;All files (*.*)')[0]
		if profileFile != '':
			self.preferences['Last directory'] = profileFile[0:profileFile.rfind('/')]
			self.ui.txtProfileFile.setText(profileFile)
			
	def openMetadataFile(self):
		metadataFile = QtWidgets.QFileDialog.getOpenFileName(self, 'Open group metadata', self.preferences['Last directory'], 'STAMP group metadata file (*.met *.tsv *.txt);;All files (*.*)')[0]
		if metadataFile != '':
			self.preferences['Last directory'] = metadataFile[0:metadataFile.rfind('/')]
			self.ui.txtMetadataFile.setText(metadataFile)
		
	def centerWindow(self):
		screen = QtWidgets.QApplication.primaryScreen().geometry()
		size =	self.geometry()
		self.move((screen.width()-size.width())//2, (screen.height()-size.height())//2)
		
	def getProfileFile(self):
		return self.ui.txtProfileFile.text()
		
	def getMetadataFile(self):
		return self.ui.txtMetadataFile.text()
	
if __name__ == "__main__": 
	pass