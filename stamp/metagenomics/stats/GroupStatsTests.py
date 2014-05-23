#=======================================================================
# Author: Donovan Parks
#
# Perform statistical tests for two groups.
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

class GroupStatTestResults(object):
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
		self.dataHeadings['MeanRelFreq1'] = 1
		self.dataHeadings['StdDevRelFreq1'] = 2
		self.dataHeadings['MeanRelFreq2'] = 3
		self.dataHeadings['StdDevRelFreq2'] = 4
		self.dataHeadings['pValues'] = 5
		self.dataHeadings['pValuesCorrected'] = 6
		self.dataHeadings['EffectSize'] = 7
		self.dataHeadings['LowerCI'] = 8
		self.dataHeadings['UpperCI'] = 9
		self.dataHeadings['Note'] = 10
		
		self.alpha = 0

		self.test = None
		self.testType = None
		self.confIntervMethod = None
		self.bConfIntervRatio = False
		self.multCompCorrection = None
		self.multCompCorrectionInfo = ''
		
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
				valueStr = '%.2e' % value
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
		
	def getFeatureStatisticAsStr(self, feature, columnName):
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
		
	def createTableHeadings(self, groupName1, groupName2, headingsSampleStats):
		oneMinAlphaStr = str(self.oneMinusAlpha()*100)

		self.tableHeadings = list(self.profile.hierarchyHeadings)
		self.tableHeadings += [groupName1 + ': mean rel. freq. (%)', groupName1 + ': std. dev. (%)']
		self.tableHeadings += [groupName2 + ': mean rel. freq. (%)', groupName2 + ': std. dev. (%)']
		self.tableHeadings += ['p-values','p-values (corrected)']
		self.tableHeadings += ['Difference between means']
		self.tableHeadings += [oneMinAlphaStr + '% lower CI']
		self.tableHeadings += [oneMinAlphaStr + '% upper CI']
		self.tableHeadings += ['Note']
		self.tableHeadings += headingsSampleStats
	
	def performMultCompCorrection(self, multCompCorrection): 
		self.multCompCorrection = multCompCorrection
		
		index = self.dataHeadings['pValuesCorrected']
		
		pValues = self.getColumn('pValues', False)
		pValuesCorrected = multCompCorrection.correct(pValues, self.alpha)
		
		for i in xrange(0, len(self.data)):
			self.data[i][index] = pValuesCorrected[i]
			
		self.multCompCorrectionInfo = multCompCorrection.additionalInfo()
			
	def filterFeatures(self, signLevelFilter, 
													seqFilter, group1Filter, group2Filter,
													parentSeqFilter, parentGroup1Filter, parentGroup2Filter,
													effectSizeMeasure1, minEffectSize1, effectSizeOperator,
													effectSizeMeasure2, minEffectSize2):
		
		self.activeData = []

		for row in self.data:
			feature = row[self.dataHeadings['Features']]
			pValue = row[self.dataHeadings['pValuesCorrected']]
			seqCountGroup1, seqCountGroup2 = self.profile.getFeatureCounts(feature)
			parentSeqCountGroup1, parentSeqCountGroup2 = self.profile.getParentFeatureCounts(feature)
			
			# feature must be in the selected list
			if feature not in self.selectedFeatures:
				continue
			
			# p-value filter
			if signLevelFilter != None and pValue > signLevelFilter:
				continue
			
			# sequence filter
			if seqFilter == 'maximum':
				bPassSeqFilter = (max(max(seqCountGroup1), max(seqCountGroup2)) >= group1Filter)
			elif seqFilter == 'minimum':
				bPassSeqFilter = (min(min(seqCountGroup1), min(seqCountGroup2)) >= group1Filter)
			elif seqFilter == 'independent, minimum':
				bPassSeqFilter = (min(seqCountGroup1) >= group1Filter and min(seqCountGroup2) >= group2Filter)
			elif seqFilter == 'independent, maximum':
				bPassSeqFilter = (max(seqCountGroup1) >= group1Filter and max(seqCountGroup2) >= group2Filter)
			else:
				bPassSeqFilter = True					 # sequence filter is disabled
				
			if not bPassSeqFilter:
				continue
				
			# parent sequence filter
			if parentSeqFilter == 'maximum':
				bPassParentSeqFilter = (max(max(parentSeqCountGroup1), max(parentSeqCountGroup2))  >= parentGroup1Filter)
			elif parentSeqFilter == 'minimum':
				bPassParentSeqFilter = (min(min(parentSeqCountGroup1), min(parentSeqCountGroup2))  >= parentGroup1Filter)
			elif parentSeqFilter == 'independent, minimum':
				bPassParentSeqFilter = (min(parentSeqCountGroup1) >= parentGroup1Filter and min(parentSeqCountGroup2) >= parentGroup2Filter)
			elif parentSeqFilter == 'independent, maximum':
				bPassParentSeqFilter = (max(parentSeqCountGroup1) >= parentGroup1Filter and max(parentSeqCountGroup2) >= parentGroup2Filter)
			else:
				bPassParentSeqFilter = True # parent sequence filter is disabled
				
			if not bPassParentSeqFilter:
				continue
			
			# effect size filters
			propGroup1, propGroup2 = self.profile.getFeatureProportions(feature)
			if effectSizeMeasure1 != None:
				effectSizeA = effectSizeMeasure1.run(propGroup1, propGroup2)
				effectSizeB = effectSizeMeasure1.run(propGroup2, propGroup1)
				effectSize1 = max(effectSizeA, effectSizeB)
				
			if effectSizeMeasure2 != None:
				effectSizeA = effectSizeMeasure2.run(propGroup1, propGroup2)
				effectSizeB = effectSizeMeasure2.run(propGroup2, propGroup1)
				effectSize2 = max(effectSizeA, effectSizeB)
				
			if effectSizeMeasure1 == None and effectSizeMeasure2 == None:
				self.activeData.append(row) 
			elif effectSizeMeasure1 != None and effectSizeMeasure2 == None:
				if effectSize1 >= minEffectSize1:
					self.activeData.append(row) 
			elif effectSizeMeasure1 == None and effectSizeMeasure2 != None:
				if effectSize2 >= minEffectSize2:
					self.activeData.append(row) 
			else:
				if effectSizeOperator == 'AND':
					if effectSize1 >= minEffectSize1 or effectSize2 >= minEffectSize2:
						self.activeData.append(row) 
				elif effectSizeOperator == 'OR':
					if effectSize1 >= minEffectSize1 and effectSize2 >= minEffectSize2:
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
	
class GroupStatsTests(object):
	'''
	Perform statistical tests.
	'''
	
	def __init__(self, preferences):
		self.results = GroupStatTestResults(preferences)
		 
	def run(self, statTest, testType, confIntervMethod, coverage, profile, progress = None):
		self.results.test = statTest.name
		self.results.testType = testType
		self.results.alpha = 1.0 - coverage
		self.results.confIntervMethod = confIntervMethod
		self.results.profile = profile
		
		if progress == 'Verbose':
			print '  Processing feature:'
		 
		self.results.data = []
		index = 0
		
		# calculate statistics
		seqsGroup1 = []
		seqsGroup2 = []
		parentSeqsGroup1 = []
		parentSeqsGroup2 = []
		pValues = []
		lowerCIs = []
		upperCIs = []
		effectSizes = []
		notes = []
		if statTest.bSingleFeatureInterface:
			# process features one at a time
			for feature in profile.getFeatures():
				if progress == 'Verbose':
					print '    ' + feature
				elif progress != None:
					if progress.wasCanceled():
						self.results.data = []
						return

					index += 1
					progress.setValue(index)
															
				# get statistics
				seqGroup1, seqGroup2 = profile.getFeatureCounts(feature)
				parentSeqGroup1, parentSeqGroup2= profile.getParentFeatureCounts(feature)
				results = statTest.run(seqGroup1, seqGroup2, parentSeqGroup1, parentSeqGroup2, confIntervMethod, coverage)
				pValueOneSided, pValueTwoSided, lowerCI, upperCI, effectSize, note = results
				
				if testType == 'One-sided':
					pValue = pValueOneSided
				elif testType == 'Two-sided':
					pValue = pValueTwoSided
				else:
					print 'Error: Unknown test type.'

				# record results
				seqsGroup1.append(seqGroup1)
				seqsGroup2.append(seqGroup2)
				parentSeqsGroup1.append(parentSeqGroup1)
				parentSeqsGroup2.append(parentSeqGroup2)
				pValues.append(pValue)
				lowerCIs.append(lowerCI)
				upperCIs.append(upperCI)
				effectSizes.append(effectSize)
				notes.append(note)
				
			if progress != None and progress != 'Verbose':
				index += 1
				progress.setValue(index)
		else:
			# process all features at once
			seqsGroup1, seqsGroup2 = profile.getFeatureCountsAll()
			parentSeqsGroup1, parentSeqsGroup2= profile.getParentFeatureCountsAll()
			pValuesOneSided, pValuesTwoSided, lowerCIs, upperCIs, effectSizes, notes = statTest.runAll(seqsGroup1, seqsGroup2, parentSeqsGroup1, parentSeqsGroup2, confIntervMethod, coverage, progress)
			if progress == 'Verbose':
				print '    Processing all features...'
			elif progress != None and progress.wasCanceled():
				self.results.data = []
				return

			if testType == 'One-sided':
				pValues = pValuesOneSided
			elif testType == 'Two-sided':
				pValues = pValuesTwoSided
			else:
				print 'Error: Unknown test type.'
				
		# record statistics
		features = profile.getFeatures()
		for i in xrange(0, len(features)):
			propGroup1 = []
			for j in xrange(0, len(seqsGroup1[i])):
				sg1 = seqsGroup1[i][j]
				psg1 = parentSeqsGroup1[i][j]
				
				if psg1 > 0:
					propGroup1.append( sg1 * 100.0 / psg1 )
				else:
					propGroup1.append( 0.0 )
			
			propGroup2 = []
			for j in xrange(0, len(seqsGroup2[i])):
				sg2 = seqsGroup2[i][j]
				psg2 = parentSeqsGroup2[i][j]
				
				if psg2 > 0:
					propGroup2.append( sg2 * 100.0 / psg2 )
				else:
					propGroup2.append( 0.0 )
			
			row = [features[i], float(mean(propGroup1)), float(std(propGroup1)), 
							float(mean(propGroup2)), float(std(propGroup2)),
							float(pValues[i]),float(pValues[i]),float(effectSizes[i]),
							float(lowerCIs[i]),float(upperCIs[i]), notes[i]]
							
			for j in xrange(0, len(seqsGroup1[i])):
				row.append(seqsGroup1[i][j])
				row.append(parentSeqsGroup1[i][j])
				if parentSeqsGroup1[i][j] > 0:
					row.append(seqsGroup1[i][j] * 100.0 / parentSeqsGroup1[i][j])
				else:
					row.append(0.0)
				
			for j in xrange(0, len(seqsGroup2[i])):
				row.append(seqsGroup2[i][j])
				row.append(parentSeqsGroup2[i][j])
				if parentSeqsGroup2[i][j] > 0:
					row.append(seqsGroup2[i][j] * 100.0 / parentSeqsGroup2[i][j])
				else:
					row.append(0.0)
																
			self.results.data.append(row)
																
		headingsSampleStats = []
		for sampleName in (profile.samplesInGroup1 + profile.samplesInGroup2):
			headingsSampleStats.append(sampleName)
			headingsSampleStats.append(sampleName + ': parent seq. count')
			headingsSampleStats.append(sampleName + ': rel. freq. (%)')
			
		self.results.createTableHeadings(profile.groupName1, profile.groupName2, headingsSampleStats)
		
		# sort results according to p-values
		if len(self.results.data) >= 1:
			self.results.data = TableHelper.SortTable(self.results.data, [self.results.dataHeadings['pValues']])
