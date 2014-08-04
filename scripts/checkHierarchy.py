#!/usr/bin/env python

###############################################################################
#                                                                             #
#    This program is free software: you can redistribute it and/or modify     #
#    it under the terms of the GNU General Public License as published by     #
#    the Free Software Foundation, either version 3 of the License, or        #
#    (at your option) any later version.                                      #
#                                                                             #
#    This program is distributed in the hope that it will be useful,          #
#    but WITHOUT ANY WARRANTY; without even the implied warranty of           #
#    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the            #
#    GNU General Public License for more details.                             #
#                                                                             #
#    You should have received a copy of the GNU General Public License        #
#    along with this program. If not, see <http://www.gnu.org/licenses/>.     #
#                                                                             #
###############################################################################

__prog_name__ = 'checkHierarchy'
__prog_desc__ = '<program description>'

__author__ = 'Donovan Parks'
__copyright__ = 'Copyright 2014'
__credits__ = ['Donovan Parks']
__license__ = 'GPL3'
__version__ = '0.0.1'
__maintainer__ = 'Donovan Parks'
__email__ = 'donovan.parks@gmail.com'
__status__ = 'Development'

import os
import sys
import argparse
from collections import defaultdict

def isNumber(s):
	"""Check is a string is a number."""
	try:
		float(s)
		return True
	except ValueError:
		return False

class CheckHierarchy(object):
	def __init__(self):
		pass
		
	def isUnclassified(self, value):
		"""Check if value (taxon, metabolic pathway) is unclassified."""

		# currently unclassified sequences need to be explicitly stated as
		# 'unclassified' (case insensitive) or '*__unclassified' which is
		# the format used by GreenGenes
		return value.lower() == 'unclassified' or value.lower()[1:] == '__unclassified'

	def determineHierarchicalColumns(self, headerValues, firstDataValues):
		"""Determine columns corresponding to user-defined hierarchy.""" 

		# first column entry that is numeric is assumed to be from first sample
		firstSampleIndex = 0
		for entry in firstDataValues:
			if isNumber(entry):
				break
			firstSampleIndex += 1

		# sanity check profile
		numSamples = len(headerValues) - firstSampleIndex
		if numSamples < 2:
			print '[Error] Profile must contain at least two samples. Identified %d samples' % numSamples
			sys.exit()

		if firstSampleIndex == 0:
			print '[Error] Profile file must contain at least one column indicating feature names.'
			sys.exit()
			
		print 'Identified %d samples.' % numSamples
		print 'Identified %d hierarchical columns.' % firstSampleIndex

		# get name of hierarchical columns
		columnNames = headerValues[0:firstSampleIndex]

		return columnNames

	def run(self, stampProfile):
		"""Verify that data forms a strict hierarchy."""
		parent = defaultdict(dict)
		
		# identify entries breaking hierarchy
		entriesWithUnclassifiedParents = []
		entriesBreakingHierarchy = []
		with open(stampProfile, 'U') as f:
			header = f.readline()
			headerValues = map(str.strip, header.split('\t'))

			columnNames = None
			for i, line in enumerate(f):
				rowNumber = i+2 # +1 for header row, +1 for zero indexing
				lineSplit = line.split('\t')
				dataValues = map(str.strip, lineSplit)
				
				if len(headerValues) != len(dataValues):
					print '[Error] Line %d does not contain as many entries as the header line.' % rowNumber
					sys.exit()
				
				if not columnNames:
					columnNames = self.determineHierarchicalColumns(headerValues, dataValues)

				categories = dataValues[0:len(columnNames)]
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
						entriesWithUnclassifiedParents.append([rowNumber, r, value])
						continue 

					if r not in parent:
						parent[r] = {}

					if value not in parent[r]:
						parent[r][value] = parentValue
					else:
						if parent[r][value] != parentValue:
							entriesBreakingHierarchy.append([rowNumber, r, value, parent[r][value], parentValue])
							
		# report entries breaking hierarchy
		if len(entriesWithUnclassifiedParents) > 0:
			print ''
			print 'The following entries have an unclassified parent:'
			for entry in entriesWithUnclassifiedParents:
				rowNumber, r, value = entry
				print '%s\t%s\t%s' % (rowNumber, columnNames[r], value)
				
		
		if len(entriesBreakingHierarchy) > 0:
			print ''
			print 'The following entries have two (and potentially more) parents:'
			for entry in entriesBreakingHierarchy:
				rowNumber, r, value, parent1, parent2 = entry
				print '%s\t%s\t%s\t%s' % (rowNumber, columnNames[r], value, ','.join([parent1, parent2]))
				
		if len(entriesWithUnclassifiedParents) == 0 and len(entriesBreakingHierarchy) == 0:
			print ''
			print 'Profile forms a strict hierarchy. You are good to go!'

if __name__ == '__main__':
	print __prog_name__ + ' v' + __version__ + ': ' + __prog_desc__
	print '  by ' + __author__ + ' (' + __email__ + ')' + '\n'

	parser = argparse.ArgumentParser(formatter_class=argparse.ArgumentDefaultsHelpFormatter)
	parser.add_argument('stamp_profile', help='STAMP profile to evaluate')

	args = parser.parse_args()

	try:
		checkHierarchy = CheckHierarchy()
		checkHierarchy.run(args.stamp_profile)
	except SystemExit:
		print "\nControlled exit resulting from an unrecoverable error or warning."
	except:
		print "\nUnexpected error:", sys.exc_info()[0]
		raise
