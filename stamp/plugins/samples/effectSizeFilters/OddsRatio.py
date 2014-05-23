#=======================================================================
# Author: Donovan Parks
#
# Odds ratio effect size measure.
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

class OddsRatio(AbstractSampleEffectSizePlugin):
  
  def __init__(self, preferences):
    AbstractSampleEffectSizePlugin.__init__(self, preferences)
    self.name = 'Odds ratio'
    self.plotTitle = 'Odds ratio'
    self.bLogScale = True 

  def run(self, seq1, seq2, totalSeq1, totalSeq2):  
    a = seq1
    b = seq2
    c = totalSeq1 - seq1
    d = totalSeq2 - seq2
    
    if a == 0 or b == 0 or c == 0 or d == 0:
      pseudocount = self.preferences['Pseudocount']
      a += pseudocount
      b += pseudocount
      c += pseudocount
      d += pseudocount

    return (float(a) * d) / (b * c)
  
if __name__ == "__main__": 
  pass