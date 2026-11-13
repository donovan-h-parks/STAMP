#=======================================================================
# Author: Donovan Parks
#
# Stores hierarchical profile information.
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

from stamp.metagenomics.StringHelper import isStrictNumber

class Metadata:
	def __init__(self):
		self.metadataDict = {}
		self.activeField = ''
		self.activeSamples = []
		
	def isActiveSample(self, sampleName):
		return (sampleName in self.activeSamples)
		
	def numActiveSamples(self):
		return len(self.activeSamples)
		
	def getValue(self, sampleName, field):
		return self.metadataDict[sampleName][field]
		
	def getUniqueValues(self, field):
		values = set([])
		for sample in self.metadataDict:
			values.add(self.metadataDict[sample][field])
			
		return list(values)
		
	def isNumericalData(self, field):
		for sample in self.metadataDict:
			if not isStrictNumber(self.metadataDict[sample][field]):
				return False
				
		return True
		
	def getNumSamples(self):
		return len(self.sampleNames)
		
	def getSampleNames(self):
		return self.metadataDict.keys()
		
	def getFeatures(self):
		return self.metadataDict[self.metadataDict.keys()[0]].keys()
		
	def getTableData(self):
		table = []
		headers = ['Sample Id']
		
		sampleNum = 0
		for sample in sorted(self.metadataDict.keys()):
			row = []
			row.append(sample)
			for field in sorted(self.metadataDict[sample].keys()):
				row.append(self.metadataDict[sample][field])
				
				if sampleNum == 0:
					headers.append(field)
					
			table.append(row)
			sampleNum += 1

		return table, headers
		
	def setActiveField(self, field, profileTree):
		self.activeField = field
		
		profileTree.groupDict = {}
		profileTree.groupActive = {}
		for (sampleName, sampleDict) in self.metadataDict.items():
			groupName = sampleDict[field]
			if groupName in profileTree.groupDict:
				profileTree.groupDict[groupName].append(sampleName)
			else:
				profileTree.groupDict[groupName] = [sampleName]
				profileTree.groupActive[groupName] = True

