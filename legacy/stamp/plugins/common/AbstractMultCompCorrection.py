#=======================================================================
# Author: Donovan Parks
#
# Abstract base class specifying interface of a multiple comparison correction method.
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

class AbstractMultCompCorrection:
  '''
  Abstract base class specifying interface of a multiple comparison correction method.
  '''
  def __init__(self, preferences):
    self.name = 'Unnamed'
    self.method = 'Not specified' # should be set to 'Familywise error rate' or 'False discovery rate'
    self.bCorrectedValues = False # indicates if a method produces a list of corrected values (True) or
                                  # only a set of significant features (False)
                                  
  def correct(self, pValues, alpha):
    '''
    Must return the corrected p-values.
    '''
    pass
  
  def additionalInfo(self):
    '''
    Return any additional information regarding a multiple comparison method. Information must
     be returned as a list of lists specifying the name and value of each additional piece of information.
     
      e.g., info = [['Label 1', 7],['Label 2', 0.33],['Label 3','Yes']]
    '''
    return []