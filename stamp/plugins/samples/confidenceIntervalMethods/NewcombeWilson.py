#=======================================================================
# Author: Donovan Parks
#
# Confidence interval method proposed by R. G. Newcombe in "Interval estimation for the difference
#	between independent propotions: comparison of eleven methods", 1997.
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

import math

from stamp.plugins.samples.AbstractSampleConfIntervMethod import AbstractSampleConfIntervMethod

from stamp.metagenomics.stats.distributions.NormalDist import zScore

class NewcombeWilson(AbstractSampleConfIntervMethod):
	
	def __init__(self, preferences):
		AbstractSampleConfIntervMethod.__init__(self, preferences)
		self.name = 'DP: Newcombe-Wilson'
		self.plotLabel = 'Difference between proportions (%)'
		self.bRatio = False
	
	def NewcombeWilsonFindRoots(self, seq, totalSeq, z):
		'''
		Find roots required by Newcombe-Wilson CI method
		'''
		value = 0.0
		stepSize = 1.0 / max(totalSeq,1000)
		steps = int(1.0 / stepSize)
		prevP = z*math.sqrt(value*(1.0-value) / totalSeq) - abs(value - float(seq) / totalSeq)
		prevValue = value
		roots = []
		for dummy in xrange(0,steps):
			p = z*math.sqrt(value*(1.0-value) / totalSeq) - abs(value - float(seq) / totalSeq)
			if p*prevP < 0 or (p == 0 and value == 0) or (p == 0 and value == 1.0):
				# we have found a root since there is a sign change
				if abs(p)+abs(prevP) != 0:
					root = prevValue + stepSize*(1.0 - (abs(p)/(abs(p)+abs(prevP))))
				else:
					root = prevValue
				roots.append(root)
				
				if len(roots) == 2:
					break
			
			prevP = p
			prevValue = value	
			value += stepSize
		
		# check if we have a double root
		if len(roots) == 1:
			roots.append(roots[0])
		
		return roots
	
	def run(self, seq1, seq2, totalSeq1, totalSeq2, coverage):
		'''
		Calculate confidence interval using Newcombe-Wilson method.
			Results are report as percent difference.
		'''
		note = ''
		
		if totalSeq1 == 0:
			totalSeq1 = self.preferences['Pseudocount']
			note = 'degenerate case: CI calculation used pseudocount'
			
		if totalSeq2 == 0:
			totalSeq2 = self.preferences['Pseudocount']
			note = 'degenerate case: CI calculation used pseudocount'
		
		z = zScore(coverage)
		
		roots1 = self.NewcombeWilsonFindRoots(seq1, totalSeq1, z)
		roots2 = self.NewcombeWilsonFindRoots(seq2, totalSeq2, z)
	
		diff = float(seq1)/totalSeq1 - float(seq2)/totalSeq2
		lowerCI = z*math.sqrt(roots1[0]*(1-roots1[0])/totalSeq1 + roots2[1]*(1-roots2[1])/totalSeq2)
		upperCI = z*math.sqrt(roots1[1]*(1-roots1[1])/totalSeq1 + roots2[0]*(1-roots2[0])/totalSeq2)
		
		return (diff-lowerCI)*100, (diff+upperCI)*100, diff*100, note
	
if __name__ == "__main__": 
	pass