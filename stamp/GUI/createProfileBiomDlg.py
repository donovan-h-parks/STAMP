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
import gzip
import os
from os.path import splitext

import biom
from PyQt5 import QtGui, QtCore, QtWidgets
from biom.parse import parse_biom_table

from stamp.GUI.createProfileBiomUI import Ui_CreateProfileBiomDlg
from stamp.metagenomics.StringHelper import isNumber

class CreateProfileBiomDlg(QtWidgets.QDialog):
	def __init__(self, preferences, parent=None):
		QtWidgets.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_CreateProfileBiomDlg()
		self.ui.setupUi(self)
		
		self.preferences = preferences

		self.centerWindow()

		self.ui.btnBiomFile.clicked.connect(self.loadBiomFile)
		self.ui.btnCreateProfile.clicked.connect(self.createProfile)
		self.ui.btnCancel.clicked.connect(self.accept)

		
		self.biomFile = None

	import os  # Ensure this is at the top of your file

	def loadBiomFile(self):
		# 1. Unpack the tuple (path, filter)
		# Using .get() prevents a KeyError if 'Last directory' isn't set yet
		fileName, _ = QtWidgets.QFileDialog.getOpenFileName(
			self,
			'Load BIOM file',
			self.preferences.get('Last directory', ''),
			'BIOM file (*.biom);;Compressed BIOM file (*.gz);;All files (*.*)'
		)

		# 2. In PyQt5, fileName is an empty string if the user cancels
		if fileName:
			# 3. Use os.path.dirname instead of lastIndexOf
			self.preferences['Last directory'] = os.path.dirname(fileName)
			self.ui.txtBiomFile.setText(fileName)
			self.biomFile = fileName
	
	def createProfile(self):
		# determine group for each sequence ID
		if self.biomFile == None:
			QtWidgets.QMessageBox.information(self, 'Missing data', 'A BIOM file must be specified.', QtWidgets.QMessageBox.Ok)
			return

		# Use unpacking (path, filter)
		outputFile, _ = QtWidgets.QFileDialog.getSaveFileName(self, 'Save STAMP profile...',
															  self.preferences.get('Last directory', ''),
															  'STAMP profile file(*.spf);;All files(*.*)')
		if outputFile == '':
			return

		if outputFile:
			# Now outputFile is just the string path, no parentheses!
			self.convertBiomFileToStampProfile(self.biomFile, outputFile, str(self.ui.cboMetadataField.currentText()))

		self.accept()

	def centerWindow(self):
		screen = QtWidgets.QDesktopWidget().screenGeometry()
		size = self.geometry()
		self.move((screen.width()-size.width())//2, (screen.height()-size.height())//2)

	import gzip
	import biom
	from os.path import splitext

	def convertBiomFileToStampProfile(self, file_name, output_name, metadata_name):
		""" Modernized for Python 3 and biom-format 2.1+ """

		# 1. Use biom.load_table – it automatically handles compression and
		# detects if the file is JSON or HDF5 (the two BIOM formats).
		try:
			# load_table is the modern replacement for parse_biom_table
			table = biom.load_table(file_name)
		except Exception as e:
			# Fallback logic for specifically gzipped legacy JSON files if load_table fails
			ext = splitext(file_name)[1]
			if ext == '.gz':
				with gzip.open(file_name, 'rt') as f:  # 'rt' for Read Text mode
					table = biom.parse_biom_table(f)
			else:
				with open(file_name, 'r', encoding='utf-8') as f:
					table = biom.parse_biom_table(f)
		
		metadata_name = metadata_name.split('(')[0].rstrip()
		if metadata_name is None or metadata_name == '<observation ids>':
			max_len_metadata = 0
		elif table.metadata(axis='observation') and metadata_name in table.metadata(axis='observation')[0]:
			#figure out the longest list within the given metadata
			max_len_metadata = max(len(p[metadata_name]) for p in table.metadata(axis='observation'))
		else:
			QtWidgets.QMessageBox.information(self, 'Unrecognized metadata file', "'" + metadata_name + "' was not found in the BIOM table.", QtWidgets.QMessageBox.Ok)
			return
		
		#make the header line
		header=[]
		#make simple labels for each level in the metadata (e.g. 'Level_1', 'Level_2', etc.) "+1" for the observation id as well.
		for i in range(max_len_metadata):
			header.append('Level_'+ str(i+1))
		header.append('Observation Ids')
		
		#add the sample ids to the header line
		header.extend(table.ids())
		
		fout = open(output_name, 'w')
		fout.write("\t".join(header) + '\n')
		
		#now process each observation (row in the table)
		for obs_vals, obs_id, obs_metadata in table.iter(axis='observation'):
			row=[]
			if max_len_metadata > 0:
				row = obs_metadata[metadata_name]
		
			# add blanks if the metadata doesn't fill each level
			if len(row) < max_len_metadata:
				for i in range(max_len_metadata - len(row)):
					row.append('unclassified')
			
			#Add the observation id as the last "Level"
			if isNumber(obs_id):
				row.append('ID' + obs_id)
			else:
				row.append(obs_id)
			
			#Add count data to the row
			row.extend(map(str,obs_vals))
			fout.write("\t".join(row) + '\n')
			
		fout.close()
