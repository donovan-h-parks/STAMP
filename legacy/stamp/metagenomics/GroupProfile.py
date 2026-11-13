#=======================================================================
# Author: Donovan Parks
#
# Stores profile information for two groups.
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

import numpy as np

class GroupProfileEntry:
	def __init__(self):
		self.hierarchy = []
		self.featureCounts = []
		self.parentCounts = []
	
class GroupProfile:
	def __init__(self):
		self.groupName1 = ''
		self.groupName2 = ''
		
		self.hierarchyHeadings = []
		self.samplesInGroup1 = []
		self.samplesInGroup2 = []

		self.parentHeading = None
		self.profileHeading = None
		
		self.profileDict = {}
		
		self.numParentCategories = 0
				
	def getFeatures(self):
		return self.profileDict.keys()
		
	def getNumFeatures(self):
		return len(self.profileDict)
	
	def getNumParentCategories(self):
		return self.numParentCategories
		
	def getData(self, feature):
		return self.profileDict[feature]
	
	def getHierarchy(self, feature):
		return self.profileDict[feature].hierarchy
		
	def getFeatureCounts(self, feature):
		profile = self.profileDict[feature]

		data1 = []
		for i in xrange(0, len(self.samplesInGroup1)):
			data1.append(profile.featureCounts[i])
			
		data2 = []
		for i in xrange(len(self.samplesInGroup1), len(profile.featureCounts)):
			data2.append(profile.featureCounts[i])
		
		return data1, data2
		
	def getParentFeatureCounts(self, feature):
		profile = self.profileDict[feature]

		data1 = []
		for i in xrange(0, len(self.samplesInGroup1)):
			data1.append(profile.parentCounts[i])
			
		data2 = []
		for i in xrange(len(self.samplesInGroup1), len(profile.parentCounts)):
			data2.append(profile.parentCounts[i])
		
		return data1, data2
		
	def getFeatureCountsAll(self):
		return self.getActiveFeatureCounts(self.profileDict.keys())

	def getActiveFeatureCounts(self, activeFeatures):
		seqGroup1 = [] 
		seqGroup2 = []
		for feature in activeFeatures:
			data1, data2 = self.getFeatureCounts(feature)
			seqGroup1.append(data1)
			seqGroup2.append(data2)
		
		return seqGroup1, seqGroup2
		
	def getParentFeatureCountsAll(self):
		parentSeqGroup1 = [] 
		parentSeqGroup2 = []
		for feature in self.profileDict.keys():
			data1, data2 = self.getParentFeatureCounts(feature)
			parentSeqGroup1.append(data1)
			parentSeqGroup2.append(data2)
		
		return parentSeqGroup1, parentSeqGroup2
		
	def getFeatureProportions(self, feature):
		profile = self.profileDict[feature]

		data1 = []
		for i in xrange(0, len(self.samplesInGroup1)):
			fc = float(profile.featureCounts[i])
			pc = profile.parentCounts[i]
			if pc > 0:
				data1.append(fc*100 / pc)
			else:
				data1.append(0.0)
			
		data2 = []
		for i in xrange(len(self.samplesInGroup1), len(profile.featureCounts)):
			fc = float(profile.featureCounts[i])
			pc = profile.parentCounts[i]
			if pc > 0:
				data2.append(fc*100 / pc)
			else:
				data2.append(0.0)

		return data1, data2
		
	def getFeatureProportionsAll(self):
		return self.getActiveFeatureProportions(self.profileDict.keys())
	
	def getActiveFeatureProportions(self, activeFeatures):
		groupData1 = [] 
		groupData2 = []
		for feature in activeFeatures:
			data1, data2 = self.getFeatureProportions(feature)
			groupData1.append(data1)
			groupData2.append(data2)
		
		return groupData1, groupData2
		
	def getFeatureMatrix(self):
		samples = self.samplesInGroup1 + self.samplesInGroup2
		
		featureMatrix = []
		for i in xrange(0, len(samples)):
			featureMatrix.append([])

		for feature in self.profileDict:
			data = self.profileDict[feature]
			
			for i in xrange(0, len(samples)):
				if data.parentCounts[i] > 0:
					featureMatrix[i].append(float(data.featureCounts[i]) / data.parentCounts[i])
				else:
					featureMatrix[i].append(0.0)

		return np.array(featureMatrix)
