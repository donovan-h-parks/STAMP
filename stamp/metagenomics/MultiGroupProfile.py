#=======================================================================
# Author: Donovan Parks
#
# Stores profile information for multiple groups.
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

class MultiGroupProfileEntry:
	def __init__(self):
		self.hierarchy = []
		self.featureCounts = []
		self.parentCounts = []
	
class MultiGroupProfile:
	def __init__(self):
		self.hierarchyHeadings = []
		self.groupNames = []
		self.samplesInGroups = []
		self.smallestGroup = 0
		
		self.activeGroups = {}
		self.activeGroupNames = []
		self.activeSamplesInGroups = []
		self.activeColumns = []

		self.parentHeading = None
		self.profileHeading = None
		
		self.profileDict = {}
		
		self.numParentCategories = 0
		
	def getNumberActiveSamples(self):
		activeSamples = 0
		for group in self.activeSamplesInGroups:
			activeSamples += len(group)
			
		return activeSamples
		
	def getSmallestActiveGroup(self):
		smallestGroup = 1e9
		for group in self.activeSamplesInGroups:
			if len(group) < smallestGroup:
				smallestGroup = len(group)
				
		return smallestGroup
		
	def setActiveGroups(self, activeGroups):
		self.activeGroups = activeGroups
		
		self.activeGroupNames = []
		self.activeSamplesInGroups = []
		self.activeColumns = []
		groupIndex = 0
		colIndex = 0
		for groupName in sorted(self.activeGroups.keys()):
			active = self.activeGroups[groupName]
			if active:
				self.activeGroupNames.append(groupName)
				self.activeSamplesInGroups.append(self.samplesInGroups[groupIndex])
				
				for _ in xrange(0, len(self.samplesInGroups[groupIndex])):
					self.activeColumns.append(colIndex)
					colIndex += 1
					
			else:
				colIndex += len(self.samplesInGroups[groupIndex])

			if self.groupNames[groupIndex] != groupName:
				print 'Error: order of group names is incorrect.'
				
			groupIndex += 1

	def getFeatures(self):
		return self.profileDict.keys()
		
	def getNumFeatures(self):
		return len(self.profileDict)
	
	def getNumParentCategories(self):
		return self.numParentCategories
		
	def getData(self, feature):
		return self.profileDict[feature]

	def getFeatureCount(self, feature):
		return self.profileDict[feature].featureCounts
	
	def getParentCount(self, feature):
		return self.profileDict[feature].parentCounts

	def getHierarchy(self, feature):
		return self.profileDict[feature].hierarchy
	
	def getSampleGroup(self, sampleId):
		for i in xrange(0, len(self.samplesInGroups)):
			if sampleId in self.samplesInGroups[i]:
				return self.groupNames[i]
			
		return None
		
	def getFeatureCounts(self, feature):
		profile = self.profileDict[feature]
		
		sampleData = []
		index = 0
		for i in xrange(0, len(self.samplesInGroups)):
			data = []
			for _ in xrange(0, len(self.samplesInGroups[i])):
				data.append(profile.featureCounts[index])
				index += 1
			sampleData.append(data)

		return sampleData
		
	def getActiveFeatureCounts(self, feature):
		profile = self.profileDict[feature]
		
		sampleData = []
		index = 0
		for i in xrange(0, len(self.activeSamplesInGroups)):
			data = []
			for _ in xrange(0, len(self.activeSamplesInGroups[i])):
				data.append(profile.featureCounts[self.activeColumns[index]])
				index += 1
			sampleData.append(data)
			
		return sampleData
	
	def getActiveFeatureCountsAll(self):
		allData = [] 
		for feature in self.profileDict.keys():
			data = self.getActiveFeatureCounts(feature)
			allData.append(data)
		
		return allData
	
	def getActiveFeatureFromActiveSamplesCounts(self, activeFeatures):
		allData = [] 
		for feature in activeFeatures:
			data = self.getActiveFeatureCounts(feature)
			allData.append(data)
		
		return allData
		
	def getParentCounts(self, feature):
		profile = self.profileDict[feature]
		
		sampleData = []
		index = 0
		for i in xrange(0, len(self.samplesInGroups)):
			data = []
			for _ in xrange(0, len(self.samplesInGroups[i])):
				data.append(profile.parentCounts[index])
				index += 1
			sampleData.append(data)

		return sampleData
		
	def getActiveParentCounts(self, feature):
		profile = self.profileDict[feature]
		
		sampleData = []
		index = 0
		for i in xrange(0, len(self.activeSamplesInGroups)):
			data = []
			for _ in xrange(0, len(self.activeSamplesInGroups[i])):
				data.append(profile.parentCounts[self.activeColumns[index]])
				index += 1
			sampleData.append(data)

		return sampleData
		
	def getFeatureProportions(self, feature):
		profile = self.profileDict[feature]
		
		sampleData = []
		index = 0
		for i in xrange(0, len(self.samplesInGroups)):
			data = []
			for _ in xrange(0, len(self.samplesInGroups[i])):
				data.append(float(profile.featureCounts[index])*100 / profile.parentCounts[index])
				index += 1
			sampleData.append(data)

		return sampleData
		
	def getActiveFeatureProportions(self, feature):
		profile = self.profileDict[feature]

		sampleData = []
		index = 0
		for i in xrange(0, len(self.activeSamplesInGroups)):
			data = []
			for _ in xrange(0, len(self.activeSamplesInGroups[i])):
				fc = float(profile.featureCounts[self.activeColumns[index]])
				pc = profile.parentCounts[self.activeColumns[index]]
				if pc > 0:
					data.append(fc*100 / pc)
				else:
					data.append(0.0)
					
				index += 1
			sampleData.append(data)

		return sampleData
	
	def getActiveFeatureProportionsAll(self):
		allData = [] 
		for feature in self.profileDict.keys():
			data = self.getActiveFeatureProportions(feature)
			allData.append(data)
		
		return allData
	
	def getActiveFeatureFromActiveSamplesProportions(self, activeFeatures):
		allData = [] 
		for feature in activeFeatures:
			data = self.getActiveFeatureProportions(feature)
			allData.append(data)
		
		return allData

	def getFeatureMatrix(self):
		numSamples = len(self.profileDict[self.profileDict.keys()[0]].featureCounts)
		
		featureMatrix = []
		for i in xrange(0, numSamples):
				featureMatrix.append([])

		for feature in self.profileDict:
			data = self.profileDict[feature]
			
			for i in xrange(0, numSamples):
				featureMatrix[i].append(float(data.featureCounts[i]) / data.parentCounts[i])

		return np.array(featureMatrix)
		
	def getActiveFeatureMatrix(self):
		featureMatrix = []
		for i in xrange(0, len(self.activeColumns)):
				featureMatrix.append([])

		for feature in self.profileDict:
			data = self.profileDict[feature]
			
			for i in xrange(0, len(self.activeColumns)):
				fc = float(data.featureCounts[self.activeColumns[i]])
				pc = data.parentCounts[self.activeColumns[i]]
				if pc > 0:
					featureMatrix[i].append( fc / pc )
				else:
					featureMatrix[i].append( 0.0 )

		return np.array(featureMatrix)
