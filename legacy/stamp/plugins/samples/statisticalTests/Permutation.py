#=======================================================================
# Author: Donovan Parks
#
# Perform non-parametric permutation test.
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

from stamp.plugins.samples.AbstractSampleStatsTestPlugin import AbstractSampleStatsTestPlugin

from numpy.random import hypergeometric

class Permutation(AbstractSampleStatsTestPlugin):
	'''
	Perform bootstrap non-parametric statistical hypothesis test.
	'''

	def __init__(self, preferences):
		AbstractSampleStatsTestPlugin.__init__(self, preferences)
		self.name = 'Permutation'

	def hypothesisTest(self, seq1, seq2, totalSeq1, totalSeq2):
		replicates = self.preferences['Replicates']

		if totalSeq1 != 0 and totalSeq2 != 0:
			# observed difference
			obsDiff = float(seq1) / totalSeq1 - float(seq2) / totalSeq2

			# randomly permute assignment of sequences
			permutationDiffs = []
			posSeq = seq1+seq2
			negSeq = totalSeq1+totalSeq2-posSeq
			for dummy in xrange(0, replicates):
				c1 = hypergeometric(posSeq, negSeq, totalSeq1)         
				c2 = posSeq - c1 
					
				permutationDiffs.append(float(c1) / totalSeq1 - float(c2) / totalSeq2) 
				  
			# find p-value of permutation test (number of replicates with a value lower/greater than the observed value)
			leftCount = 0
			rightCount = 0
			twoSidedCount = 0
			for value in permutationDiffs:
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

		return float(oneSidedCount) / replicates, float(twoSidedCount) / replicates, note
  
 
if __name__ == "__main__": 
	permutation = Permutation()
	pValueOneSided, pValueTwoSided = permutation.hypothesisTest(11, 11, 30, 60)
	print pValueOneSided
	print pValueTwoSided