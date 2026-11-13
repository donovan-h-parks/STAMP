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

import sys

from stamp.metagenomics.SampleProfile import SampleProfile, SampleProfileEntry
from stamp.metagenomics.GroupProfile import GroupProfile, GroupProfileEntry
from stamp.metagenomics.MultiGroupProfile import MultiGroupProfile

class Node:
	def __init__(self, name, parent = None):
		self.name = name
		self.parent = parent
		self.children = []
		self.countData = {}

	def depth(self):
		depth = 0
		curNode = self
		while curNode.parent != None:
			depth += 1
			curNode = curNode.parent
			
		return depth
	
	def isLeaf(self):
		return (len(self.children) == 0)
	
	def isRoot(self):
		return (self.parent == None)
	
	def childWithName(self, name):
		for child in self.children:
			if child.name == name:
				return child
			
		return None
	
class ProfileTree:
	def __init__(self):
		self.hierarchyHeadings = []
		self.sampleNames = []
		self.groupDict = {}
		self.groupActive = {}
		self.numSeqInSample = []
		
		self.root = Node('Entire sample')
		
	def numSamples(self):
		return len(self.sampleNames)
	
	def numSequencesInSample(self, name):
		if name == '':
			return 0
			
		index = self.sampleNames.index(name)
		return self.numSeqInSample[index]
		
	def numSequencesInGroup(self, group, metadata):
		if group == '':
			return 0
			
		seqIds = set(self.groupDict[group]).intersection(metadata.activeSamples)
		
		totalSeqs = 0
		for seqId in seqIds:
			index = self.sampleNames.index(seqId)
			totalSeqs += self.numSeqInSample[index]

		return totalSeqs
		
	def numSequences(self, metadata):
		totalSeqs = 0
		for group in self.groupDict:
			totalSeqs += self.numSequencesInGroup(group, metadata)

		return totalSeqs
	
	def numHierarchicalLevels(self):
		return len(self.hierarchyHeadings)
	
	def getHierarchicalLevelDepth(self, name):
		if name == 'Entire sample':
			return 0
		else:
			return self.hierarchyHeadings.index(name) + 1
		
	def getNodeWithName(self, node, name):
		if node.name == name:
			return self		
		elif node.isLeaf():
			return None
		
		for child in node.children:
			node = self.getNodeWidthName(child, name)
			if node != None:
				return node
			
		return None
	
	def getLeafNodes(self):
		curNode = self.root
		leafNodes = []
		for child in curNode.children:
			self.getLeafNodesRecursive(child, leafNodes)
				
		return leafNodes
	
	def getLeafNodesRecursive(self, node, leafNodes):
		if node.isLeaf():
			leafNodes.append(node)
			return
		
		for child in node.children:
			self.getLeafNodesRecursive(child, leafNodes)
	
	def createSampleProfile(self, sampleName1, sampleName2, parentHeading, profileHeading, unclassifiedTreatment):
		profile = SampleProfile() 
		
		# get depth of hierarchical levels of interest 
		self.parentHeading = parentHeading
		self.profileHeading = profileHeading
		if parentHeading == 'Entire sample':
			parentDepth = 0
		else:
			parentDepth = self.hierarchyHeadings.index(parentHeading) + 1
			
		profileDepth = self.hierarchyHeadings.index(profileHeading) + 1
		
		profile.hierarchyHeadings = self.hierarchyHeadings[0:profileDepth]
		
		# get index for samples of interest
		sampleIndex1 = self.sampleNames.index(sampleName1)
		sampleIndex2 = self.sampleNames.index(sampleName2)
		profile.sampleNames = [sampleName1, sampleName2]
		
		# get all leaf nodes
		leafNodes = self.getLeafNodes()
		
		# traverse up tree from each leaf node
		parentSeqDict = {} 
		for leaf in leafNodes:
			curDepth = len(self.hierarchyHeadings) 
			
			curNode = leaf
			hierarchy = []
			bRemoveUnclassified = False
			while curNode != None:
				if not curNode.isRoot() and curDepth <= profileDepth:
					hierarchy.append(curNode.name)
				
				# add profile level information
				if curDepth == profileDepth:
					if 'unclassified' in curNode.name.lower():
						if unclassifiedTreatment == 'Remove unclassified reads':
							bRemoveUnclassified = True
							break
						elif unclassifiedTreatment == 'Use only for calculating frequency profiles':
							bRemoveUnclassified = True
					
					if not bRemoveUnclassified:
						name = curNode.name
						
						# remove ' - #' if feature is being calculated relative to the entire sample
						bTruncatedName = False
						if curNode.isLeaf() and parentDepth == 0:
							if name.rfind(' - #') != -1:
								name = name[0:name.rfind(' - #')]
								bTruncatedName = True
						
						profileEntry = profile.profileDict.get(name)
						if bTruncatedName and profileEntry != None:
							bRemoveUnclassified = True
							break
							
						if profileEntry == None:
							profileEntry = SampleProfileEntry()
							profileEntry.featureCounts = [0, 0]
							profile.profileDict[name] = profileEntry

						profileEntry.featureCounts[0] += leaf.countData[sampleName1]
						profileEntry.featureCounts[1] += leaf.countData[sampleName2]
			
				# add parent level information
				if curDepth == parentDepth:
					sequences = parentSeqDict.get(curNode.name)
					if sequences == None:
						sequences = [0, 0]
						parentSeqDict[curNode.name] = sequences

					sequences[0] += leaf.countData[sampleName1]
					sequences[1] += leaf.countData[sampleName2]
						
					if not bRemoveUnclassified:
						profileEntry.parentCounts = sequences
						
				curDepth -= 1
				curNode = curNode.parent
			
			if not bRemoveUnclassified:
				hierarchy.reverse()
				profileEntry.hierarchy = hierarchy

		profile.numParentCategories = len(parentSeqDict)
		
		return profile
		
	def createGroupProfile(self, groupName1, groupName2, parentHeading, profileHeading, metadata, unclassifiedTreatment):
		groupProfile = GroupProfile() 
		
		if groupName1 == '' or groupName2 == '':
			return groupProfile
		
		groupProfile.groupName1 = groupName1
		groupProfile.groupName2 = groupName2
		
		# get depth of hierarchical levels of interest 
		if parentHeading == 'Entire sample':
			parentDepth = 0
		else:
			parentDepth = self.hierarchyHeadings.index(parentHeading) + 1
			
		profileDepth = self.hierarchyHeadings.index(profileHeading) + 1
		
		groupProfile.hierarchyHeadings = self.hierarchyHeadings[0:profileDepth]
		
		# get list of samples in each group for samples of interest
		samplesInGroup1 = list(set(self.groupDict[groupName1]).intersection(metadata.activeSamples))
		if groupName2 != '<All other samples>':
			samplesInGroup2 = list(set(self.groupDict[groupName2]).intersection(metadata.activeSamples))
		else:
			samplesInGroup2 = set([])
			for groupName in self.groupDict:
				if groupName != groupName1:
					samplesInGroup2 = samplesInGroup2.union(set(self.groupDict[groupName]).intersection(metadata.activeSamples))
			samplesInGroup2 = list(samplesInGroup2)
					
		groupProfile.samplesInGroup1 = sorted(samplesInGroup1)
		groupProfile.samplesInGroup2 = sorted(samplesInGroup2)
		samples = groupProfile.samplesInGroup1 + groupProfile.samplesInGroup2
		
		# get counts for all samples
		leafNodes = self.getLeafNodes()

		# traverse up tree from each leaf node
		parentSeqDict = {} 
		for leaf in leafNodes:
			curDepth = len(self.hierarchyHeadings) 
			
			curNode = leaf
			hierarchy = []
			bRemoveUnclassified = False
			while curNode != None:
				if not curNode.isRoot() and curDepth <= profileDepth:
					hierarchy.append(curNode.name)
				
				# add profile level information
				if curDepth == profileDepth:
					if 'unclassified' in curNode.name.lower():
						if unclassifiedTreatment == 'Remove unclassified reads':
							bRemoveUnclassified = True
							break
						elif unclassifiedTreatment == 'Use only for calculating frequency profiles':
							bRemoveUnclassified = True
					
					if bRemoveUnclassified == False:
						name = curNode.name
						
						# remove ' - #' if feature is being calculated relative to the entire sample
						bTruncatedName = False
						if curNode.isLeaf() and parentDepth == 0:
							if name.rfind(' - #') != -1:
								name = name[0:name.rfind(' - #')]
								bTruncatedName = True
						
						profileEntry = groupProfile.profileDict.get(name)
						if bTruncatedName == True and profileEntry != None:
							bRemoveUnclassified = True
							break
							
						if profileEntry == None:
							profileEntry = GroupProfileEntry()
							profileEntry.featureCounts = [0]*len(samples)
							groupProfile.profileDict[name] = profileEntry
							
						col = 0
						for sampleName in samples:
							profileEntry.featureCounts[col] += leaf.countData[sampleName]
							col += 1
									
				# add parent level information
				if curDepth == parentDepth:
					sequences = parentSeqDict.get(curNode.name)
					if sequences == None:
						sequences = [0]*len(samples)
						parentSeqDict[curNode.name] = sequences
						
					col = 0
					for sampleName in samples:
						sequences[col] += leaf.countData[sampleName]
						col += 1
						
					if bRemoveUnclassified == False:
						profileEntry.parentCounts = sequences
						
				curDepth -= 1
				curNode = curNode.parent
			
			if bRemoveUnclassified == False:
				hierarchy.reverse()
				profileEntry.hierarchy = hierarchy
			
		groupProfile.numParentCategories = len(parentSeqDict)
	
		return groupProfile
		
	def createMultiGroupProfile(self, groupNames, parentHeading, profileHeading, metadata, unclassifiedTreatment):
		multiGroupProfile = MultiGroupProfile() 
		
		multiGroupProfile.groupNames = sorted(groupNames)
		
		# get depth of hierarchical levels of interest 
		if parentHeading == 'Entire sample':
			parentDepth = 0
		else:
			parentDepth = self.hierarchyHeadings.index(parentHeading) + 1
			
		profileDepth = self.hierarchyHeadings.index(profileHeading) + 1
		
		multiGroupProfile.hierarchyHeadings = self.hierarchyHeadings[0:profileDepth]
		
		# get list of samples in each group for samples of interest
		multiGroupProfile.samplesInGroups = []
		samples = []
		multiGroupProfile.smallestGroup = sys.maxint
		for groupName in multiGroupProfile.groupNames:
			samplesInGroup = list(set(self.groupDict[groupName]).intersection(metadata.activeSamples))
			sortedSampleNames = sorted(samplesInGroup)
			multiGroupProfile.samplesInGroups.append(sortedSampleNames)
			samples += sortedSampleNames
			
			if len(sortedSampleNames) < multiGroupProfile.smallestGroup:
				multiGroupProfile.smallestGroup = len(sortedSampleNames)
				
		# get counts for all samples
		leafNodes = self.getLeafNodes()

		# traverse up tree from each leaf node
		parentSeqDict = {} 
		for leaf in leafNodes:
			curDepth = len(self.hierarchyHeadings) 
			
			curNode = leaf
			hierarchy = []
			bRemoveUnclassified = False
			while curNode != None:
				if not curNode.isRoot() and curDepth <= profileDepth:
					hierarchy.append(curNode.name)
				
				# add profile level information
				if curDepth == profileDepth:
					if 'unclassified' in curNode.name.lower():
						if unclassifiedTreatment == 'Remove unclassified reads':
							bRemoveUnclassified = True
							break
						elif unclassifiedTreatment == 'Use only for calculating frequency profiles':
							bRemoveUnclassified = True
					
					if bRemoveUnclassified == False:
						name = curNode.name
						
						# remove ' - #' if feature is being calculated relative to the entire sample
						bTruncatedName = False
						if curNode.isLeaf() and parentDepth == 0:
							if name.rfind(' - #') != -1:
								name = name[0:name.rfind(' - #')]
								bTruncatedName = True
						
						profileEntry = multiGroupProfile.profileDict.get(name)
						if bTruncatedName == True and profileEntry != None:
							bRemoveUnclassified = True
							break
					
						if profileEntry == None:
							profileEntry = GroupProfileEntry()
							profileEntry.featureCounts = [0]*len(samples)
							multiGroupProfile.profileDict[curNode.name] = profileEntry
							
						col = 0
						for sampleName in samples:
							profileEntry.featureCounts[col] += leaf.countData[sampleName]
							col += 1
									
				# add parent level information
				if curDepth == parentDepth:
					sequences = parentSeqDict.get(curNode.name)
					if sequences == None:
						sequences = [0]*len(samples)
						parentSeqDict[curNode.name] = sequences
						
					col = 0
					for sampleName in samples:
						sequences[col] += leaf.countData[sampleName]
						col += 1
						
					if bRemoveUnclassified == False:
						profileEntry.parentCounts = sequences
						
				curDepth -= 1
				curNode = curNode.parent
		
			if bRemoveUnclassified == False:
				hierarchy.reverse()
				profileEntry.hierarchy = hierarchy

		multiGroupProfile.numParentCategories = len(parentSeqDict)
		multiGroupProfile.setActiveGroups(self.groupActive)

		return multiGroupProfile
		