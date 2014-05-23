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

from stamp.plugins.groups.AbstractGroupEffectSizePlugin import AbstractGroupEffectSizePlugin

class DiffBetweenProp(AbstractGroupEffectSizePlugin):
	
	def __init__(self, preferences):
		AbstractGroupEffectSizePlugin.__init__(self, preferences)
		self.name = 'Difference between proportions'
		self.plotTitle = 'Difference between proportions (%)'
		self.bLogScale = False 

	def run(self, propGroup1, propGroup2):
		if len(propGroup1) > 0:
			meanG1 = float(sum(propGroup1)) / len(propGroup1)
		else:
			meanG1 = 0
			
		if len(propGroup2) > 0:
			meanG2 = float(sum(propGroup2)) / len(propGroup2)
		else:
			meanG2 = 0
			
		dp = meanG1 - meanG2
		return dp
	
if __name__ == "__main__": 
	pass