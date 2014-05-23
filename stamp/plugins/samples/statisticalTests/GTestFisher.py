#=======================================================================
# Author: Donovan Parks
#
# Perform G-test with Yates' continuity correction unless there is a
# table entry with < 20 samples in which case Fisher's exact test (hypergeometric test) with a 
# is used.
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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.	See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.	If not, see <http://www.gnu.org/licenses/>.
#=======================================================================

from stamp.plugins.samples.AbstractSampleStatsTestPlugin import AbstractSampleStatsTestPlugin

from stamp.plugins.samples.statisticalTests.Fishers import Fishers
from stamp.plugins.samples.statisticalTests.GTestYates import GTestYates

from scipy import special

class GTestFisher(AbstractSampleStatsTestPlugin):
	'''
	Perform G-test w/ Yates' correction or Fisher's exact test.
	'''
	
	def __init__(self, preferences):
		AbstractSampleStatsTestPlugin.__init__(self, preferences)
		self.name = 'G-test (w/ Yates\') + Fisher\'s'
		self.fishers = Fishers(self.preferences)
		self.gTestYates = GTestYates(self.preferences)
		
	def hypothesisTest(self, seq1, seq2, totalSeq1, totalSeq2):
		a = seq1
		b = seq2
		c = totalSeq1 - seq1
		d = totalSeq2 - seq2
		
		if a < 20 or b < 20 or c < 20 or d < 20:
			return self.fishers.hypothesisTest(seq1, seq2, totalSeq1, totalSeq2)
		else:
			return self.gTestYates.hypothesisTest(seq1, seq2, totalSeq1, totalSeq2)
	
if __name__ == "__main__": 
	pass

