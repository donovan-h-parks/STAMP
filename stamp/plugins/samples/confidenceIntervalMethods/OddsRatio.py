#=======================================================================
# Author: Donovan Parks
#
# Calculate odds ratio confidence interval. 
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

class OddsRatio(AbstractSampleConfIntervMethod):
	
	def __init__(self, preferences):
		AbstractSampleConfIntervMethod.__init__(self, preferences)
		self.name = 'OR: Haldane adjustment'
		self.plotLabel = 'Odds ratio'
		self.bRatio = True
		
	def tableValues(self, seq1, seq2, totalSeq1, totalSeq2):
		a = seq1
		b = seq2
		c = totalSeq1 - seq1
		d = totalSeq2 - seq2
		
		# boundary correction (Haldane, 1956 modification; see Agresti, Biometrics 1999)
		note = ''
		if a == 0 or b == 0 or c == 0 or d == 0:
			pseudocount = self.preferences['Pseudocount']
			a += pseudocount
			b += pseudocount
			c += pseudocount
			d += pseudocount
			note = 'degenerate case: CI calculation used pseudocount'
			
		return a, b, c, d, note
		
	def run(self, seq1, seq2, totalSeq1, totalSeq2, coverage):
		'''
		Calculate odds ratio confidence interval. 
		'''
		a, b, c, d, note = self.tableValues(seq1, seq2, totalSeq1, totalSeq2)
		
		effectSize = (float(a) * d) / (float(b) * c)
		logEffectSize = math.log(effectSize)
		
		logSE = math.sqrt(1.0/a + 1.0/b + 1.0/c + 1.0/d)
		
		z = zScore(coverage)
		logLowerCI = logEffectSize - z*logSE
		logUpperCI = logEffectSize + z*logSE
		
		lowerCI = math.exp(logLowerCI)
		upperCI = math.exp(logUpperCI)
		
		return lowerCI, upperCI, effectSize, note
	
if __name__ == "__main__": 
	oddsRatio = OddsRatio()
	lowerCI, upperCI, effectSize = oddsRatio.run(141,420,928+141,13525+420,0.05)
	print lowerCI
	print upperCI
	print effectSize