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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.  If not, see <http://www.gnu.org/licenses/>.
#=======================================================================

from stamp.plugins.samples.AbstractSampleEffectSizePlugin import AbstractSampleEffectSizePlugin

class DiffBetweenProp(AbstractSampleEffectSizePlugin):
  
  def __init__(self, preferences):
    AbstractSampleEffectSizePlugin.__init__(self, preferences)
    self.name = 'Difference between proportions'
    self.plotTitle = 'Difference between proportions (%)'
    self.bLogScale = False 

  def run(self, seq1, seq2, totalSeq1, totalSeq2):  
    p1 = float(seq1)/ max(totalSeq1, 1)
    p2 = float(seq2)/ max(totalSeq2, 1)
    return ( p1 - p2 ) * 100
  
if __name__ == "__main__": 
  pass