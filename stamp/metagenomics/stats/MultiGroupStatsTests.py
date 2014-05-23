#=======================================================================
# Author: Donovan Parks
#
# Perform statistical tests for multiple groups.
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

from stamp.metagenomics import TableHelper

from numpy import mean, std

class PostHocResults:
	def __init__(self):
		self.feature = ''
		self.alpha = 0
		self.labels = []
		self.pValues = []
		self.effectSizes = []
		self.lowerCIs = []
		self.upperCIs = []
		self.note = ''


class MultiGroupStatTestResults(object):
	'''
	Results of performing a statistical test.
	'''
	def __init__(self, preferences):
		self.data = []
		self.activeData = []
		self.activeFeatures = []
		self.selectedFeatures = []
		self.profile = None
		
		self.dataHeadings = {}
		self.dataHeadings['Features'] = 0
		self.dataHeadings['pValues'] = 1
		self.dataHeadings['pValuesCorrected'] = 2
		self.dataHeadings['EffectSize'] = 3
		self.dataHeadings['Note'] = 4

		self.test = None
		self.multCompCorrection = None
		self.multCompCorrectionInfo = ''
		self.postHocResults = PostHocResults()
		
		self.tableHeadings = []
		
		self.preferences = preferences
	
	def oneMinusAlpha(self):
		return 1.0 - self.alpha
		
	def getColumn(self, columnName, bActiveFeatures = True):
		columnData = []
		index = self.dataHeadings[columnName]
		
		if bActiveFeatures:
			data = self.activeData
		else:
			data = self.data
		
		for row in data:
			columnData.append(row[index])
			
		return columnData
		
	def getColumnAsFloatStr(self, columnName, bActiveFeatures = True):
		columnData = []
		index = self.dataHeadings[columnName]
		
		if bActiveFeatures:
			data = self.activeData
		else:
			data = self.data
		
		for row in data:
			columnData.append('%.3f' % row[index] )
			
		return columnData

	def getColumnAsStr(self, columnName, bActiveFeatures = True):
		columnData = []
		index = self.dataHeadings[columnName]
		
		if bActiveFeatures:
			data = self.activeData
		else:
			data = self.data
			
		for row in data:
			value = row[index]	 
			if value < 0.01:
				valueStr = '%.2e' % row[index]
				if row[index] < 10**self.preferences['Minimum reported p-value exponent']:
					valueStr = '< 1e' + str(int(self.preferences['Minimum reported p-value exponent']))
				elif 'e-00' in valueStr:
					valueStr = valueStr.replace('e-00', 'e-')
				elif 'e-0' in valueStr:
					valueStr = valueStr.replace('e-0', 'e-')
			else:
				valueStr = '%.3f' % row[index] 
				
			columnData.append(valueStr)
				
		return columnData
		
	def getPValueStr(self, pValue):
		valueStr = ''
		if pValue < 0.01:
			valueStr = '%.2e' % pValue
			if valueStr < 10**self.preferences['Minimum reported p-value exponent']:
					valueStr = '< 1e' + str(int(self.preferences['Minimum reported p-value exponent']))
			elif 'e-00' in valueStr:
				valueStr = valueStr.replace('e-00', 'e-')
			elif 'e-0' in valueStr:
				valueStr = valueStr.replace('e-0', 'e-')
		else:
			valueStr = '%.3f' % pValue
				
		return valueStr
		
	def getFeatureStatisticAsStr(self, feature, columnName):
		valueStr = ''
		featureIndex = self.dataHeadings['Features']
		index = self.dataHeadings[columnName]

		for row in self.data:
			if row[featureIndex] == feature:
				value = row[index]
				if value < 0.01:
					valueStr = '%.2e' % row[index]
					if row[index] < 10**self.preferences['Minimum reported p-value exponent']:
						valueStr = '< 1e' + str(int(self.preferences['Minimum reported p-value exponent']))
					elif 'e-00' in valueStr:
						valueStr = valueStr.replace('e-00', 'e-')
					elif 'e-0' in valueStr:
						valueStr = valueStr.replace('e-0', 'e-')
				else:
					valueStr = '%.3f' % row[index] 
					
				break
				
		return valueStr
	
	def signFeatures(self):
		signData = [row for row in self.data if row[self.dataHeadings['pValuesCorrected']] < self.alpha]
		return signData

	def tableData(self, bActiveFeaturesOnly = False):
		# get table data
		data = []
		
		featureIndex = self.dataHeadings['Features']
		activeFeatures = self.getActiveFeatures()
		for row in self.data:
			if bActiveFeaturesOnly and (row[featureIndex] not in activeFeatures):
				continue
			
			newRow = list(self.profile.getHierarchy(row[0]))
			newRow += row[1:]
			data.append(newRow)
			
		return data, self.tableHeadings
	
	def createTableHeadings(self, groupNames, headingsSampleStats):
		self.tableHeadings = list(self.profile.hierarchyHeadings)
		self.tableHeadings += ['p-values','p-values (corrected)']
		self.tableHeadings += ['Effect size']
		self.tableHeadings += ['Note']
		
		for groupName in groupNames:
			self.tableHeadings += [groupName + ': mean rel. freq. (%)', groupName + ': std. dev. (%)']
			
		self.tableHeadings += headingsSampleStats
		
	def getDataFromTable(self, feature, columnName):
		featureIndex = self.dataHeadings['Features']
		dataIndex = self.tableHeadings.index(columnName) - len(self.profile.hierarchyHeadings) + 1
		
		for row in self.data:
			if row[featureIndex] == feature:
				return row[dataIndex]
				
		return 0.0
	
	def performMultCompCorrection(self, multCompCorrection): 
		self.multCompCorrection = multCompCorrection
		
		index = self.dataHeadings['pValuesCorrected']
		
		pValues = self.getColumn('pValues', False)
		pValuesCorrected = multCompCorrection.correct(pValues, 0.05)
		
		for i in xrange(0, len(self.data)):
			self.data[i][index] = pValuesCorrected[i]
			
		self.multCompCorrectionInfo = multCompCorrection.additionalInfo()
			
	def filterFeatures(self, signLevelFilter, effectSizeMeasure, minEffectSize):
		
		self.activeData = []

		for row in self.data:
			feature = row[self.dataHeadings['Features']]
			pValue = row[self.dataHeadings['pValuesCorrected']]
			effectSize = row[self.dataHeadings['EffectSize']]
			
			# feature must be in the selected list
			if feature not in self.selectedFeatures:
				continue
			
			# p-value filter
			if signLevelFilter != None and pValue > signLevelFilter:
				continue
				
			if effectSizeMeasure != None and effectSize < minEffectSize:
				continue
				
			self.activeData.append(row)
						
		self.activeFeatures = self.getColumn('Features', True)
	
	def getActiveFeatures(self):
		return self.activeFeatures
		
	def getSelectedFeatures(self):
		return self.selectedFeatures
								
	def setSelectedFeatures(self, selectedFeatures):
		self.selectedFeatures = selectedFeatures
		
	def selectAllFeautres(self):
		self.selectedFeatures = self.getColumn('Features', False)
	
class MultiGroupStatsTests(object):
	'''
	Perform statistical tests.
	'''
	
	def __init__(self, preferences):
		self.results = MultiGroupStatTestResults(preferences)
		self.preferences = preferences
		 
	def run(self, statTest, effectSizeMeasure, profile, progress = None):
		self.results.data = []
		self.results.test = statTest.name
		self.results.profile = profile
		
		if progress == 'Verbose':
			print '  Processing feature:'
		 
		index = 0
		for feature in profile.getFeatures():
			if progress == 'Verbose':
					print '    ' + feature
			elif progress != None:
				if progress.wasCanceled():
					self.results.data = []
					return

				index += 1
				progress.setValue(index)
								
			seqCount = profile.getActiveFeatureCounts(feature)
			parentCount = profile.getActiveParentCounts(feature)
			data = profile.getActiveFeatureProportions(feature)
			pValue, note = statTest.hypothesisTest(data)
			effectSize = effectSizeMeasure.run(data)
 
			row = [feature, float(pValue), float(pValue), effectSize, note]
			
			for i in xrange(0, len(seqCount)):
				propGroup = []
				for j in xrange(0, len(seqCount[i])):
					sc = seqCount[i][j]
					pc = parentCount[i][j]
					if pc > 0:
						propGroup.append( sc * 100.0 / pc)
					else:
						propGroup.append( 0.0 )
					
				row.append(float(mean(propGroup)))
				row.append(float(std(propGroup)))
			
			for i in xrange(0, len(seqCount)):
				for j in xrange(0, len(seqCount[i])):
					sc = seqCount[i][j]
					pc = parentCount[i][j]
					row.append(sc)
					row.append(pc)
					if pc > 0:
						row.append(sc * 100.0 / pc)
					else:
						row.append( 0.0 )
					
			self.results.data.append(row)

		headingsSampleStats = []
		for i in xrange(0, len(profile.activeSamplesInGroups)):
			for sampleName in profile.activeSamplesInGroups[i]:
				headingsSampleStats.append(sampleName)
				headingsSampleStats.append(sampleName + ': parent seq. count')
				headingsSampleStats.append(sampleName + ': rel. freq. (%)')
			
		self.results.createTableHeadings(profile.activeGroupNames, headingsSampleStats)
			
		if len(self.results.data) >= 1:
			# sort results according to p-values
			self.results.data = TableHelper.SortTable(self.results.data, [self.results.dataHeadings['pValues']])
			
		if progress != None and progress != 'Verbose':
			index += 1
			progress.setValue(index)
			
	def runPostHocTest(self, postHocTest, profile, selectedFeature, coverage):
		if selectedFeature == '':
			self.results.postHocResults = PostHocResults()
			return
			
		self.results.postHocTest = postHocTest.name
		
		data = profile.getActiveFeatureProportions(selectedFeature)
		pValues, effectSize, lowerCI, upperCI, labels, note = postHocTest.run(data, coverage, profile.activeGroupNames)
		
		self.results.postHocResults.feature = selectedFeature
		self.results.postHocResults.alpha = 1.0 - coverage
		self.results.postHocResults.labels = labels
		self.results.postHocResults.pValues = pValues
		self.results.postHocResults.effectSizes = effectSize
		self.results.postHocResults.lowerCIs = lowerCI
		self.results.postHocResults.upperCIs = upperCI
		self.results.postHocResults.note = note




