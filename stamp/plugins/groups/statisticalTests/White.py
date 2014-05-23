#=======================================================================
# Author: Donovan Parks
#
# Perform non-parametric t-test proposed by JR White.
#
# White JR, Nagarajan N, Pop M. (2009). Statistical Methods for Detecting 
#   Differentially Abundant Features in Clinical Metagenomic Samples, 
#   PLoS Computational Biology, 5: e1000352.
#
# This implementation is based on the R implementation of JR White
#  (see below)
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

from stamp.metagenomics.Bootstrap import bootstrapDiffOfMeanProp
from stamp.plugins.groups.AbstractGroupStatsTestPlugin import AbstractGroupStatsTestPlugin

class White(AbstractGroupStatsTestPlugin):
	'''
	Perform White's non-parametric t-test
	'''
	
	def __init__(self, preferences):
		AbstractGroupStatsTestPlugin.__init__(self, preferences)
		
		self.name = "White's non-parametric t-test"
		self.confIntervMethods = ["DP: bootstrap"]
		
		self.preferences = preferences
		
		self.bSingleFeatureInterface = False
		
	def runAll(self, seqGroup1, seqGroup2, parentSeqGroup1, parentSeqGroup2, confIntervMethod, coverage, progress):
		return detect_differentially_abundant_features(seqGroup1, seqGroup2, parentSeqGroup1, parentSeqGroup2, coverage, self.preferences['Replicates'], self.preferences, progress)



#*****************************************************************************************************
#*****************************************************************************************************
#  Last modified: 4/14/2009 
#  
#  Author: james robert white, whitej@umd.edu, Center for Bioinformatics and Computational Biology.
#  University of Maryland - College Park, MD 20740
#
# Ported to Python and modified by Donovan Parks in April, 2011.
#
#*****************************************************************************************************
#*****************************************************************************************************

from stamp.plugins.samples.statisticalTests.Fishers import Fishers
from stamp.plugins.samples.confidenceIntervalMethods.DiffBetweenPropAsymptoticCC import DiffBetweenPropAsymptoticCC

from numpy import var

import random

# Perform non-parametric t-test proposed by White et al. (2009)
#  -> B is the number of permutations to use in estimating the null t-stat distribution.
def detect_differentially_abundant_features(seqGroup1, seqGroup2, parentSeqGroup1, parentSeqGroup2, coverage, B, preferences, progress):
	numFeatures = len(seqGroup1)   
	n1 = len(seqGroup1[0])
	n2 = len(seqGroup2[0])
	
	# convert to proportions
	propGroup1 = []
	for r in xrange(0, numFeatures):
		row = []
		for c in xrange(0, n1):
			row.append(float(seqGroup1[r][c]) / parentSeqGroup1[r][c])
		propGroup1.append(row)
			
	propGroup2 = []
	for r in xrange(0, numFeatures):
		row = []
		for c in xrange(0, n2):
			row.append(float(seqGroup2[r][c]) / parentSeqGroup2[r][c])
		propGroup2.append(row)

	# calculate t-statistics for unpooled variances for each feature
	T_statistics, effectSizes, notes = calc_twosample_ts(propGroup1, propGroup2)

	# generate statistics using non-parametric t-test based on permutations of the t-statistic
	pValuesOneSided, pValuesTwoSided, lowerCIs, upperCIs = permuted_statistics(propGroup1, propGroup2, seqGroup1, seqGroup2, T_statistics, coverage, B, progress)
	if progress != None and progress.wasCanceled():
		return [], [], [], [], [], []
	
	# generate p values for sparse data using fisher's exact test
	fishers = Fishers(preferences)
	diffBetweenProp = DiffBetweenPropAsymptoticCC(preferences)
	for r in xrange(0, numFeatures):
		if sum(seqGroup1[r]) < n1 and sum(seqGroup2[r]) < n2:
			p1, p2, note = fishers.hypothesisTest(sum(seqGroup1[r]), sum(seqGroup2[r]), sum(parentSeqGroup1[r]), sum(parentSeqGroup2[r]))
			l, u, es, note = diffBetweenProp.run(sum(seqGroup1[r]), sum(seqGroup2[r]), sum(parentSeqGroup1[r]), sum(parentSeqGroup2[r]), coverage)
			pValuesOneSided[r] = p1 
			pValuesTwoSided[r] = p2 
			lowerCIs[r] = l
			upperCIs[r] = u
			effectSizes[r] = es
			notes[r] = "heuristic: statistics calculated with Fisher's test"

	return pValuesOneSided, pValuesTwoSided, lowerCIs, upperCIs, effectSizes, notes 

# Function to calculate permuted pvalues from Storey and Tibshirani(2003)
def permuted_statistics(propGroup1, propGroup2, seqGroup1, seqGroup2, T_statistics, coverage, B, progress):
	n1 = len(seqGroup1[0])
	n2 = len(seqGroup2[0]) 
	numSamples = n1 + n2
	numFeatures = len(seqGroup1)
	progressIndex = 0
	
	if progress != None and progress != 'Verbose':
		progress.setMaximum(B + numFeatures)
		progress.setLabelText('Calculating null distribution...')
	
	# calculate null distribution of the t-statistics using B permutations
	permuted_ttests = []
	permuted_effectSizes = []
	for j in xrange(0, B):  
		if progress != None and progress != 'Verbose':
			progressIndex += 1
			progress.setValue(progressIndex)
			if progress.wasCanceled():
					return [], [], [], []
					
		trial_ts, trail_effectSizes, notes = permute_and_calc_ts(propGroup1, propGroup2, random.sample(xrange(numSamples), numSamples))
		permuted_ttests.append(trial_ts)
		
	if progress != None:
		progress.setLabelText('Calculating p-values...')

	# calculate each pvalue using the null ts
	pValuesOneSided = []
	pValuesTwoSided = []

	if n1 < 8 or n2 < 8:
		# pool just the frequently observed ts  
		cleanedpermuted_ttests = permuted_ttests
		highFreqIndices = []
		for r in xrange(0, numFeatures): 
			if sum(seqGroup1[r]) >= n1 or sum(seqGroup2[r]) >= n2:
				highFreqIndices.append(r)
			else:
				if progress != None and progress != 'Verbose':
					progressIndex += 1
					progress.setValue(progressIndex)
			  
		#now for each feature
		pValuesOneSided = [0.0] * numFeatures
		pValuesTwoSided = [0.0] * numFeatures
		lowerCIs = [0.0] * numFeatures
		upperCIs = [0.0] * numFeatures
		for hfIndex in highFreqIndices:
			if progress != None and progress != 'Verbose':
				progressIndex += 1
				progress.setValue(progressIndex)
				if progress.wasCanceled():
						return [], [], [], []
					
			oneTailed = 0
			twoTailed = 0
			for i in xrange(0, B):
				for hfIndex2 in highFreqIndices: 
					if cleanedpermuted_ttests[i][hfIndex2] > T_statistics[hfIndex]:
						oneTailed += 1
					if abs(cleanedpermuted_ttests[i][hfIndex2]) > abs(T_statistics[hfIndex]):
						twoTailed += 1

			pValuesOneSided[hfIndex] = (1.0/(B*len(highFreqIndices))) * oneTailed
			pValuesTwoSided[hfIndex] = (1.0/(B*len(highFreqIndices))) * twoTailed
	else:
		for r in xrange(0, numFeatures): 
			if progress != None and progress != 'Verbose':
				progressIndex += 1
				progress.setValue(progressIndex)
				if progress.wasCanceled():
						return [], [], [], []
					
			oneTailed = 0
			twoTailed = 0
			for i in xrange(0, B):
				if permuted_ttests[i][r] > T_statistics[r]:
					oneTailed += 1
				if abs(permuted_ttests[i][r]) > abs(T_statistics[r]):
					twoTailed += 1
					
			p1 = (1.0/(B+1)) * (oneTailed+1)
			pValuesOneSided.append(p1)
			
			p2 = (1.0/(B+1)) * (twoTailed+1)
			pValuesTwoSided.append(p2)
			
	# calculate difference in mean proportions confidence intervals using a bootstrapping procedure
	lowerCIs = []
	upperCIs = []
	for r in xrange(0, numFeatures): 
		lowerCI, upperCI = bootstrapDiffOfMeanProp(propGroup1[r], propGroup2[r], coverage, replicates = B)
		lowerCIs.append(lowerCI*100)
		upperCIs.append(upperCI*100)
			
	if progress != None and progress != 'Verbose':
		progressIndex += 1
		progress.setValue(progressIndex)

	return pValuesOneSided, pValuesTwoSided, lowerCIs, upperCIs

# Calculate t-statistic under different permutations of the samples.
def permute_and_calc_ts(propGroup1, propGroup2, permVec):
	n1 = len(propGroup1[0])
	n2 = len(propGroup2[0]) 
	numSamples = n1 + n2
	numFeatures = len(propGroup1) 
	
	# first permute the rows in the matrix
	permPropGroup1 = []
	permPropGroup2 = []
	for r in xrange(0, numFeatures):
		row = propGroup1[r] + propGroup2[r]
		
		group1 = []
		for i in xrange(0, n1):
			group1.append(row[permVec[i]])
		permPropGroup1.append(group1)
		
		group2 = []
		for i in xrange(0, n2):
			group2.append(row[permVec[i+n1]])
		permPropGroup2.append(group2)

	return calc_twosample_ts(permPropGroup1, permPropGroup2)
	
# Calc two sample two statistics
def calc_twosample_ts(propGroup1, propGroup2):
	n1 = len(propGroup1[0])
	n2 = len(propGroup2[0]) 
	numFeatures = len(propGroup1)
	
	T_statistics = []
	effectSizes = []
	notes = []
	for r in xrange(0, numFeatures):
		meanG1 = float(sum(propGroup1[r])) / n1
		varG1 = var(propGroup1[r], ddof=1)
		stdErrG1 = varG1 / n1

		meanG2 = float(sum(propGroup2[r])) / n2
		varG2 = var(propGroup2[r], ddof=1)
		stdErrG2 = varG2 / n2 
		
		dp = meanG1 - meanG2
		effectSizes.append(dp*100)
		
		denom = math.sqrt(stdErrG1 + stdErrG2)

		if denom == 0:
			notes.append('degenerate case: zero variance for both groups; variance set to 1e-6.')
			T_statistics.append(dp/1e-6) 
		else:
			notes.append('')
			T_statistics.append(dp/denom) 

	return T_statistics, effectSizes, notes
	
if __name__ == "__main__": 
	pass

