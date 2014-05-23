#=======================================================================
# Author: Donovan Parks
#
# Ratio of proportions effect size measure.
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

class RatioProportions(AbstractGroupEffectSizePlugin):
	
	def __init__(self, preferences):
		AbstractGroupEffectSizePlugin.__init__(self, preferences)
		self.name = 'Ratio of proportions'
		self.plotTitle = 'Ratio of proportions'
		self.bLogScale = True 
		
	def run(self, propGroup1, propGroup2):
		meanG1 = float(sum(propGroup1)) / len(propGroup1)
		meanG2 = float(sum(propGroup2)) / len(propGroup2)
		if meanG1 == 0 or meanG2 == 0:
			pseudocount = self.preferences['Pseudocount'] / (meanG1 + meanG2)
			meanG1 += pseudocount
			meanG2 += pseudocount
			
		return meanG1 / meanG2
	
if __name__ == "__main__": 
	pass