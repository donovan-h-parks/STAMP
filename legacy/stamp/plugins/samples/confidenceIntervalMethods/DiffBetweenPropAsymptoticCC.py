#=======================================================================
# Author: Donovan Parks
#
# Asymptotic confidence interval with continuity correction often used 
# with a difference between proportions test.
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

import math

from stamp.plugins.samples.AbstractSampleConfIntervMethod import AbstractSampleConfIntervMethod

from stamp.metagenomics.stats.distributions.NormalDist import zScore

class DiffBetweenPropAsymptoticCC(AbstractSampleConfIntervMethod):
	
	def __init__(self, preferences):
		AbstractSampleConfIntervMethod.__init__(self, preferences)
		self.name = 'DP: Asymptotic-CC'
		self.plotLabel = 'Difference between proportions (%)'
		self.bRatio = False

	def run(self, seq1, seq2, totalSeq1, totalSeq2, coverage):
		'''
		Calculate confidence interval using asymptotic method with a continuity correction.
			Results are report as percent difference.
		'''
		note = ''
		
		if totalSeq1 == 0:
			totalSeq1 = self.preferences['Pseudocount']
			note = 'degenerate case: CI calculation used pseudocount'
			
		if totalSeq2 == 0:
			totalSeq2 = self.preferences['Pseudocount']
			note = 'degenerate case: CI calculation used pseudocount'
			
		R1 = float(seq1) / totalSeq1
		R2 = float(seq2) / totalSeq2
	
		diff = R1 - R2
		stdErr = math.sqrt((R1*(1-R1)) / totalSeq1 + (R2*(1-R2)) / totalSeq2) + (1.0/totalSeq1 + 1.0/totalSeq2)/2
		offset = zScore(coverage) * stdErr
	
		return (diff - offset) * 100, (diff + offset) * 100, diff * 100, note
	
if __name__ == "__main__": 
	pass