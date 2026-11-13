#=======================================================================
# Author: Donovan Parks
#
# Perform Chi-square statistical hypothesis test.
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

from stamp.plugins.samples.AbstractSampleStatsTestPlugin import AbstractSampleStatsTestPlugin
from stamp.metagenomics.stats.distributions.NormalDist import standardNormalCDF, zScore

from scipy.stats import chi2

class ChiSquare(AbstractSampleStatsTestPlugin):
  '''
 Perform Chi-square statistical hypothesis test
  '''
  
  def __init__(self, preferences):
    AbstractSampleStatsTestPlugin.__init__(self, preferences)
    self.name = 'Chi-square test'
  
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
  
    X2 = (abs(x1 - E00))**2 / E00
    X2 += (abs(x2 - E01))**2 / E01
    X2 += (abs(y1 - E10))**2 / E10
    X2 += (abs(y2 - E11))**2 / E11
    
    # calculate p-value
    pValueTwoSided = 1.0 - chi2.cdf(X2,1)
  
    return float('inf'), pValueTwoSided, ''
  
  def power(self, seq1, seq2, totalSeq1, totalSeq2, alpha): 
    # The chi-square test is equivalent to the difference between proportions
    # test as illustrated by Rivals et al., 2007. Here we use the standard
    # asymptotic power formulation for a difference between proportions test.
    oneMinusAlpha = 1.0 - alpha
     
    p1 = float(seq1) / totalSeq1
    p2 = float(seq2) / totalSeq2
    d = p1 - p2

    stdDev = math.sqrt( (p1 * (1-p1)) / totalSeq1 + (p2 * (1 - p2)) / totalSeq2 )
    
    if stdDev != 0:    
      p = float(totalSeq1*p1 + totalSeq2*p2) / (totalSeq1 + totalSeq2)
      q = 1-p
      pooledStdDev = math.sqrt( (p*q) / totalSeq1 + (p*q) / totalSeq2 )
      
      zScore = zScore(oneMinusAlpha)
      zLower = ( -zScore * pooledStdDev - d ) / stdDev
      zUpper= ( zScore * pooledStdDev - d ) / stdDev
    
      return standardNormalCDF(zLower) + (1.0 - standardNormalCDF(zUpper))
    else:
      return 1.0
  
  
  def equalSampleSize(self, seq1, seq2, totalSeq1, totalSeq2, alpha, beta):
    # The chi-square test is equivalent to the difference between proportions
    # test as illustrated by Rivals et al., 2007. Here we use the standard
    # equal sample size formulation for a difference between proportions test.
    oneMinusAlpha = 1.0 - alpha
    oneMinusBeta = 1.0 - beta
    
    p1 = float(seq1) / totalSeq1
    p2 = float(seq2) / totalSeq2
    q1 = 1.0 - p1
    q2 = 1.0 - p2
    d = p1 - p2
    
    if d == 0:
      return 1  

    return (zScore(oneMinusAlpha) * math.sqrt((p1 + p2)*(q1 + q2)/2) + zScore(oneMinusBeta)*math.sqrt((p1*q1) + (p2*q2)))**2 / (d**2)

if __name__ == "__main__": 
  chiSquare = ChiSquare()
  pValueOne, pValueTwo = chiSquare.hypothesisTest(10, 20, 60, 50)
  print pValueOne
  print pValueTwo
