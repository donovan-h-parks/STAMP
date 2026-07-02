#=======================================================================
# Author: Donovan Parks
#
# Dialog box used to assign COG categories to IMG/M profiles.
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
from stamp.metagenomics.fileIO.COG_IO import COG_IO
from stamp.GUI.assignCOGUI import Ui_AssignCOGsDlg

class AssignCOGsDlg(QtWidgets.QDialog):
	def __init__(self, preferences, parent=None):
		QtWidgets.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_AssignCOGsDlg()
		self.ui.setupUi(self)

		self.centerWindow()
		
		self.preferences = preferences
		
		self.ui.btnLoadProfiles.clicked.connect(self.loadProfiles)
		self.ui.btnCreateProfile.clicked.connect(self.createProfile)
		self.ui.btnCancel.clicked.connect(self.accept)
		
		self.inputProfile = []
		
	def loadProfiles(self):
		self.inputProfile = QtWidgets.QFileDialog.getOpenFileName(self, 'Load profile', self.preferences['Last directory'], 'IMG/M profiles (*.xls *.tsv);;All files (*.*)')[0]
		if self.inputProfile != '':
			self.preferences['Last directory'] = self.inputProfile[0:self.inputProfile.rfind('/')]
			self.ui.txtInputProfile.setText(self.inputProfile)
			self.ui.btnCreateProfile.setEnabled(True)
			
	def createProfile(self):
		# get filename to save STAMP profile to
		stampFilename = QtWidgets.QFileDialog.getSaveFileName(self, 'Save STAMP profile...', self.preferences['Last directory'],'STAMP profile file(*.spf);;All files(*.*)')[0]
		if stampFilename == '':
			return
			
		self.preferences['Last directory'] = stampFilename[0:stampFilename.rfind('/')]
		
		cogIO = COG_IO()			
		cogIO.appendCategories(str(self.inputProfile), str(self.ui.cboMultiCogTreatment.currentText()), str(stampFilename), self.preferences)
		
		self.accept()

	def centerWindow(self):
		screen = QtWidgets.QApplication.primaryScreen().geometry()
		size =	self.geometry()
		self.move((screen.width()-size.width())//2, (screen.height()-size.height())//2)
