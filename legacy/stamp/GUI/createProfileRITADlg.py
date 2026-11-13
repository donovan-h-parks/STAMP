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
from createProfileRITA_UI import Ui_CreateProfileRITADlg

from stamp.GUI.customizeHeadingsDlg import CustomizeHeadingsDlg

class ProfileRow():
	def __init__(self):
		countData = []
		hierarchy = []

class CreateProfileRITADlg(QtGui.QDialog):
	def __init__(self, preferences, parent=None):
		QtGui.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_CreateProfileRITADlg()
		self.ui.setupUi(self)
		
		self.preferences = preferences

		self.centerWindow()
		
		QtCore.QObject.connect(self.ui.btnLoadProfiles, QtCore.SIGNAL("clicked()"), self.loadProfiles)
		QtCore.QObject.connect(self.ui.btnCreateProfile, QtCore.SIGNAL("clicked()"), self.createProfile)
		QtCore.QObject.connect(self.ui.btnCancel, QtCore.SIGNAL("clicked()"), self.accept)
		
		self.selectedFiles = []
			
	def loadProfiles(self):
		selectedFiles = QtGui.QFileDialog.getOpenFileNames(self, 'Load profiles', self.preferences['Last directory'], 'RITA profiles (*.txt);;All files (*.*)')

		if len(selectedFiles) > 0:
			self.preferences['Last directory'] = selectedFiles[0][0:selectedFiles[0].lastIndexOf('/')]
			for file in selectedFiles:
				self.selectedFiles.append(str(file))
				self.ui.lstSelectedProfiles.addItem(file)
			self.ui.btnCreateProfile.setEnabled(True)
	
	def createProfile(self):
		# get filename to save STAMP profile to
		stampFilename = QtGui.QFileDialog.getSaveFileName(self, 'Save STAMP profile...', self.preferences['Last directory'], 'STAMP profile file(*.spf);;All files(*.*)')
		if stampFilename == '':
			return
			
		# get checked groups
		checkedGroups = []
		if self.ui.chkNB_DBLASTN.isChecked():
			checkedGroups.append('NB and D-BLASTN')
		if self.ui.chkDBLASTN.isChecked():
			checkedGroups.append('D-BLASTN ratio')
		if self.ui.chkNB_BLASTN.isChecked():
			checkedGroups.append('NB and BLASTN')
		if self.ui.chkBLASTN.isChecked():
			checkedGroups.append('BLASTN ratio')
		if self.ui.chkNB_BLASTX.isChecked():
			checkedGroups.append('NB and BLASTX')
		if self.ui.chkBLASTX.isChecked():
			checkedGroups.append('BLASTX ratio')
		if self.ui.chkNB.isChecked():
			checkedGroups.append('NB ratio')
			
		# get profile information from each file
		profileDict = {} 
		profileIndex = 0
		sampleNames = []
		mostSpecificRankIndex = 0
		ranks = ['DOMAIN', 'PHYLUM', 'CLASS', 'ORDER', 'FAMILY', 'GENUS', 'SPECIES']
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
			for i in xrange(1, len(data)):
				line = data[i]
				if line == "":
						continue	# skip blank lines
				
				lineSplit = line.split('\t')
				groupName = lineSplit[2]
				rank = lineSplit[3]
				
				if groupName not in checkedGroups:
					continue
				
				if ranks.index(rank) > mostSpecificRankIndex:
					mostSpecificRankIndex = ranks.index(rank)

				hierarchy = ';'.join(lineSplit[4:])
				
				row = profileDict.get(hierarchy, None)
				if row == None:
					row = ProfileRow()
					row.countData = [0] * len(self.selectedFiles)
					row.hierarchy = hierarchy
					profileDict[hierarchy] = row
					
				row.countData[profileIndex] += 1
				
			profileIndex += 1
			
		# write out STAMP profile
		try:
			fout = open(stampFilename, 'w')
		except IOError:
			QtGui.QMessageBox.information(self, 'Failed to save STAMP profile', 'Write permission for file denied.', QtGui.QMessageBox.Ok)
			return

		mostSpecificRank = ranks[mostSpecificRankIndex]
		if mostSpecificRank == 'DOMAIN':
			fout.write('Domain')
		elif mostSpecificRank == 'PHYLUM':
			fout.write('Domain\tPhylum')
		elif mostSpecificRank == 'CLASS':
			fout.write('Domain\tPhylum\tClass')
		elif mostSpecificRank == 'ORDER':
			fout.write('Domain\tPhylum\tClass\tOrder')
		elif mostSpecificRank == 'FAMILY':
			fout.write('Domain\tPhylum\tClass\tOrder\tFamily')
		elif mostSpecificRank == 'GENUS':
			fout.write('Domain\tPhylum\tClass\tOrder\tFamily\tGenus')
		elif mostSpecificRank == 'SPECIES':
			fout.write('Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies')

		for sampleName in sampleNames:
			fout.write('\t' + sampleName)
		fout.write('\n')
			
		for key in profileDict.keys():
			row = profileDict[key]
			hierarchy = row.hierarchy.split(';')
			for h in xrange(len(hierarchy)-1, -1, -1):
				fout.write(hierarchy[h] + '\t')
			for i in xrange(0, mostSpecificRankIndex-len(hierarchy)+1):
				fout.write('Unclassified' + '\t')
			
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
