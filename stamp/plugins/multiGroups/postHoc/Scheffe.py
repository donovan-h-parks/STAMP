#=======================================================================# Author: Donovan Parks## Perform Scheffe post-hoc test.## Copyright 2011 Donovan Parks## This file is part of STAMP.## STAMP is free software: you can redistribute it and/or modify# it under the terms of the GNU General Public License as published by# the Free Software Foundation, either version 3 of the License, or# (at your option) any later version.## STAMP is distributed in the hope that it will be useful,# but WITHOUT ANY WARRANTY; without even the implied warranty of# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.	See the# GNU General Public License for more details.## You should have received a copy of the GNU General Public License# along with STAMP.	If not, see <http://www.gnu.org/licenses/>.#======================================================================='''import mathfrom stamp.plugins.multiGroups.AbstractPostHocTestPlugin import AbstractPostHocTestPluginfrom numpy import var, meanfrom scipy.stats import distributionsclass Scheffe(AbstractPostHocTestPlugin):	'''	Perform Scheffe post-hoc test.	'''		def __init__(self, preferences):		AbstractPostHocTestPlugin.__init__(self, preferences)		self.name = u'Scheff\xE8'		def run(self, data, coverage, groupNames):		note = ''				# calculate critical value		N = 0		for i in xrange(0, len(data)):			N += len(data[i])					dfN = len(data) - 1		dfD = N - len(data)				cv = dfN*distributions.f.ppf(coverage, dfN, dfD)		# calculate mean of each group		groupMean = []		for i in xrange(0, len(data)):			groupMean.append(mean(data[i]))				# calculate within group variance		withinGroupVar = 0		for i in xrange(0, len(data)):			withinGroupVar += (len(data[i])-1)*var(data[i], ddof=1)		withinGroupVar /= dfD		withinGroupStdDev = math.sqrt(withinGroupVar)				if withinGroupVar == 0:			note = 'degenerate case: within group variance is zero; set to 1e-6.'			withinGroupVar = 1e-6					# calculate Fs, effect size, and CI for each pair of groups		pValues = []		effectSize = []		lowerCI = []		upperCI = []		labels = []		for i in xrange(0, len(data)):			for j in xrange(i+1, len(data)):				es = groupMean[i] - groupMean[j]				effectSize.append(es)				invSampleSize = (1.0/len(data[i]) + 1.0/len(data[j]))				Fs = (es * es) / (withinGroupVar*invSampleSize)								pValue = 1.0 - distributions.f.cdf(Fs / dfN, dfN, dfD)				pValues.append(pValue)				# confidence interval				confInter = math.sqrt(cv*invSampleSize)*withinGroupStdDev				lowerCI.append(es - confInter)				upperCI.append(es + confInter)								labels.append(groupNames[i] + ' : ' + groupNames[j])					return pValues, effectSize, lowerCI, upperCI, labels, noteif __name__ == "__main__": 	pass