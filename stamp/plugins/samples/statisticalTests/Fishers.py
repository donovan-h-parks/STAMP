#=======================================================================
# Author: Donovan Parks
#
# Perform Fisher's exact test (hypergeometric test) with a minimum-likelihood approach
# to calculating p-values.
#
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

class Fishers(AbstractSampleStatsTestPlugin):
	'''
	Perform Fisher's exact test.
	'''
	
	def __init__(self, preferences):
		AbstractSampleStatsTestPlugin.__init__(self, preferences)
		self.name = 'Fisher\'s exact test'
		
	def logChoose(self, n, k):
		lgn1 = special.gammaln(n+1)
		lgk1 = special.gammaln(k+1)
		lgnk1 = special.gammaln(n-k+1)
		return lgn1 - (lgnk1 + lgk1)
	
	def gaussHypergeometric(self, a,b,c,d):
		return math.exp(self.logChoose(a+b,a) +
											self.logChoose(c+d,c) -
											self.logChoose(a+b+c+d,a+c))
	
	def hypergeometricPDF(self, a,b,c,d):
		return self.gaussHypergeometric(a,b,c,d)
	
	def hypothesisTest(self, seq1, seq2, totalSeq1, totalSeq2):
		a = int(math.floor(seq1 + 0.5))
		b = int(math.floor(seq2 + 0.5))
		c = int(math.floor(totalSeq1 - seq1 + 0.5))
		d = int(math.floor(totalSeq2 - seq2 + 0.5))
		
		r1 = a+b
		r2 = c+d
		c1 = a+c
		c2 = b+d
		
		P_cutoff = self.hypergeometricPDF(a,b,c,d)
		
		pValueTwoSided = 0
		pValueRight = 0
		pValueLeft = 0
		for i in xrange(0, min(int(round(r1)), int(round(c1)))+1):
			a = i
			b = r1 - a
			c = c1 - a
			d = c2 - b
			
			if b >= 0 and c >= 0 and d >= 0 and d == r2-c:
				p =	self.hypergeometricPDF(a,b,c,d)
				
				if p <= P_cutoff:
					pValueTwoSided += p
				
				# One could assess which tables are more extreme than the observed table by considering an
				# effect size measure. This is suggested by Wolfram (http://mathworld.wolfram.com/FishersExactTest.html) and in 
				# Barnard's, "Significance Tests for 2x2 Tables". The method here based on the hypergeometric distribution
				# is more commonly used (i.e., implemented in R and StatXact) and is known as the 'minimum-likelihood' approach.
				# One can also use a simple doubling of a one-sided p-value. This approach is available in STAMP under the
				# handle of 'hypergeometric test'. 
				#
				# See 'Enrichment of depletion of a GO category within a class of genes: which test?' by Rivals et al. for more details.
						
				if a >= seq1:
					pValueRight += p
					
				if a <= seq1:
					pValueLeft += p
					
		pValueOneSided = pValueLeft
		if pValueOneSided > pValueRight:
			pValueOneSided = pValueRight

		return pValueOneSided, pValueTwoSided, ''

if __name__ == "__main__": 
	fishers = Fishers()
	pValueOneSided, pValueTwoSided = fishers.hypothesisTest(11, 11, 30, 60)
	print pValueOneSided
	print pValueTwoSided
	
	'''
	fout = open('FisherTiming.csv', 'w')
	
	for a in xrange(100, 10001, 100):
		print a
		start = time.time()
		for i in xrange(0, 10):
			pValueOne, pValueTwo = fishers.hypothesisTest(a/2, a/2, 1000000, 1000000)
		elapsed = (time.time() - start) / 10
		fout.write(str(a) + ',' + str(elapsed) + '\n')
		print elapsed
		
	fout.close()
	'''

