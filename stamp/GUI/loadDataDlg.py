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
import os

from PyQt5 import QtGui, QtCore,QtWidgets
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

	import os

	def openProfileFile(self):
		# 1. Unpack the tuple: (file_path, filter_used)
		fileName, _ = QtWidgets.QFileDialog.getOpenFileName(
			self,
			'Open profile',
			self.preferences.get('Last directory', ''),
			'STAMP profile file (*.spf *.tsv *.txt);;All files (*.*)'
		)

		# 2. Check if a file was actually selected (fileName won't be empty)
		if fileName:
			# 3. Use os.path.dirname to get the folder path
			self.preferences['Last directory'] = os.path.dirname(fileName)

			# 4. Update the UI text box
			self.ui.txtProfileFile.setText(fileName)

	import os  # Make sure this is at the very top of your file

	def openMetadataFile(self):
		# Unpack the tuple: fileName is the first item, _ ignores the filter string
		fileName, _ = QtWidgets.QFileDialog.getOpenFileName(
			self,
			'Open group metadata',
			self.preferences.get('Last directory', ''),
			'STAMP group metadata file (*.met *.tsv *.txt);;All files (*.*)'
		)

		# In PyQt5/Python 3, fileName will be an empty string if the user cancels
		if fileName:
			# Use os.path.dirname to get the directory (replaces lastIndexOf)
			self.preferences['Last directory'] = os.path.dirname(fileName)
			self.ui.txtMetadataFile.setText(fileName)
		
	def centerWindow(self):
		# Modern way if the old way fails
		screen = QtWidgets.QApplication.primaryScreen().geometry()
		size = self.geometry()
		self.move((screen.width() - size.width()) // 2,
				  (screen.height() - size.height()) // 2)
		
	def getProfileFile(self):
		return self.ui.txtProfileFile.text()
		
	def getMetadataFile(self):
		return self.ui.txtMetadataFile.text()
	
if __name__ == "__main__": 
	pass