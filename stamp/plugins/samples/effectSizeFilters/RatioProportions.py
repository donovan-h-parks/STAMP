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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.  If not, see <http://www.gnu.org/licenses/>.
#=======================================================================

from stamp.plugins.samples.AbstractSampleEffectSizePlugin import AbstractSampleEffectSizePlugin

class RatioProportions(AbstractSampleEffectSizePlugin):
  
  def __init__(self, preferences):
    AbstractSampleEffectSizePlugin.__init__(self, preferences)
    self.name = 'Ratio of proportions'
    self.plotTitle = 'Ratio of proportions'
    self.bLogScale = True 
    
  def run(self, seq1, seq2, totalSeq1, totalSeq2):  
    if seq1 == 0 or seq2 == 0:
      pseudocount = self.preferences['Pseudocount']
      seq1 += pseudocount
      seq2 += pseudocount
      totalSeq1 += pseudocount
      totalSeq2 += pseudocount
      
    return (float(seq1) / totalSeq1) / (float(seq2) / totalSeq2)
  
if __name__ == "__main__": 
  pass