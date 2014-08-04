#=======================================================================
# Author: Donovan Parks
#
# Create a feature profile for a pair of metagenomic samples.
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
from collections import defaultdict

from stamp.metagenomics.ProfileTree import ProfileTree, Node
from stamp.metagenomics.StringHelper import isNumber

class StampIO(object):
	def __init__(self, preferences):
		self.preferences = preferences
		
	def read(self, filename):
		errMsg = None
		
		fin = open(filename, 'U')
		data = map(string.strip, fin.readlines())
		fin.close()

		profileTree = ProfileTree()
		
		# determine number of hierarchical levels and samples
		self.determineColumns(data, profileTree)

		if profileTree.numSamples() < 2:
			errMsg = 'Profile file must contain at least two samples.'
			return None, errMsg
		
		if profileTree.numHierarchicalLevels() == 0:
			errMsg = 'Profile file must contain a column indicating feature names.'
			return None, errMsg
		
		# verify data forms a strict hierarchy
		errMsg = self.checkHierarchy(data, profileTree.numHierarchicalLevels())
		if errMsg != None:
			return None, errMsg
		
		# construct profile tree
		try:
			profileTree.numSeqInSample = [0] * profileTree.numSamples()
			for i in xrange(1, len(data)):
				# ignore blank lines
				if data[i].strip() == "":
					continue

				lineSplit = data[i].split('\t')
				lineSplit = map(string.strip, lineSplit)
				
				categories = lineSplit[0:profileTree.numHierarchicalLevels()]
				countData = [float(count) for count in lineSplit[profileTree.numHierarchicalLevels():]]
				
				# check for unclassified categories
				taxa = ''
				for j in xrange(0, len(categories)):
					if self.isUnclassified(categories[j]):
						categories[j] = 'Unclassified ' + taxa
						categories[j] = categories[j].rstrip()
					else:
						taxa = categories[j]
				
				# add all hierarchical levels
				curNode = profileTree.root
				for category in categories:
					node = curNode.childWithName(category)
					if node == None:
						node = Node(category, curNode)
						curNode.children.append(node)
						
					curNode = node
					
				# add count data to leaf node
				for j in xrange(0, len(profileTree.sampleNames)):
					sampleName = profileTree.sampleNames[j]
					curNode.countData[sampleName] = curNode.countData.get(sampleName, 0) + countData[j]
						
				# add count data to total sequence count
				for j in xrange(0, len(countData)):
					profileTree.numSeqInSample[j] += countData[j]
		except:
			errMsg = 'Failed to correctly parse line: ' + str(i+1)
			
		return profileTree, errMsg
	
	def isUnclassified(self, value):
		"""Check if value (taxon, metabolic pathway) is unclassified."""
		
		# currently unclassified sequences need to be explicitly stated as
		# 'unclassified' (case insensitive) or '*__unclassified' which is
		# the format used by GreenGenes
		return value.lower() == 'unclassified' or value.lower()[1:] == '__unclassified'
				
	def determineColumns(self, data, profileTree):
		firstDataRow = data[1].split('\t')
		
		# first column entry that is numeric is assumed to be from first sample
		firstSampleIndex = 0
		for entry in firstDataRow:
			if isNumber(entry):
				break
			firstSampleIndex += 1
			
		# get hierarchical and sample names
		headings = data[0].split('\t')
		headings = map(string.strip, headings)
		profileTree.hierarchyHeadings = headings[0:firstSampleIndex]
		profileTree.sampleNames = headings[firstSampleIndex:]
		
	def checkHierarchy(self, data, numHierarchicalLevels):
		"""Verify that data forms a strict hierarchy."""
		parent = defaultdict(dict)
		for line in data:
			lineSplit = line.split('\t')
			lineSplit = map(string.strip, lineSplit)
				
			categories = lineSplit[0:numHierarchicalLevels]
			for r, value in enumerate(categories):
				# top of hierarchy has no parent
				if r == 0:
					continue 
				
				# ignore unclassified sequences
				if self.isUnclassified(value):
					continue 
				
				# make sure parent is not unclassified
				parentValue = categories[r-1]
				if self.isUnclassified(parentValue):
					return "Child %s has an unclassified parent." % value
					continue 
				
				if r not in parent:
					parent[r] = {}
					
				if value not in parent[r]:
					parent[r][value] = parentValue
				else:
					if parent[r][value] != parentValue:
						# data is not a strict hierarchy
						return "Data does not form a strict hierarchy. Child %s has multiple parents (e.g., %s, %s)." % (value, parent[r][value], parentValue)		
		return None
			