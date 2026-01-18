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
import os
import string

from PyQt5 import QtGui, QtCore, QtWidgets
from stamp.GUI.createProfileRITA_UI import Ui_CreateProfileRITADlg

from stamp.GUI.customizeHeadingsDlg import CustomizeHeadingsDlg

class ProfileRow():
	def __init__(self):
		countData = []
		hierarchy = []

class CreateProfileRITADlg(QtWidgets.QDialog):
	def __init__(self, preferences, parent=None):
		QtWidgets.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_CreateProfileRITADlg()
		self.ui.setupUi(self)
		
		self.preferences = preferences

		self.centerWindow()

		self.ui.btnLoadProfiles.clicked.connect(self.loadProfiles)
		self.ui.btnCreateProfile.clicked.connect(self.createProfile)
		self.ui.btnCancel.clicked.connect(self.accept)
		
		self.selectedFiles = []


	def loadProfiles(self):
		# 1. Unpack the tuple: files is a list, _ is the filter string
		files, _ = QtWidgets.QFileDialog.getOpenFileNames(
			self,
			'Load profiles',
			self.preferences.get('Last directory', ''),
			'RITA profiles (*.txt);;All files (*.*)'
		)

		# 2. Check if the list is not empty
		if files:
			# 3. Update 'Last directory' using the first file in the list
			self.preferences['Last directory'] = os.path.dirname(files[0])

			for file_path in files:
				# In Python 3, file_path is already a string
				self.selectedFiles.append(file_path)
				self.ui.lstSelectedProfiles.addItem(file_path)

			self.ui.btnCreateProfile.setEnabled(True)

	import os

	def createProfile(self):
		# 1. Unpack PyQt5 tuple
		stampFilename, _ = QtWidgets.QFileDialog.getSaveFileName(
			self, 'Save STAMP profile...',
			self.preferences.get('Last directory', ''),
			'STAMP profile file(*.spf);;All files(*.*)'
		)
		if not stampFilename:
			return

		# Update directory preference
		self.preferences['Last directory'] = os.path.dirname(stampFilename)

		# get checked groups
		checkedGroups = []
		# (Your checkbox logic remains the same)
		if self.ui.chkNB_DBLASTN.isChecked(): checkedGroups.append('NB and D-BLASTN')
		if self.ui.chkDBLASTN.isChecked(): checkedGroups.append('D-BLASTN ratio')
		if self.ui.chkNB_BLASTN.isChecked(): checkedGroups.append('NB and BLASTN')
		if self.ui.chkBLASTN.isChecked(): checkedGroups.append('BLASTN ratio')
		if self.ui.chkNB_BLASTX.isChecked(): checkedGroups.append('NB and BLASTX')
		if self.ui.chkBLASTX.isChecked(): checkedGroups.append('BLASTX ratio')
		if self.ui.chkNB.isChecked(): checkedGroups.append('NB ratio')

		profileDict = {}
		profileIndex = 0
		sampleNames = []
		mostSpecificRankIndex = 0
		ranks = ['DOMAIN', 'PHYLUM', 'CLASS', 'ORDER', 'FAMILY', 'GENUS', 'SPECIES']

		for file in self.selectedFiles:
			# 2. Python 3: Remove 'U', use utf-8 encoding
			with open(file, 'r', encoding='utf-8', errors='ignore') as fin:
				# Python 3: map returns an iterator; use a list comprehension instead
				data = [line.strip() for line in fin.readlines()]

			# 3. Use os.path to get sample name (replaces manual rfind slicing)
			baseName = os.path.basename(file)
			sampleName = os.path.splitext(baseName)[0]
			if '_' in sampleName:
				sampleName = sampleName.split('_', 1)[1]
			sampleNames.append(sampleName)

			for i in range(1, len(data)):
				line = data[i]
				if not line: continue

				lineSplit = line.split('\t')
				if len(lineSplit) < 4: continue

				groupName = lineSplit[2]
				rank = lineSplit[3]

				if groupName not in checkedGroups:
					continue

				if rank in ranks:
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

		# 4. Write out STAMP profile with explicit encoding
		try:
			fout = open(stampFilename, 'w', encoding='utf-8')
		except IOError:
			QtWidgets.QMessageBox.information(self, 'Failed to save STAMP profile', 'Write permission denied.')
			return

		# Header Logic
		rank_headers = ['Domain', 'Phylum', 'Class', 'Order', 'Family', 'Genus', 'Species']
		fout.write('\t'.join(rank_headers[:mostSpecificRankIndex + 1]))

		for sampleName in sampleNames:
			fout.write('\t' + sampleName)
		fout.write('\n')

		# 5. In Python 3, keys() is a view; sorted() helps with comparison
		for key in sorted(profileDict.keys()):
			row = profileDict[key]
			hierarchy = row.hierarchy.split(';')

			# Process hierarchy
			for h in range(len(hierarchy) - 1, -1, -1):
				fout.write(hierarchy[h] + '\t')
			for i in range(0, mostSpecificRankIndex - len(hierarchy) + 1):
				fout.write('Unclassified' + '\t')

			fout.write('\t'.join(map(str, row.countData)))
			fout.write('\n')

		fout.close()
		self.accept()

	def centerWindow(self):
		screen = QtWidgets.QDesktopWidget().screenGeometry()
		size =	self.geometry()
		self.move((screen.width()-size.width())//2, (screen.height()-size.height())//2)
