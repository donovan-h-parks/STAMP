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

	import os

	def read(self, filename):
		errMsg = None

		# 1. Python 3: Remove 'U', use 'r' with utf-8 encoding
		try:
			with open(filename, 'r', encoding='utf-8', errors='ignore') as fin:
				# map in Python 3 returns an iterator; we need a list for indexing
				data = [line.strip() for line in fin.readlines()]
		except IOError:
			return None, "Unable to open file."

		profileTree = ProfileTree()

		# 2. determine number of hierarchical levels and samples
		self.determineColumns(data, profileTree)

		if profileTree.numSamples() < 2:
			errMsg = 'Profile file must contain at least two samples.'
			return None, errMsg

		if profileTree.numHierarchicalLevels() == 0:
			errMsg = 'Profile file must contain a column indicating feature names.'
			return None, errMsg

		# verify data forms a strict hierarchy
		errMsg = self.checkHierarchy(data, profileTree.numHierarchicalLevels())
		if errMsg is not None:
			return None, errMsg

		# construct profile tree
		try:
			# Initialize with 0.0 to ensure float arithmetic from the start
			profileTree.numSeqInSample = [0.0] * profileTree.numSamples()
			for i in range(1, len(data)):
				# ignore blank lines
				if not data[i].strip():
					continue

				# 3. Use list comprehension instead of map(string.strip, ...)
				# This converts the map iterator into a usable list of strings
				lineSplit = [item.strip() for item in data[i].split('\t')]

				numH = profileTree.numHierarchicalLevels()
				categories = lineSplit[0:numH]
				countData = [float(count) for count in lineSplit[numH:]]

				# check for unclassified categories
				taxa = ''
				for j in range(0, len(categories)):
					if self.isUnclassified(categories[j]):
						categories[j] = ('Unclassified ' + taxa).rstrip()
					else:
						taxa = categories[j]

				# 4. Build the tree structure
				curNode = profileTree.root
				for category in categories:
					node = curNode.childWithName(category)
					if node is None:
						node = Node(category, curNode)
						curNode.children.append(node)

					curNode = node

				# add count data to leaf node
				for j in range(0, len(profileTree.sampleNames)):
					sampleName = profileTree.sampleNames[j]
					# Ensure the sampleName key exists in the node's countData dictionary
					curNode.countData[sampleName] = curNode.countData.get(sampleName, 0.0) + countData[j]

				# add count data to total sequence count
				for j in range(0, len(countData)):
					profileTree.numSeqInSample[j] += countData[j]

		except Exception as e:
			# Providing the actual error 'e' helps debug specific Python 3 issues
			errMsg = f'Failed to correctly parse line {i + 1}: {str(e)}'

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
		headings = [line.strip() for line in headings]
		profileTree.hierarchyHeadings = headings[0:firstSampleIndex]
		profileTree.sampleNames = headings[firstSampleIndex:]
		
	def checkHierarchy(self, data, numHierarchicalLevels):
		"""Verify that data forms a strict hierarchy."""
		parent = defaultdict(dict)
		for line in data:
			lineSplit = line.split('\t')
			lineSplit = [item.strip() for item in lineSplit]
				
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
			