#=======================================================================
# Author: Donovan Parks
#
# Difference between proportions effect size measure.
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

from stamp.plugins.multiGroups.AbstractMultiGroupEffectSizePlugin import AbstractMultiGroupEffectSizePlugin

class EtaSquared(AbstractMultiGroupEffectSizePlugin):
	
	def __init__(self, preferences):
		AbstractMultiGroupEffectSizePlugin.__init__(self, preferences)
		self.name = 'Eta-squared'
		self.plotTitle = 'Eta-squared'
		self.bLogScale = False 

	def run(self, data):
		for group in data:
			if len(group) < 1:
				return -1.0
				
		totalSum = 0.0
		N = 0.0
		for i in xrange(0, len(data)):
			for x in data[i]:
				totalSum += x
				N += 1
		grandMean = totalSum / N

		totalSS = 0.0
		betweenGroupSS = 0.0
		for i in xrange(0, len(data)):
			groupSum = 0.0
			for x in data[i]:
				totalSS += (x - grandMean)*(x - grandMean)
				groupSum += x
			betweenGroupSS += (groupSum*groupSum) / len(data[i])
		betweenGroupSS -= (totalSum*totalSum) / N

		if totalSS != 0:
			etaSquared = betweenGroupSS / totalSS
		else:
			etaSquared = -1.0	# degenerate case where all samples have the same value
		
		return etaSquared
	
if __name__ == "__main__": 
	pass