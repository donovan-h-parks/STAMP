#=======================================================================
# Author: Donovan Parks
#
# Wilson score confidence interval for a binomial proportion. Incorporates small probability modification proposed in
#  "Interval Estimation for a Binomial Proportion" by Lawrence D. Brown, T. Tony Cai and Anirban DasGupta in Statistical Science,
#   2001. 
# 
# Also see:
#  "Two-sided confidence intervals for the single proportion: Comparison of seven methods"
#  by Newcombe, R.G in STATISTICS IN MEDICINE, 1998
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

from stamp.metagenomics.stats.distributions.NormalDist import zScore

from scipy.stats import chi2

class WilsonCI():
	
	def __init__(self):
		self.name = 'Wilson with Brown modification'
		
	def run(self, posSeqs, totalSeqs, coverage, zScore):
		'''
		 Calculate Wilson CI for a binomial distribution. 
		 
		 posSeqs: number of positive sequences
		 totalSeqs: total sequences drawn
		 coverage: desired coverage
		 zScore: standard normal z-value corresponding to desired coverage
		 
		 Note: clearly zScore could be calculates in this function. However, this calculation is expensive.
		 To facilitate calling this function on several different binomial random variables this is taken as a
		 parameter so it only needs to be calculated once.
		 '''
		 
		totalSeqs = max(totalSeqs, 1.0) 
		
		z = zScore
		zSqrd = z*z
		
		p = float(posSeqs) / totalSeqs
		q = 1.0 - p

		term1 = p + zSqrd / (2*totalSeqs)
		offset = z * math.sqrt(p*q / totalSeqs + zSqrd / (4*totalSeqs*totalSeqs))
		denom = 1 + zSqrd / totalSeqs
		
		lowerCI = (term1 - offset) / denom
		upperCI = (term1 + offset) / denom
		
		# Good correction, but computationally expensive
		#if posSeqs >= 1 and posSeqs <=3:
			# use one-sided Poisson approximation when probability ~= 0 (see Brown et al., 2001)
		#	lowerCI = 0.5*chi2.isf(coverage, 2*posSeqs) / totalSeqs
		
		return lowerCI, upperCI, p
	
if __name__ == "__main__": 
	wilsonCI = WilsonCI()
	lowerCI, upperCI, p = wilsonCI.run(10,100, 0.95, zScore(0.95))
	print lowerCI, upperCI, p
