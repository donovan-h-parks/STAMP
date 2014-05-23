#=======================================================================
# Author: Donovan Parks
#
# No correction.
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

class NoCorrection(AbstractMultCompCorrection):
  
  def __init__(self, preferences):
    AbstractMultCompCorrection.__init__(self, preferences)
    self.name = 'No correction'
    self.method = 'Per comparison error rate'
    self.bCorrectedValues = True
    self.numSignFeatures = 0
    
  def correct(self, pValues, alpha):
    self.numSignFeatures = len([x for x in pValues if x <= alpha])
    return pValues
 
  def additionalInfo(self):
    return [['Number of significant features', self.numSignFeatures]]
  
if __name__ == "__main__": 
  pass