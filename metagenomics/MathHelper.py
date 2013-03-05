#=======================================================================
# Author: Donovan Parks
#
# Provides helpful mathematical functions.
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

def mean(x):
	if len(x) == 0:
		return float('nan')
		
	return float(sum(x)) / len(x)
	
def stdDev(x, mean):
	if len(x) == 0:
		return float('nan')

	sumsq = 0.0
	for i in xrange(0, len(x)):
		sumsq += (x[i] - mean)*(x[i] - mean)
	return math.sqrt(sumsq / len(x))
	
def variance(data, mean):
	if len(data) == 1:
		return 0.0

	v = 0
	for d in data:
		v += (d - mean)*(d - mean)
	v /= (len(data)-1)
	
	return v


	a, b, c, d = abcd[qtype-1]
	n = len(x)
	g, j = modf( a + (n+b) * q -1)
	if j < 0:
			return y[0]
	elif j >= n:					 
			return y[n-1]	 # oct. 8, 2010 y[n]???!! uncaught	off by 1 error!!!

	j = int(floor(j))
	if g ==	0:
		 return y[j]
	else:
		 return y[j] + (y[j+1]- y[j])* (c + d * g)	 
