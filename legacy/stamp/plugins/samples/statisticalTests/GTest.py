#=======================================================================
# Author: Donovan Parks
#
# Perform G-test statistical hypothesis test.
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

import math

from stamp.plugins.samples.AbstractSampleStatsTestPlugin import AbstractSampleStatsTestPlugin

from scipy.stats import chi2

class GTest(AbstractSampleStatsTestPlugin):
	'''
	Perform G-test statistical hypothesis test
	'''

	def __init__(self, preferences):
		AbstractSampleStatsTestPlugin.__init__(self, preferences)
		self.name = 'G-test'

	def hypothesisTest(self, seq1, seq2, totalSeq1, totalSeq2):
		# Contingency table:
		# x1 x2
		# y1 y2
		x1 = seq1
		x2 = seq2
		y1 = totalSeq1 - x1
		y2 = totalSeq2 - x2

		# calculate g-test statistic
		N = x1+x2+y1+y2

		if N > 0:
			E00 = float((x1+x2) * (x1+y1)) / N
			E01 = float((x1+x2) * (x2+y2)) / N
			E10 = float((y1+y2) * (x1+y1)) / N
			E11 = float((y1+y2) * (x2+y2)) / N
			
			gTest = 0
			if (x1 != 0):
			  gTest = x1 * math.log(x1/E00)
			  
			if (x2 != 0):
			  gTest += x2 * math.log(x2/E01)
			  
			if (y1 != 0):
			  gTest += y1 * math.log(y1/E10)
			  
			if (y2 != 0):
			  gTest += y2 * math.log(y2/E11)
			  
			gTest = 2*gTest

			# calculate p-value
			pValueTwoSided = 1.0 - chi2.cdf(gTest,1)
			note = ''
		else:
			pValueTwoSided = 1.0
			note = 'degenerate case: all values are zero'
			
		return float('inf'), pValueTwoSided, note

if __name__ == "__main__": 
	gTest = GTest()
	pValueOne, pValueTwo = gTest.hypothesisTest(10, 20, 60, 50)
	print pValueOne
	print pValueTwo
