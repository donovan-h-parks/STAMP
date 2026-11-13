#=======================================================================
# Author: Donovan Parks
#
# Abstract base class specifying interface of a two sample statistical hypothesis test.
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

class AbstractSampleStatsTestPlugin:
  '''
  Abstract base class specifying interface of a two sample statistical hypothesis test.
  '''
  def __init__(self, preferences):
    self.preferences = preferences
    self.name = 'Unnamed'
  
  def hypothesisTest(self, seq1, seq2, totalSeq1, totalSeq2):
    '''
    Must return the one-sided and two-sided p-values for the hypothesis test along
      with a note indicating any information about the resulting p-values.
    '''
    pass
  
  def power(self, seq1, seq2, totalSeq1, totalSeq2, alpha):
    '''
    Power of the statistical test. Return an empty list if the power cannot be calculated.
    '''
    return 'N/A'

  
  def equalSampleSize(self,seq1, seq2, totalSeq1, totalSeq2, alpha, beta):
    '''
    Equal sample size required to achieve a given power. Return an empty list if the power cannot be calculated.
    ''' 
    return 'N/A'