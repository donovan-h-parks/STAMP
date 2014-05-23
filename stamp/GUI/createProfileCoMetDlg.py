#=======================================================================
# Author: Donovan Parks
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

import string

from PyQt4 import QtGui, QtCore
from createProfileCoMetUI import Ui_CreateProfileCoMetDlg

class ProfileRow():
	def __init__(self):
		countData = []
		hierarchy = []

class CreateProfileCoMetDlg(QtGui.QDialog):
	def __init__(self, preferences, parent=None):
		QtGui.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_CreateProfileCoMetDlg()
		self.ui.setupUi(self)
		
		self.preferences = preferences

		self.centerWindow()
		
		QtCore.QObject.connect(self.ui.btnLoadProfiles, QtCore.SIGNAL("clicked()"), self.loadProfiles)
		QtCore.QObject.connect(self.ui.btnCreateProfile, QtCore.SIGNAL("clicked()"), self.createProfile)
		QtCore.QObject.connect(self.ui.btnCancel, QtCore.SIGNAL("clicked()"), self.accept)
		
		self.selectedFiles = []
			
	def loadProfiles(self):
		selectedFiles = QtGui.QFileDialog.getOpenFileNames(self, 'Load profiles', self.preferences['Last directory'], 'CoMet profiles (*.txt);;All files (*.*)')

		if len(selectedFiles) > 0:
			self.preferences['Last directory'] = selectedFiles[0][0:selectedFiles[0].lastIndexOf('/')]
			for file in selectedFiles:
				self.selectedFiles.append(str(file))
				self.ui.lstSelectedProfiles.addItem(file)
			self.ui.btnCreateProfile.setEnabled(True)
	
	def createProfile(self):
		# get filename to save STAMP profile to
		stampFilename = QtGui.QFileDialog.getSaveFileName(self, 'Save STAMP profile...', self.preferences['Last directory'],'STAMP profile file(*.spf);;All files(*.*)')
		if stampFilename == '':
			return
		
		# get profile information from each file
		profileDict = {} 
		profileIndex = 0
		sampleNames = []
		for file in self.selectedFiles:
			fin = open(file, 'U')
			data = map(string.strip, fin.readlines())
			fin.close()
			
			sampleName = file[file.rfind('/')+1:file.find('.')]
			if sampleName.find('_') != -1:
				sampleName = sampleName[sampleName.find('_')+1:]
			sampleNames.append(sampleName)
			
			# add profile info
			categories = set([])
			for line in data:
				if line == "":
						continue	# skip blank lines
				
				firstSpaceIndex = line.find(' ')
				lastSemiColonIndex = line.rfind(':')
				openParanethsisIndex = line.find('(')
				closeParanethsisIndex = line.rfind(')')
				
				if firstSpaceIndex == -1 or lastSemiColonIndex == -1 or openParanethsisIndex == -1 or closeParanethsisIndex == -1:
					QtGui.QMessageBox.information(self, 'Unrecognized file format', 'Your file does not appear to be a valid CoMet profile.')
					return
				
				category = line[openParanethsisIndex+1:closeParanethsisIndex].strip()
				count = float(line[lastSemiColonIndex+1:])

				hierarchy = [category]
				
				row = profileDict.get(category, None)
				if row == None:
					row = ProfileRow()
					row.countData = [0] * len(self.selectedFiles)
					row.hierarchy = hierarchy
					profileDict[category] = row
					
				row.countData[profileIndex] += count
				
			profileIndex += 1
			
		# write out STAMP profile
		try:
			fout = open(stampFilename, 'w')
		except IOError:
			QtGui.QMessageBox.information(self, 'Failed to save STAMP profile', 'Write permission for file denied.', QtGui.QMessageBox.Ok)
			return

		fout.write('Category')

		for sampleName in sampleNames:
			fout.write('\t' + sampleName)
		fout.write('\n')
			
		for key in profileDict.keys():
			row = profileDict[key]
			for h in row.hierarchy:
				fout.write(h + '\t')
			
			fout.write(str(row.countData[0]))
			for c in row.countData[1:]:
				fout.write('\t' + str(c))
			fout.write('\n')
			
		fout.close()
				
		self.accept()

	def centerWindow(self):
		screen = QtGui.QDesktopWidget().screenGeometry()
		size =	self.geometry()
		self.move((screen.width()-size.width())/2, (screen.height()-size.height())/2)
