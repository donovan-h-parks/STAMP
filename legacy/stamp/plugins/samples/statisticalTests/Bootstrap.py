#=======================================================================
# Author: Donovan Parks
#
# Perform bootstrap test.
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

import random
from stamp.plugins.samples.AbstractSampleStatsTestPlugin import AbstractSampleStatsTestPlugin

from numpy.random import binomial

class Bootstrap(AbstractSampleStatsTestPlugin):
	'''
	Perform bootstrap test.
	'''

	def __init__(self, preferences):
		AbstractSampleStatsTestPlugin.__init__(self, preferences)
		self.name = 'Bootstrap'

	def hypothesisTest(self, seq1, seq2, totalSeq1, totalSeq2):
		replicates = self.preferences['Replicates']

		# create null distribution
		if totalSeq1 != 0 and totalSeq2 != 0:
			pooledN = totalSeq1 + totalSeq2
			pooledP = float(seq1 + seq2) / pooledN

			diff = []
			for dummy in xrange(0, replicates):
				c1 = binomial(totalSeq1, pooledP)
				c2 = binomial(totalSeq2, pooledP)
					
				diff.append(float(c1) / totalSeq1 - float(c2) / totalSeq2) 

			# determine number of replicates w/ an effect size more extreme than the observed data
			obsDiff = float(seq1) / totalSeq1 - float(seq2) / totalSeq2

			leftCount = 0
			rightCount = 0
			twoSidedCount = 0
			for value in diff:
				if value <= obsDiff:
					leftCount += 1
				if value >= obsDiff:
					rightCount += 1
				if abs(value) >= abs(obsDiff):
					twoSidedCount += 1
				
			oneSidedCount = leftCount
			if rightCount < oneSidedCount:
				oneSidedCount = rightCount
			note = ''
		else:
			oneSidedCount = replicates
			twoSidedCount = replicates
			note = 'degenerate case: parent has a count of zero'

		return float(oneSidedCount) / replicates, float(twoSidedCount) / replicates, ''
 
if __name__ == "__main__": 
	bootstrap = Bootstrap()
	pValueOneSided, pValueTwoSided = bootstrap.hypothesisTest(20, 1, 50, 50)
	print pValueOneSided, pValueTwoSided