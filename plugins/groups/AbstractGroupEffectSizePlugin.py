#=======================================================================
# Author: Donovan Parks
#
# Abstract base class specifying an effect size filter.
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

class AbstractGroupEffectSizePlugin:
  '''
  Abstract base class specifying an effect size filter.
  '''
  def __init__(self, preferences):
    self.preferences = preferences # dictionary indicating user-defined preferences
    self.name = 'Unnamed'         # name of filter
    self.plotTitle = 'Untitled'   # title to use in plots
    self.bLogScale = False        # indicate if effect size is returned in log space

  
  def run(self, propGroup1, propGroup2):
    '''
    Must return the effect size.
    '''
    pass
