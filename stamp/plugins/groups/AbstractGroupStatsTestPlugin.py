#=======================================================================
# Author: Donovan Parks
#
# Abstract base class specifying interface of a two group statistical hypothesis test.
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
#=======================================================================

class AbstractGroupStatsTestPlugin:
	'''
	Abstract base class specifying interface of a statistical hypothesis test.
	'''
	def __init__(self, preferences):
		self.preferences = preferences
		self.name = 'Unnamed'		# name of test
		self.confIntervMethods = []	# list of CI methods supported by this test
		
		self.bSingleFeatureInterface = True
	
	def run(self, seqGroup1, seqGroup2, parentSeqGroup1, parentSeqGroup2, confIntervMethod, coverage):
		'''
		Process a single feature at a time.
		
		Return the one-sided p-value, two-sided p-value, lower CI, upper CI, effect size, 
		  and a note indicating any information about the resulting statistics.
		'''
		pass
		
	def runAll(self, seqGroup1, seqGroup2, parentSeqGroup1, parentSeqGroup2, confIntervMethod, coverage, progress = None):
		'''
		Process all features simultaneously.
		
		Return lists indicating, for each feature, the one-sided p-value, two-sided p-value, lower CI, upper CI, effect size, 
		  and a note indicating any information about the resulting statistics.
		'''