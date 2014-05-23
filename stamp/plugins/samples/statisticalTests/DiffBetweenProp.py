#=======================================================================
# Author: Donovan Parks
#
# Perform difference between proportions statistical hypothesis test.
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
from stamp.metagenomics.stats.distributions.NormalDist import standardNormalCDF, zScore

class DiffBetweenProp(AbstractSampleStatsTestPlugin):
	'''
	Perform difference between proportions statistical hypothesis test.
	'''
	
	def __init__(self, preferences):
		AbstractSampleStatsTestPlugin.__init__(self, preferences)
		self.name = 'Difference between proportions'
	
	def hypothesisTest(self, seq1, seq2, totalSeq1, totalSeq2):
		if (seq1 == 0 and seq2 == 0) or (seq1 == totalSeq1 and seq2 == totalSeq2):
			D = 0
			note = 'degenerate case: suspect p-value'
		else:
			R1 = float(seq1) / totalSeq1
			R2 = float(seq2) / totalSeq2
			diff = R1 - R2
			P = float(seq1 + seq2) / (totalSeq1 + totalSeq2)
			Q = 1.0 - P
		
			D = diff / math.sqrt(P*Q*((1.0/totalSeq1) + (1.0/totalSeq2)))
			note = ''
		
		# calculate one-sided and two-sided p-value
		ZScore = abs(D)
		pValueOneSided = standardNormalCDF(ZScore)
		if pValueOneSided > 0.5:
			pValueOneSided = 1.0 - pValueOneSided
		pValueTwoSided = 2*pValueOneSided
	
		return pValueOneSided, pValueTwoSided, note
	
	def power(self, seq1, seq2, totalSeq1, totalSeq2, alpha): 
		oneMinusAlpha = 1.0 - alpha
		 
		p1 = float(seq1) / totalSeq1
		p2 = float(seq2) / totalSeq2
		d = p1 - p2

		stdDev = math.sqrt( (p1 * (1-p1)) / totalSeq1 + (p2 * (1 - p2)) / totalSeq2 )
		
		if stdDev != 0:		
			p = float(totalSeq1*p1 + totalSeq2*p2) / (totalSeq1 + totalSeq2)
			q = 1-p
			pooledStdDev = math.sqrt( (p*q) / totalSeq1 + (p*q) / totalSeq2 )
			
			zScore = zScore(oneMinusAlpha)
			zLower = ( -zScore * pooledStdDev - d ) / stdDev
			zUpper= ( zScore * pooledStdDev - d ) / stdDev
		
			return standardNormalCDF(zLower) + (1.0 - standardNormalCDF(zUpper))
		else:
			return 1.0
	
	
	def equalSampleSize(self, seq1, seq2, totalSeq1, totalSeq2, alpha, beta):
		oneMinusAlpha = 1.0 - alpha
		oneMinusBeta = 1.0 - beta
		
		p1 = float(seq1) / totalSeq1
		p2 = float(seq2) / totalSeq2
		q1 = 1.0 - p1
		q2 = 1.0 - p2
		d = p1 - p2
		
		if d == 0:
			return 1	

		return (zScore(oneMinusAlpha) * math.sqrt((p1 + p2)*(q1 + q2)/2) + zScore(oneMinusBeta)*math.sqrt((p1*q1) + (p2*q2)))**2 / (d**2)


if __name__ == "__main__": 
	diffBetweenProp = DiffBetweenProp()
	pValueOne, pValueTwo = diffBetweenProp.hypothesisTest(23, 10, 13221, 2317)
	print pValueOne
	print pValueTwo