#=======================================================================
# Author: Donovan Parks
#
# Abstract base class specifying interface for a post-hoc test.
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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.	See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.	If not, see <http://www.gnu.org/licenses/>.
#======================================================================='''

class AbstractPostHocTestPlugin:
	'''
	Abstract base class specifying interface for a post-hoc test
	'''
	def __init__(self, preferences):
		self.name = 'Unnamed'
	
	def run(self, data, coverage, groupNames):
		'''
		Must return the p-values, effect sizes, lower CIs, upper CIs, and labels for each contrast 
		  along with a note indicating additional information about the statistic 
		  (e.g., degenerate cases).
		'''
		pass
