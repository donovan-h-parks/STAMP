#=======================================================================
# Author: Donovan Parks
#
# Benjamini-Hochberg false discovery rate method.
#
# Benjamini Y and Hochberg R. (1995) Controlling the False Discovery 
# Rate: a Practical and Powerful Approach to Multiple Testing. Journal 
# of the Royal Statistical Society (Series B), Vol. 57, No. 1, pp. 289-300. 
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

'''

'''

from stamp.plugins.common.AbstractMultCompCorrection import AbstractMultCompCorrection

class BenjaminiHochbergFDR(AbstractMultCompCorrection):
  
  def __init__(self, preferences):
    AbstractMultCompCorrection.__init__(self, preferences)
    self.name = 'Benjamini-Hochberg FDR'
    self.method = 'False discovery rate'
    self.bCorrectedValues = True
    self.numSignFeatures = 0
    
  def correct(self, pValues, alpha):
    # append an index identifier to each p-value
    indexedList = []
    index = 0
    for value in pValues:
      indexedList.append([value, index])
      index += 1
      
    # sort p-values in descending order
    indexedList.sort(reverse = True)
    
    # determine significant features
    numComparisons = len(pValues)
    modifier = numComparisons
    self.numSignFeatures = 0
    for i in xrange(0, len(indexedList)):
      index = indexedList[i][1]     
      
      pValues[index] = pValues[index] * numComparisons / float(modifier)
      if pValues[index] < alpha:
        self.numSignFeatures += 1
        
      modifier -= 1
  
    return pValues
  
  def additionalInfo(self):
    return [['Number of significant features', self.numSignFeatures]]
  
if __name__ == "__main__": 
  pass