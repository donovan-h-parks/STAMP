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
from os.path import splitext

from PyQt4 import QtGui, QtCore
from biom.parse import parse_biom_table

from createProfileBiomUI import Ui_CreateProfileBiomDlg
from stamp.metagenomics.StringHelper import isNumber

class CreateProfileBiomDlg(QtGui.QDialog):
	def __init__(self, preferences, parent=None):
		QtGui.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_CreateProfileBiomDlg()
		self.ui.setupUi(self)
		
		self.preferences = preferences

		self.centerWindow()
		
		QtCore.QObject.connect(self.ui.btnBiomFile, QtCore.SIGNAL("clicked()"), self.loadBiomFile)
		QtCore.QObject.connect(self.ui.btnCreateProfile, QtCore.SIGNAL("clicked()"), self.createProfile)
		QtCore.QObject.connect(self.ui.btnCancel, QtCore.SIGNAL("clicked()"), self.accept)
		
		self.biomFile = None

	def loadBiomFile(self):
		selectedFile = QtGui.QFileDialog.getOpenFileName(self, 'Load BIOM file', self.preferences['Last directory'], 'BIOM file (*.biom);;Compressed BIOM file (*.gz);;All files (*.*)')
		if selectedFile != '':
			self.preferences['Last directory'] = selectedFile[0:selectedFile.lastIndexOf('/')]
			self.ui.txtBiomFile.setText(selectedFile)
			self.biomFile = selectedFile
	
	def createProfile(self):
		# determine group for each sequence ID
		if self.biomFile == None:
			QtGui.QMessageBox.information(self, 'Missing data', 'A BIOM file must be specified.', QtGui.QMessageBox.Ok)
			return
			
		outputFile = QtGui.QFileDialog.getSaveFileName(self, 'Save STAMP profile...', self.preferences['Last directory'],'STAMP profile file(*.spf);;All files(*.*)')
		if outputFile == '':
			return
				
		self.convertBiomFileToStampProfile(str(self.biomFile), str(outputFile), str(self.ui.cboMetadataField.currentText()))

		self.accept()

	def centerWindow(self):
		screen = QtGui.QDesktopWidget().screenGeometry()
		size = self.geometry()
		self.move((screen.width()-size.width())/2, (screen.height()-size.height())/2)
		
	def convertBiomFileToStampProfile(self, file_name, output_name, metadata_name):
		""" Function taken from PICRUSt by Morgan Langill.
		      https://github.com/mlangill/get_mgrast_data/blob/master/biom_to_stamp.py
		"""

		#allow file to be optionally gzipped (must use extension '.gz')
		ext=splitext(file_name)[1]
		if (ext == '.gz'):
			table = parse_biom_table(gzip.open(file_name,'rb'))
		else:
			table = parse_biom_table(open(file_name,'U'))
		
		metadata_name = metadata_name.split('(')[0].rstrip()
		if metadata_name is None or metadata_name == '<observation ids>':
			max_len_metadata = 0
		elif table.observation_metadata and metadata_name in table.observation_metadata[0]:
			#figure out the longest list within the given metadata
			max_len_metadata = max(len(p[metadata_name]) for p in table.observation_metadata)
		else:
			QtGui.QMessageBox.information(self, 'Unrecognized metadata file', "'" + metadata_name + "' was not found in the BIOM table.", QtGui.QMessageBox.Ok)
			return
		
		#make the header line
		header=[]
		#make simple labels for each level in the metadata (e.g. 'Level_1', 'Level_2', etc.) "+1" for the observation id as well.
		for i in range(max_len_metadata):
			header.append('Level_'+ str(i+1))
		header.append('Observation Ids')
		
		#add the sample ids to the header line
		header.extend(table.sample_ids)
		
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
