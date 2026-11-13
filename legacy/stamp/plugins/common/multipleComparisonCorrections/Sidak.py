#=======================================================================
# Author: Donovan Parks
#
# Sidak multiple comparison correction.
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

from stamp.plugins.common.AbstractMultCompCorrection import AbstractMultCompCorrection

class Sidak(AbstractMultCompCorrection):
  
  def __init__(self, preferences):
    AbstractMultCompCorrection.__init__(self, preferences)
    self.name = 'Sidak'
    self.method = 'Familywise error rate'
    self.bCorrectedValues = True
    self.numSignFeatures = 0
    
  def correct(self, pValues, alpha):
    self.alpha = alpha
    self.numComparisons = len(pValues)
    
    corrected = []
    for pValue in pValues:
      correctedValue = 1.0 - (1.0 - pValue)**self.numComparisons
      corrected.append(correctedValue)
      if correctedValue <= alpha:
        self.numSignFeatures += 1
      
    return corrected

  def additionalInfo(self):
    correctedErrorRate = 1.0 - (1.0 - self.alpha)**(1.0/self.numComparisons)
    return [['Number of significant features', self.numSignFeatures],
            ['Corrected error rate', correctedErrorRate]]
  
if __name__ == "__main__": 
  pass