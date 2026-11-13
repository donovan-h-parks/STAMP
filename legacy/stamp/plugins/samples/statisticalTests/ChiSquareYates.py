#=======================================================================
# Author: Donovan Parks
#
# Perform Chi-square statistical hypothesis test (w/ Yates' correction)
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

from stamp.plugins.samples.AbstractSampleStatsTestPlugin import AbstractSampleStatsTestPlugin

from scipy.stats import chi2

class ChiSquareYates(AbstractSampleStatsTestPlugin):
  '''
 Perform Chi-square statistical hypothesis test (w/ Yates' correction)
  '''
  
  def __init__(self, preferences):
    AbstractSampleStatsTestPlugin.__init__(self, preferences)
    self.name = 'Chi-square test (w/ Yates\')'
  
  def hypothesisTest(self, seq1, seq2, totalSeq1, totalSeq2):
    # Contingency table:
    # x1 x2
    # y1 y2
    x1 = seq1
    x2 = seq2
    y1 = totalSeq1 - x1
    y2 = totalSeq2 - x2
    
    if (x1 == 0 and x2 == 0) or (x1 == totalSeq1 or x2 == totalSeq2):
      return float('inf'), 1.0, 'degenerate case: suspect p-value'
    
    N = x1+x2+y1+y2
    
    E00 = float((x1+x2) * (x1+y1)) / N
    E01 = float((x1+x2) * (x2+y2)) / N
    E10 = float((y1+y2) * (x1+y1)) / N
    E11 = float((y1+y2) * (x2+y2)) / N
  
    X2 = (abs(x1 - E00)-0.5)**2 / E00
    X2 += (abs(x2 - E01)-0.5)**2 / E01
    X2 += (abs(y1 - E10)-0.5)**2 / E10
    X2 += (abs(y2 - E11)-0.5)**2 / E11
    
    # calculate p-value
    pValueTwoSided = 1.0 - chi2.cdf(X2,1)
  
    return float('inf'), pValueTwoSided, ''

if __name__ == "__main__": 
  chiSquareYates = ChiSquareYates()
  pValueOne, pValueTwo = chiSquareYates.hypothesisTest(10, 20, 60, 50)
  print pValueOne
  print pValueTwo
