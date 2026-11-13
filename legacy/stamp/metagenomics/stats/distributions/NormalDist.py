#=======================================================================
# Author: Donovan Parks
#
# Perform statistical tests for two samples.
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
from scipy.stats import norm
from scipy.special import erfinv
						 
def standardNormalCDF(z):
	'''
	Standard normal cumulative distribution function
	'''
	return norm.cdf(z)

def zScore(area):
	if area == 0.90:	# hard-coded, normal case
		return 1.6448536269514722
	elif area == 0.95:
		return 1.959963984540054
	elif area == 0.98:
		return 2.3263478740408408
	elif area == 0.99:
		return 2.5758293035489004
		
	return erfinv(area)*math.sqrt(2.0)

if __name__ == "__main__": 
	pass
