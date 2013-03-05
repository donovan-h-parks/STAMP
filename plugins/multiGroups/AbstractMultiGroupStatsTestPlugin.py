#=======================================================================
# Author: Donovan Parks
#
# Abstract base class specifying interface of a multiple group statistical hypothesis test.
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

class AbstractMultiGroupStatsTestPlugin:
	'''
	Abstract base class specifying interface of a multiple group statistical hypothesis test.
	'''
	def __init__(self, preferences):
		self.name = 'Unnamed'
	
	def hypothesisTest(self, data):
		'''
		Must return the p-values for the hypothesis test
		  and a note indicating any information about the 
		  resulting test.
		'''
		pass
