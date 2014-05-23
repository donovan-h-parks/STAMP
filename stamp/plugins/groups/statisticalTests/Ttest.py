#=======================================================================
# Author: Donovan Parks
#
# Perform t-test under the assumption of equal variance.
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
from stamp.plugins.groups.AbstractGroupStatsTestPlugin import AbstractGroupStatsTestPlugin
from numpy import var

from scipy.stats.distributions import t

class Ttest(AbstractGroupStatsTestPlugin):
	'''
	Perform t-test statistical hypothesis test
	'''
	
	def __init__(self, preferences):
		AbstractGroupStatsTestPlugin.__init__(self, preferences)
		
		self.name = "t-test (equal variance)"
		self.confIntervMethods = ["DP: t-test inverted"]
		
	def run(self, seqGroup1, seqGroup2, parentSeqGroup1, parentSeqGroup2, confIntervMethod, coverage):
		note = ''
		
		n1 = len(seqGroup1)
		n2 = len(seqGroup2)
		
		try:
			if n1 < 2 or n2 < 2:
				raise Exception('degenerate case: both groups must contain at least 2 samples')
				
			# calculate proportions
			propGroup1 = []
			for i in xrange(0, n1):
				if parentSeqGroup1[i] > 0:
					propGroup1.append(float(seqGroup1[i]) / parentSeqGroup1[i])
				else:
					propGroup1.append( 0.0 )
					note = 'degenerate case: parent group had a count of zero'
				
			propGroup2 = []
			for i in xrange(0, n2):
				if parentSeqGroup2[i] > 0:
					propGroup2.append(float(seqGroup2[i]) / parentSeqGroup2[i])
				else:
					propGroup2.append( 0.0 )
					note = 'degenerate case: parent group had a count of zero'
			
			# calculate statistics
			meanG1 = float(sum(propGroup1)) / n1
			meanG2 = float(sum(propGroup2)) / n2
			dp = meanG1 - meanG2
			
			varG1 = var(propGroup1, ddof=1)
			varG2 = var(propGroup2, ddof=1)
			
			dof = n1 + n2 - 2
			pooledVar = ((n1 - 1)*varG1 + (n2 - 1)*varG2) / (n1 + n2 - 2)
			sqrtPooledVar = math.sqrt(pooledVar)
			denom = sqrtPooledVar * math.sqrt(1.0/n1 + 1.0/n2)
				
			# p-value
			T_statistic = (meanG1 - meanG2) / denom
			pValue = t.cdf(T_statistic, dof)
			
			# CI
			tCritical = t.isf(0.5 * (1.0-coverage), dof) # 0.5 factor accounts from symmetric nature of distribution
			lowerCI = dp - tCritical*denom
			upperCI = dp + tCritical*denom

		except Exception as note:
			pValue = 0.5
			lowerCI = 0.0
			upperCI = 0.0
			dp = 0.0
		except ZeroDivisionError:
			if meanG1 != meanG2:
				pValue = 0.0 # the difference (at least according to these samples) must be true as there is no variance
			else:
				pValue = 0.5
				
			lowerCI = dp
			upperCI = dp
			note = 'degenerate case: variance of both groups is zero'

		return 1.0 - pValue, 2*min(pValue, 1.0 - pValue), lowerCI*100, upperCI*100, dp*100, note

if __name__ == "__main__": 
	tTest = Ttest()
	pValueOne, pValueTwo, lowerCI, upperCI, dp, note = tTest.run([5,4,6,4,3], [5,2,2,5,6,7], [10,10,10,10,10], [10,10,10,10,10,10], "DP: t-test inverted", 0.95)

