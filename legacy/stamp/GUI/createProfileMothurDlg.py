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
from createProfileMothurUI import Ui_CreateProfileMothurDlg

class ProfileRow():
	def __init__(self):
		countData = []
		hierarchy = []

class CreateProfileMothurDlg(QtGui.QDialog):
	def __init__(self, preferences, parent=None):
		QtGui.QWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_CreateProfileMothurDlg()
		self.ui.setupUi(self)
		
		self.preferences = preferences

		self.centerWindow()
		
		QtCore.QObject.connect(self.ui.btnTaxonomyFile, QtCore.SIGNAL("clicked()"), self.loadTaxonomyFile)
		QtCore.QObject.connect(self.ui.btnGroupsFile, QtCore.SIGNAL("clicked()"), self.loadGroupsFile)
		QtCore.QObject.connect(self.ui.btnNamesFile, QtCore.SIGNAL("clicked()"), self.loadNamesFile)
		QtCore.QObject.connect(self.ui.btnCreateProfile, QtCore.SIGNAL("clicked()"), self.createProfile)
		QtCore.QObject.connect(self.ui.btnCancel, QtCore.SIGNAL("clicked()"), self.accept)
		
		self.taxonomyFile = None
		self.groupsFile = None
		self.namesFile = None

	def loadTaxonomyFile(self):
		selectedFile = QtGui.QFileDialog.getOpenFileName(self, 'Load taxonomy file', self.preferences['Last directory'], 'Taxonomy file (*.taxonomy);;All files (*.*)')
		if selectedFile != '':
			self.preferences['Last directory'] = selectedFile[0:selectedFile.lastIndexOf('/')]
			self.ui.txtTaxonomyFile.setText(selectedFile)
			self.taxonomyFile = selectedFile
		
	def loadGroupsFile(self):
		selectedFile = QtGui.QFileDialog.getOpenFileName(self, 'Load groups file', self.preferences['Last directory'], 'Groups file (*.groups);;All files (*.*)')
		if selectedFile != '':
			self.preferences['Last directory'] = selectedFile[0:selectedFile.lastIndexOf('/')]
			self.ui.txtGroupsFile.setText(selectedFile)
			self.groupsFile = selectedFile
		
	def loadNamesFile(self):
		selectedFile = QtGui.QFileDialog.getOpenFileName(self, 'Load names file', self.preferences['Last directory'], 'Names file (*.names);;All files (*.*)')
		if selectedFile != '':
			self.preferences['Last directory'] = selectedFile[0:selectedFile.lastIndexOf('/')]
			self.ui.txtNamesFile.setText(selectedFile)
			self.namesFile = selectedFile
	
	def createProfile(self):
		# determine group for each sequence ID
		if self.groupsFile != None:
			fin = open(self.groupsFile, 'U')
			data = map(string.strip, fin.readlines())
			fin.close()
		else:
			QtGui.QMessageBox.information(self, 'Missing data', 'A Group file must be specified.', QtGui.QMessageBox.Ok)
			return
			
		outputFile = QtGui.QFileDialog.getSaveFileName(self, 'Save STAMP profile...', self.preferences['Last directory'],'STAMP profile file(*.spf);;All files(*.*)')
		if outputFile == '':
			return
			
		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))
			
		seqIdToSampleId = {}
		sampleIds = set([])
		for line in data:
			lineSplit = line.split('\t')
			seqId = lineSplit[0].strip()
			sampleId = lineSplit[1].strip()
			
			seqIdToSampleId[seqId] = sampleId
			sampleIds.add(sampleId)
			
		sampleIds = sorted(list(sampleIds))
			
		# determine representative sequence for OTUs
		if self.namesFile != None:
			fin = open(self.namesFile, 'U')
			data = map(string.strip, fin.readlines())
			fin.close()

			seqIdToSeqIds = {}
			for line in data:
				lineSplit = line.split('\t')
				seqId = lineSplit[0].strip()
				
				seqIds = lineSplit[1].split(',')
				map(string.strip, seqIds)
				seqIdToSeqIds[seqId] = seqIds
				
		# read taxonomy file and create profile for each sample
		deepestRank = 0

		fin = open(self.taxonomyFile, 'U')
		data = map(string.strip, fin.readlines())
		fin.close()

		sampleProfiles = {}
		for line in data:
			lineSplit = line.split('\t')
			seqId = lineSplit[0]
			
			taxonomy = lineSplit[1].split(';')
			classificationStr = ''
			depth = 0
			for t in taxonomy:
				if t.strip() != '':
					if t[-1] == ')':	# remove trailing confidence scores
						t = t[0:t.rfind('(')]
					classificationStr = classificationStr + t + '$'
					depth += 1
					
			if depth > deepestRank:
				deepestRank = depth
				
			if classificationStr not in sampleProfiles:
				sampleProfiles[classificationStr] = {}
				
			# determine number of times each sample is associated with this classification
			seqIds = [seqId]
			if self.namesFile != None:
				seqIds = seqIdToSeqIds[seqId]
				
			samplesInOTU = []
			for seqId in seqIds:
				samplesInOTU.append(seqIdToSampleId[seqId])
				
			for sampleId in samplesInOTU:
				sampleProfiles[classificationStr][sampleId] = sampleProfiles[classificationStr].get(sampleId, 0) + 1
				
		# write out results
		fout = open(outputFile, 'w')

		taxonomicRanks = ['Level 1', 'Level 2', 'Level 3', 'Level 4', 'Level 5', 'Level 6', 'Level 7', 'Level 8', 'Level 9', 'Level 10', 'Level 11', 'Level 12']
		fout.write(taxonomicRanks[0])
		for r in xrange(1, deepestRank):
			fout.write('\t' + taxonomicRanks[r])

		for sampleId in sampleIds:
			fout.write('\t' + sampleId)
		fout.write('\n')

		for classificationStr in sampleProfiles:
			classification = classificationStr.split('$')
			classification = classification[0:len(classification)-1]
			
			fout.write(classification[0])
			for c in xrange(1, len(classification)):
				fout.write('\t' + classification[c])
				
			for c in xrange(len(classification), deepestRank):
				fout.write('\t' + 'unclassified')
				
			counts = sampleProfiles[classificationStr]
			for sampleId in sampleIds:
				if sampleId in counts:
					fout.write('\t' + str(counts[sampleId]))
				else:
					fout.write('\t' + '0')
			fout.write('\n')
			
		fout.close()

		QtGui.QApplication.instance().restoreOverrideCursor()
		
		self.accept()

	def centerWindow(self):
		screen = QtGui.QDesktopWidget().screenGeometry()
		size =	self.geometry()
		self.move((screen.width()-size.width())/2, (screen.height()-size.height())/2)
