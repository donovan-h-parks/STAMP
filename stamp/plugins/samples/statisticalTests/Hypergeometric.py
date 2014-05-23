#=======================================================================
# Author: Donovan Parks
#
# Perform hypergeometric test (Fisher's exact test) with a 'doubling' approach
# to calculating p-values.

# See 'Enrichment of depletion of a GO category within a class of genes: which test?' 
#	by Rivals et al., Bioinformatics, 2007 for more details.
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

from stamp.plugins.samples.AbstractSampleStatsTestPlugin import AbstractSampleStatsTestPlugin

from scipy import special

class Hypergeometric(AbstractSampleStatsTestPlugin):
	'''
	Perform hypergeometric test.
	'''
	
	def __init__(self, preferences):
		AbstractSampleStatsTestPlugin.__init__(self, preferences)
		self.name = 'Hypergeometric'
		
	def logChoose(self, n, k):
		lgn1 = special.gammaln(n+1)
		lgk1 = special.gammaln(k+1)
		lgnk1 = special.gammaln(n-k+1)
		
		return lgn1 - (lgnk1 + lgk1)
	
	def hypergeometricPDF(self, a,b,c,d):
		return math.exp(self.logChoose(a+b,a) +
											self.logChoose(c+d,c) -
											self.logChoose(a+b+c+d,a+c))
	
	def hypergeometricCDF(self, a,b,c,d):
		cdf = 0
		for i in xrange(0, int(a)+1):
			cdf += self.hypergeometricPDF(i,b+(a-i),c+(a-i),d-(a-i))
			
		if cdf > 1.0:
			# this is possible due to rounding error accumulated over the summation
			cdf = 1.0
			
		return cdf
			
	def hypothesisTest(self, seq1, seq2, totalSeq1, totalSeq2):
		a = seq1
		b = seq2
		c = totalSeq1 - seq1
		d = totalSeq2 - seq2

		if seq1 > seq2:
			pValueLeft = self.hypergeometricCDF(a,b,c,d)
		else:
			pValueLeft = self.hypergeometricCDF(b,a,d,c)
			
		pValueRight = (1.0 - pValueLeft) + self.hypergeometricPDF(a,b,c,d)
		
		pValueTwoSided = 2 * min(pValueLeft, pValueRight)
					
		pValueOneSided = pValueLeft
		if pValueOneSided > pValueRight:
			pValueOneSided = pValueRight

		return pValueOneSided, pValueTwoSided, ''

if __name__ == "__main__": 
	preferences = {}
	hypergeometric = Hypergeometric(preferences)
	pValueOneSided, pValueTwoSided = hypergeometric.hypothesisTest(10, 30, 100, 700)
	print pValueOneSided
	print pValueTwoSided
	
	fout = open('HypergeometricTiming.csv', 'w')
	
	import time
	for a in xrange(100, 10001, 100):
		print a
		start = time.time()
		for i in xrange(0, 10):
			pValueOne, pValueTwo = hypergeometric.hypothesisTest(a/10, a/10, 1000000, 1000000)
		elapsed = (time.time() - start) / 10
		fout.write(str(a) + ',' + str(elapsed) + '\n')
		print elapsed
		
	fout.close()
