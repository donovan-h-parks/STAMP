'''
Perform Barnard's test.

@author: Donovan Parks
'''

import math
from plugins.samples.AbstractSampleStatsTestPlugin import AbstractSampleStatsTestPlugin

from scipy import special

class Barnard(AbstractSampleStatsTestPlugin):
	'''
	Perform Barnard's test.
	'''
	
	def __init__(self, preferences):
		AbstractSampleStatsTestPlugin.__init__(self, preferences)
		self.name = 'Barnard\'s exact test'
		
	def logChoose(self, n, k):
		lgn1 = special.gammaln(n+1)
		lgk1 = special.gammaln(k+1)
		lgnk1 = special.gammaln(n-k+1)
		return lgn1 - (lgnk1 + lgk1)
	
	def hypothesisTest(self, seq1, seq2, totalSeq1, totalSeq2):
		if (seq1 == 0 and seq2== 0) or (seq1 == totalSeq1 and seq2 == totalSeq2):
			# if the measured effect size is zero, the probability of observing a table
			# more extreme will be large (not strictly 1.0, but large and this degenerate
			# case is problematic to calculate exactly)
			return 1.0, 1.0
		
		# calculate Wald statistic (standardized difference between proportions) for observed data
		pooledP = float(seq1 + seq2) / (totalSeq1 + totalSeq2)
		stdDev = math.sqrt(pooledP * (1.0 - pooledP) * (1.0/totalSeq1 + 1.0/totalSeq2))
		
		obsDiff = float(seq1) /	totalSeq1 - float(seq2) / totalSeq2
		obsStdDiff =	abs(obsDiff / stdDev)
		
		# determine tables more extreme than the observed data
		extremeTables = []
		for a in xrange(0, totalSeq1+1):
			p1 = float(a) / totalSeq1
			
			for b in xrange(0, totalSeq2+1):				
				if (a == 0 and b == 0) or (a == totalSeq1 and b == totalSeq2):
					# difference in proportions is zero so this is not an extreme table
					# but will cause a division by zero exception
					continue
				
				# calculate Wald statistic for current table
				pooledP = float(a + b) / (totalSeq1 + totalSeq2)
				stdDev = math.sqrt(pooledP * (1.0 - pooledP) * (1.0/totalSeq1 + 1.0/totalSeq2))
		
				p2 = float(b) / totalSeq2
				diff = p1 - p2
				stdDiff = abs(diff / stdDev)
				
				if stdDiff >= obsStdDiff:
					# Barnard recommends adding extreme tables under symmetrical and convexity conditions.
					# The latter will be handled here simply by considering all tables. Symmetry may not 
					# occur unless explicitly enforced. Note, that there is no concrete mathematical argument
					# to enforce symmetry. In fact, Barnard describes his general method for finding extreme tables
					# as "laziness".
					if [a,b] not in extremeTables:
						extremeTables.append([a,b])
						extremeTables.append([totalSeq1 - a, totalSeq2 - b])

		# calculate Barnard's p-value (must find optimal population proportion)
		steps = 100
		dPi = 0.5 / float(steps)
		pi = 0.0
		pValueTwoSided = 0
		for dummy in xrange(0,steps):
			pi += dPi
			
			pValue = 0
			for table in extremeTables:
				c1, c2 = table

				nCr1 = math.exp(self.logChoose(totalSeq1,c1))
				nCr2 = math.exp(self.logChoose(totalSeq2,c2)) 
				
				prob = nCr1 * nCr2 * math.power(pi,c1+c2) * math.power(1.0 - pi,totalSeq1+totalSeq2-c1-c2)
				pValue += prob

			if pValue > pValueTwoSided:				
				pValueTwoSided = pValue
	
		return float('inf'), pValueTwoSided

if __name__ == "__main__": 
	barnard = Barnard()
	pValueOne, pValueTwo = barnard.hypothesisTest(8, 3, 100, 100)
	print pValueTwo

