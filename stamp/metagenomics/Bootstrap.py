#=======================================================================
# Author: Donovan Parks
#
# Stores hierarchical profile information.
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

import numpy
from numpy import mean

def bootstrapDiffOfMeanProp(group1, group2, coverage, replicates = 1000):
	sampleSize1 = len(group1)
	sampleSize2 = len(group2)
	
	g1 = numpy.asarray(group1) # Don't copy if possible
	g2 = numpy.asarray(group2)

	distribution = []
	for _ in range(0, replicates):
		# Draw samples from groups at random, with replacement
		choices1 = numpy.random.randint(0, sampleSize1, sampleSize1)
		choices2 = numpy.random.randint(0, sampleSize2, sampleSize2)
		samplesGroup1 = g1[choices1]
		samplesGroup2 = g2[choices2]
		
		# Apply function to sample of random distribution
		diffOfMeanProp = mean(samplesGroup1) - mean(samplesGroup2)
		distribution.append(diffOfMeanProp)

	distribution.sort()
	lowerCI = distribution[max(0, int(math.floor(0.5*(1.0-coverage)*len(distribution))))]
	upperCI = distribution[min(len(distribution)-1, int(math.ceil((coverage + 0.5*(1.0-coverage))*len(distribution))))]

	return lowerCI, upperCI

	
