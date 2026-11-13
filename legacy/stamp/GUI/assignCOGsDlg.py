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

from PyQt4 import QtGui, QtCore
from stamp.metagenomics.fileIO.COG_IO import COG_IO
from assignCOGUI import Ui_AssignCOGsDlg

class AssignCOGsDlg(QtGui.QDialog):
	def __init__(self, preferences, parent=None):
		QtGui.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_AssignCOGsDlg()
		self.ui.setupUi(self)

		self.centerWindow()
		
		self.preferences = preferences
		
		QtCore.QObject.connect(self.ui.btnLoadProfiles, QtCore.SIGNAL("clicked()"), self.loadProfiles)
		QtCore.QObject.connect(self.ui.btnCreateProfile, QtCore.SIGNAL("clicked()"), self.createProfile)
		QtCore.QObject.connect(self.ui.btnCancel, QtCore.SIGNAL("clicked()"), self.accept)
		
		self.inputProfile = []
		
	def loadProfiles(self):
		self.inputProfile = QtGui.QFileDialog.getOpenFileName(self, 'Load profile', self.preferences['Last directory'], 'IMG/M profiles (*.xls *.tsv);;All files (*.*)')
		if self.inputProfile != '':
			self.preferences['Last directory'] = self.inputProfile[0:self.inputProfile.lastIndexOf('/')]
			self.ui.txtInputProfile.setText(self.inputProfile)
			self.ui.btnCreateProfile.setEnabled(True)
			
	def createProfile(self):
		# get filename to save STAMP profile to
		stampFilename = QtGui.QFileDialog.getSaveFileName(self, 'Save STAMP profile...', self.preferences['Last directory'],'STAMP profile file(*.spf);;All files(*.*)')
		if stampFilename == '':
			return
			
		self.preferences['Last directory'] = stampFilename[0:stampFilename.lastIndexOf('/')]
		
		cogIO = COG_IO()			
		cogIO.appendCategories(str(self.inputProfile), str(self.ui.cboMultiCogTreatment.currentText()), str(stampFilename), self.preferences)
		
		self.accept()

	def centerWindow(self):
		screen = QtGui.QDesktopWidget().screenGeometry()
		size =	self.geometry()
		self.move((screen.width()-size.width())/2, (screen.height()-size.height())/2)
