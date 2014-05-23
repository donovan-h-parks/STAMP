#=======================================================================
# Author: Donovan Parks
#
# Perform ANOVA statistical hypothesis test.
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
#======================================================================='''

import math
from stamp.plugins.multiGroups.AbstractMultiGroupStatsTestPlugin import AbstractMultiGroupStatsTestPlugin

from scipy.stats import f_oneway

class ANOVA(AbstractMultiGroupStatsTestPlugin):
	'''
	Perform ANOVA statistical hypothesis test
	'''
	
	def __init__(self, preferences):
		AbstractMultiGroupStatsTestPlugin.__init__(self, preferences)
		self.name = 'ANOVA'
	
	def hypothesisTest(self, data):
		note = ''
		for group in data:
			if len(group) < 2:
				note = 'degenerate case: at least one group contains less than 2 samples'
				return 1.0, note
				
		F_value, pValue = apply(f_oneway, data)
		if math.isnan(pValue):
			pValue = 1.0 # invalid data for calculating p-value so assume large p-value
			note = 'degenerate case: failed to calculate p-value'
			
		return pValue, note

if __name__ == "__main__": 
	anova = ANOVA()
	pValueOne, pValueTwo = anova.hypothesisTest([[10, 20, 30], [20, 30, 40], [10, 30, 50, 70]])
	print pValueOne
	print pValueTwo
