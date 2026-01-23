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
# =======================================================================
# Author: Donovan Parks / Python 3 conversion
# =======================================================================

import sys

from stamp.metagenomics.SampleProfile import SampleProfile, SampleProfileEntry
from stamp.metagenomics.GroupProfile import GroupProfile, GroupProfileEntry
from stamp.metagenomics.MultiGroupProfile import MultiGroupProfile


class Node:
	def __init__(self, name, parent=None):
		self.name = name
		self.parent = parent
		self.children = []
		self.countData = {}

	def depth(self):
		depth = 0
		curNode = self
		while curNode.parent is not None:
			depth += 1
			curNode = curNode.parent
		return depth

	def isLeaf(self):
		return len(self.children) == 0

	def isRoot(self):
		return self.parent is None

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
		try:
			index = self.sampleNames.index(name)
			return self.numSeqInSample[index]
		except ValueError:
			return 0

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
			return node  # Fixed: Original returned 'self', likely a bug
		elif node.isLeaf():
			return None

		for child in node.children:
			found_node = self.getNodeWithName(child, name)  # Fixed: Typo in original 'getNodeWidthName'
			if found_node is not None:
				return found_node
		return None

	def getLeafNodes(self):
		leafNodes = []
		for child in self.root.children:
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

		if parentHeading == 'Entire sample':
			parentDepth = 0
		else:
			parentDepth = self.hierarchyHeadings.index(parentHeading) + 1

		profileDepth = self.hierarchyHeadings.index(profileHeading) + 1
		profile.hierarchyHeadings = self.hierarchyHeadings[0:profileDepth]

		profile.sampleNames = [sampleName1, sampleName2]
		leafNodes = self.getLeafNodes()

		parentSeqDict = {}
		for leaf in leafNodes:
			curDepth = len(self.hierarchyHeadings)
			curNode = leaf
			hierarchy = []
			bRemoveUnclassified = False
			profileEntry = None

			while curNode is not None:
				if not curNode.isRoot() and curDepth <= profileDepth:
					hierarchy.append(curNode.name)

				if curDepth == profileDepth:
					if 'unclassified' in curNode.name.lower():
						if unclassifiedTreatment == 'Remove unclassified reads':
							bRemoveUnclassified = True
							break
						elif unclassifiedTreatment == 'Use only for calculating frequency profiles':
							bRemoveUnclassified = True

					if not bRemoveUnclassified:
						name = curNode.name
						bTruncatedName = False
						if curNode.isLeaf() and parentDepth == 0:
							if ' - #' in name:
								name = name[0:name.rfind(' - #')]
								bTruncatedName = True

						profileEntry = profile.profileDict.get(name)
						if bTruncatedName and profileEntry is not None:
							bRemoveUnclassified = True
							break

						if profileEntry is None:
							profileEntry = SampleProfileEntry()
							profileEntry.featureCounts = [0, 0]
							profile.profileDict[name] = profileEntry

						profileEntry.featureCounts[0] += leaf.countData[sampleName1]
						profileEntry.featureCounts[1] += leaf.countData[sampleName2]

				if curDepth == parentDepth:
					sequences = parentSeqDict.get(curNode.name, [0, 0])
					parentSeqDict[curNode.name] = sequences
					sequences[0] += leaf.countData[sampleName1]
					sequences[1] += leaf.countData[sampleName2]

					if not bRemoveUnclassified and profileEntry is not None:
						profileEntry.parentCounts = sequences

				curDepth -= 1
				curNode = curNode.parent

			if not bRemoveUnclassified and profileEntry is not None:
				hierarchy.reverse()
				profileEntry.hierarchy = hierarchy

		profile.numParentCategories = len(parentSeqDict)
		return profile

	def createGroupProfile(self, groupName1, groupName2, parentHeading, profileHeading, metadata,
						   unclassifiedTreatment):
		groupProfile = GroupProfile()
		if not groupName1 or not groupName2:
			return groupProfile

		groupProfile.groupName1 = groupName1
		groupProfile.groupName2 = groupName2

		parentDepth = 0 if parentHeading == 'Entire sample' else self.hierarchyHeadings.index(parentHeading) + 1
		profileDepth = self.hierarchyHeadings.index(profileHeading) + 1
		groupProfile.hierarchyHeadings = self.hierarchyHeadings[0:profileDepth]

		# Group sample logic
		active = set(metadata.activeSamples)
		samplesInGroup1 = list(set(self.groupDict[groupName1]).intersection(active))

		if groupName2 != '<All other samples>':
			samplesInGroup2 = list(set(self.groupDict[groupName2]).intersection(active))
		else:
			samplesInGroup2 = set()
			for gName, gSamples in self.groupDict.items():
				if gName != groupName1:
					samplesInGroup2.update(set(gSamples).intersection(active))
			samplesInGroup2 = list(samplesInGroup2)

		groupProfile.samplesInGroup1 = sorted(samplesInGroup1)
		groupProfile.samplesInGroup2 = sorted(samplesInGroup2)
		samples = groupProfile.samplesInGroup1 + groupProfile.samplesInGroup2

		leafNodes = self.getLeafNodes()
		parentSeqDict = {}

		for leaf in leafNodes:
			curDepth = len(self.hierarchyHeadings)
			curNode = leaf
			hierarchy = []
			bRemoveUnclassified = False
			profileEntry = None

			while curNode is not None:
				if not curNode.isRoot() and curDepth <= profileDepth:
					hierarchy.append(curNode.name)

				if curDepth == profileDepth:
					if 'unclassified' in curNode.name.lower():
						if unclassifiedTreatment == 'Remove unclassified reads':
							bRemoveUnclassified = True
							break
						elif unclassifiedTreatment == 'Use only for calculating frequency profiles':
							bRemoveUnclassified = True

					if not bRemoveUnclassified:
						name = curNode.name
						bTruncatedName = False
						if curNode.isLeaf() and parentDepth == 0:
							if ' - #' in name:
								name = name[0:name.rfind(' - #')]
								bTruncatedName = True

						profileEntry = groupProfile.profileDict.get(name)
						if bTruncatedName and profileEntry is not None:
							bRemoveUnclassified = True
							break

						if profileEntry is None:
							profileEntry = GroupProfileEntry()
							profileEntry.featureCounts = [0] * len(samples)
							groupProfile.profileDict[name] = profileEntry

						for i, sName in enumerate(samples):
							profileEntry.featureCounts[i] += leaf.countData[sName]

				if curDepth == parentDepth:
					sequences = parentSeqDict.get(curNode.name, [0] * len(samples))
					parentSeqDict[curNode.name] = sequences
					for i, sName in enumerate(samples):
						sequences[i] += leaf.countData[sName]

					if not bRemoveUnclassified and profileEntry is not None:
						profileEntry.parentCounts = sequences

				curDepth -= 1
				curNode = curNode.parent

			if not bRemoveUnclassified and profileEntry is not None:
				hierarchy.reverse()
				profileEntry.hierarchy = hierarchy

		groupProfile.numParentCategories = len(parentSeqDict)
		return groupProfile

	def createMultiGroupProfile(self, groupNames, parentHeading, profileHeading, metadata, unclassifiedTreatment):
		multiGroupProfile = MultiGroupProfile()
		multiGroupProfile.groupNames = sorted(groupNames)

		parentDepth = 0 if parentHeading == 'Entire sample' else self.hierarchyHeadings.index(parentHeading) + 1
		profileDepth = self.hierarchyHeadings.index(profileHeading) + 1
		multiGroupProfile.hierarchyHeadings = self.hierarchyHeadings[0:profileDepth]

		multiGroupProfile.samplesInGroups = []
		samples = []
		multiGroupProfile.smallestGroup = sys.maxsize  # Python 3 use maxsize

		active = set(metadata.activeSamples)
		for groupName in multiGroupProfile.groupNames:
			samplesInGroup = sorted(list(set(self.groupDict[groupName]).intersection(active)))
			multiGroupProfile.samplesInGroups.append(samplesInGroup)
			samples += samplesInGroup
			multiGroupProfile.smallestGroup = min(multiGroupProfile.smallestGroup, len(samplesInGroup))

		leafNodes = self.getLeafNodes()
		parentSeqDict = {}

		for leaf in leafNodes:
			curDepth = len(self.hierarchyHeadings)
			curNode = leaf
			hierarchy = []
			bRemoveUnclassified = False
			profileEntry = None

			while curNode is not None:
				if not curNode.isRoot() and curDepth <= profileDepth:
					hierarchy.append(curNode.name)

				if curDepth == profileDepth:
					if 'unclassified' in curNode.name.lower():
						if unclassifiedTreatment == 'Remove unclassified reads':
							bRemoveUnclassified = True
							break
						elif unclassifiedTreatment == 'Use only for calculating frequency profiles':
							bRemoveUnclassified = True

					if not bRemoveUnclassified:
						name = curNode.name
						bTruncatedName = False
						if curNode.isLeaf() and parentDepth == 0:
							if ' - #' in name:
								name = name[0:name.rfind(' - #')]
								bTruncatedName = True

						profileEntry = multiGroupProfile.profileDict.get(name)
						if bTruncatedName and profileEntry is not None:
							bRemoveUnclassified = True
							break

						if profileEntry is None:
							profileEntry = GroupProfileEntry()
							profileEntry.featureCounts = [0] * len(samples)
							multiGroupProfile.profileDict[name] = profileEntry

						for i, sName in enumerate(samples):
							profileEntry.featureCounts[i] += leaf.countData[sName]

				if curDepth == parentDepth:
					sequences = parentSeqDict.get(curNode.name, [0] * len(samples))
					parentSeqDict[curNode.name] = sequences
					for i, sName in enumerate(samples):
						sequences[i] += leaf.countData[sName]

					if not bRemoveUnclassified and profileEntry is not None:
						profileEntry.parentCounts = sequences

				curDepth -= 1
				curNode = curNode.parent

			if not bRemoveUnclassified and profileEntry is not None:
				hierarchy.reverse()
				profileEntry.hierarchy = hierarchy

		multiGroupProfile.numParentCategories = len(parentSeqDict)
		multiGroupProfile.setActiveGroups(self.groupActive)
		return multiGroupProfile