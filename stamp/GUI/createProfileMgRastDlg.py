#=======================================================================
# Author: Donovan Parks
#
# Dialog box for creating STAMP profiles.
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

from PyQt6 import QtCore, QtGui, QtWidgets
from stamp.GUI.createProfileMgRastUI import Ui_CreateProfileMgRastDlg

from stamp.GUI.customizeHeadingsDlg import CustomizeHeadingsDlg

class ProfileRow():
	def __init__(self):
		countData = []
		hierarchy = []

class CreateProfileMgRastDlg(QtWidgets.QDialog):
	def __init__(self, preferences, parent=None):
		QtWidgets.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_CreateProfileMgRastDlg()
		self.ui.setupUi(self)

		self.centerWindow()
		
		self.preferences = preferences
		
		self.ui.btnLoadProfiles.clicked.connect(self.loadProfiles)
		self.ui.btnCustomizeHeadings.clicked.connect(self.customizeHeadings)
		self.ui.btnCreateProfile.clicked.connect(self.createProfile)
		self.ui.btnCancel.clicked.connect(self.accept)
		
		self.headings = []
		
		self.selectedFile = ''

	def loadProfiles(self):
		self.selectedFile = QtWidgets.QFileDialog.getOpenFileName(self, 'Load profile', self.preferences['Last directory'], 'MG-RAST profile (*.tsv);;All files (*.*)')[0]

		if self.selectedFile != '':
			self.preferences['Last directory'] = self.selectedFile[0:self.selectedFile.rfind('/')]
			
			# read profile information from file
			fin = open(self.selectedFile)
			self.data = [__s.strip() for __s in fin.readlines()]
			fin.close()
			
			# get header data
			self.header = self.data[0].split('\t')
			
			if self.header[0].strip() != 'metagenome':
				QtWidgets.QMessageBox.information(self, 'Failed to parse MG-RAST profile', "This file does not appear to be a valid MG-RAST profile as it does not begin with a 'metagenome' column.", QtWidgets.QMessageBox.StandardButton.Ok)
				return
			
			if 'abundance' not in self.header:
				QtWidgets.QMessageBox.information(self, 'Failed to parse MG-RAST profile', "This file does not appear to be a valid MG-RAST profile as it does not contain an 'abundance' column.", QtWidgets.QMessageBox.StandardButton.Ok)
				return
			
			if self.header[1] != 'level 1' and self.header[1] != 'source' and self.header[1] != 'domain': 
				QtWidgets.QMessageBox.information(self, 'Failed to parse MG-RAST profile', "This file does not appear to be a valid MG-RAST profile as the second column is not 'level 1', 'source', or 'domain'.", QtWidgets.QMessageBox.StandardButton.Ok)
				return
				
			if self.header[1] == 'level 1' or self.header[1] == 'domain':
				self.startIndex = 1
			else:
				self.startIndex = 2
					
			for i in range(self.startIndex, self.header.index('abundance')):
				self.headings.append(self.header[i])
					
			for i in range(0, 8-len(self.headings)):
				self.headings.append('')
		
			self.ui.btnCustomizeHeadings.setEnabled(True)
			self.ui.btnCreateProfile.setEnabled(True)
			
	def customizeHeadings(self):
		customizeHeadingsDlg = CustomizeHeadingsDlg(self) 
		
		numHeadings = len(self.headings)
		if '' in self.headings:
			numHeadings = self.headings.index('')
		customizeHeadingsDlg.ui.txtInfo.setText('This MG-RAST profiles consists of ' + str(numHeadings) + ' hierarchical levels.')
	 
		customizeHeadingsDlg.ui.txtLevel1.setText(self.headings[0])
		customizeHeadingsDlg.ui.txtLevel2.setText(self.headings[1])
		customizeHeadingsDlg.ui.txtLevel3.setText(self.headings[2])
		customizeHeadingsDlg.ui.txtLevel4.setText(self.headings[3])
		customizeHeadingsDlg.ui.txtLevel5.setText(self.headings[4])
		customizeHeadingsDlg.ui.txtLevel6.setText(self.headings[5])
		customizeHeadingsDlg.ui.txtLevel7.setText(self.headings[6])
		customizeHeadingsDlg.ui.txtLevel8.setText(self.headings[7])
					 
		if customizeHeadingsDlg.exec() == QtWidgets.QDialog.DialogCode.Accepted:
			self.headings[0] = customizeHeadingsDlg.ui.txtLevel1.text()
			self.headings[1] = customizeHeadingsDlg.ui.txtLevel2.text()
			self.headings[2] = customizeHeadingsDlg.ui.txtLevel3.text()
			self.headings[3] = customizeHeadingsDlg.ui.txtLevel4.text()
			self.headings[4] = customizeHeadingsDlg.ui.txtLevel5.text()
			self.headings[5] = customizeHeadingsDlg.ui.txtLevel6.text()
			self.headings[6] = customizeHeadingsDlg.ui.txtLevel7.text()
			self.headings[7] = customizeHeadingsDlg.ui.txtLevel8.text()
	
	def createProfile(self):
		splitCh = '\t'
	
		# get filename to save STAMP profile to
		stampFilename = QtWidgets.QFileDialog.getSaveFileName(self, 'Save STAMP profile...', self.preferences['Last directory'], 'STAMP profile file(*.spf);;All files(*.*)')[0]
		if stampFilename == '':
			return
			
		self.preferences['Last directory'] = stampFilename[0:stampFilename.rfind('/')]

		# set profile specific parsing information
		hierarchyStartIndex = self.startIndex
		dataIndex = self.header.index('abundance')

		# determine samples in profile
		sampleNames = []
		for i in range(1, len(self.data)):
			sampleId = self.data[i].split(splitCh)[0]
			if sampleId not in sampleNames:
				sampleNames.append(sampleId)
				
		# add profile info
		profileDict = {}
		
		parentMap = {}
		for i in range(1, dataIndex-hierarchyStartIndex):
			parentMap[i] = {}
			
		for i in range(1, len(self.data)):
			if self.data[i] == "":
				continue	# skip blank lines
			
			lineSplit = self.data[i].split(splitCh)
			if len(lineSplit) <= dataIndex:
				QtWidgets.QMessageBox.information(self, 'Unrecognized file format', 'Your file does not appear to be a valid MG-RAST profile.')
				return
			
			count = int(lineSplit[dataIndex])
			hierarchy = lineSplit[hierarchyStartIndex:dataIndex]
			
			# replace '-' categories with parent
			for i in range(1, len(hierarchy)):
				if hierarchy[i] == '-':
					if self.header[1] == 'domain':
						if 'Unclassified' not in hierarchy[i-1]:
							hierarchy[i] = 'Unclassified ' + hierarchy[i-1]
						else:
							hierarchy[i] = hierarchy[i-1]
					else:
						hierarchy[i] = hierarchy[i-1]
					
			# force MG-RAST profile to be strictly tree-like
			for i in range(1, len(hierarchy)): 
				parent = '-'.join(hierarchy[0:i])
				child = hierarchy[i]
				
				currentParentMap = parentMap[i]
				parentList = currentParentMap.get(child, [])
				if parent not in parentList:
					parentList.append(parent)

				parentMap[i][child] = parentList
				
				parentIndex = parentList.index(parent)
				
				if parentIndex != 0:
					newChild = child + ' - #' + str(parentIndex)
					hierarchy[i] = newChild

			# add to profile
			sampleId = lineSplit[0]
			profileIndex = sampleNames.index(sampleId)
			
			row = profileDict.get(hierarchy[-1], None)
			if row == None:
				row = ProfileRow()
				row.countData = [0] * len(sampleNames)
				row.hierarchy = hierarchy
				profileDict[hierarchy[-1]] = row

			row.countData[profileIndex] += count

		# write out STAMP profile
		try:
			fout = open(stampFilename, 'w')
		except IOError:
			QtWidgets.QMessageBox.information(self, 'Failed to save STAMP profile', 'Write permission for file denied.', QtWidgets.QMessageBox.StandardButton.Ok)
			return

		fout.write(self.headings[0])
		for heading in self.headings[1:(dataIndex-hierarchyStartIndex)]:
			fout.write('\t' + heading)
			
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
		screen = QtWidgets.QApplication.primaryScreen().geometry()
		size =	self.geometry()
		self.move((screen.width()-size.width())//2, (screen.height()-size.height())//2)
