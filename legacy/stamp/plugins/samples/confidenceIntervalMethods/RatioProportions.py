#=======================================================================
# Author: Donovan Parks
#
# Calculate ratio of proportions (relative risk) confidence interval. 
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

class RatioProportions(AbstractSampleConfIntervMethod):
	
	def __init__(self, preferences):
		AbstractSampleConfIntervMethod.__init__(self, preferences)
		self.name = 'RP: Asymptotic'
		self.plotLabel = 'Ratio of proportions'
		self.bRatio = True
		
	def run(self, seq1, seq2, totalSeq1, totalSeq2, coverage):
		'''
		Calculate ratio of proportions (relative risk) confidence interval. 
		'''
		note = ''
		if seq1 == 0 or seq2 == 0:
			pseudocount = self.preferences['Pseudocount']
			seq1 += pseudocount
			seq2 += pseudocount
			totalSeq1 += 2*pseudocount
			totalSeq2 += 2*pseudocount
			note = 'degenerate case: CI calculation used pseudocount'
			
		effectSize = (float(seq1) / totalSeq1) / (float(seq2) / totalSeq2)
		logEffectSize = math.log(effectSize)
		
		logSE = math.sqrt(1.0/seq1 - 1.0/totalSeq1 + 1.0/seq2 - 1.0/totalSeq2)
		
		z = zScore(coverage)
		logLowerCI = logEffectSize - z*logSE
		logUpperCI = logEffectSize + z*logSE
		
		lowerCI = math.exp(logLowerCI)
		upperCI = math.exp(logUpperCI)
		
		return lowerCI, upperCI, effectSize, note
	
if __name__ == "__main__": 
		ratioProp = RatioProportions()
		lowerCI, upperCI, effectSize = ratioProp.run(14, 17, 23, 19,0.05)
		print lowerCI
		print upperCI
		print effectSize