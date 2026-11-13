#=======================================================================
# Author: Donovan Parks
#
# Perform statistical tests for two samples.
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

from stamp.metagenomics import TableHelper

class SampleStatTestResults(object):
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
		self.dataHeadings['Seq1'] = 1
		self.dataHeadings['Seq2'] = 2
		self.dataHeadings['ParentalSeq1'] = 3
		self.dataHeadings['ParentalSeq2'] = 4
		self.dataHeadings['RelFreq1'] = 5
		self.dataHeadings['RelFreq2'] = 6
		self.dataHeadings['pValues'] = 7
		self.dataHeadings['pValuesCorrected'] = 8
		self.dataHeadings['EffectSize'] = 9
		self.dataHeadings['LowerCI'] = 10
		self.dataHeadings['UpperCI'] = 11
		self.dataHeadings['Note'] = 12

		self.alpha = 0
		self.beta = 0.2
		
		self.test = None
		self.testType = None
		self.confIntervMethod = None
		self.multCompCorrection = None
		self.multCompCorrectionInfo = ''
		
		self.preferences = preferences
			
	def oneMinusAlpha(self):
		return 1.0 - self.alpha
	
	def oneMinusBeta(self):
		return 1.0 - self.beta
					
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
		
	def getFeatureStatistic(self, feature, columnName):
		featureIndex = self.dataHeadings['Features']
		index = self.dataHeadings[columnName]

		for row in self.data:
			if row[featureIndex] == feature:
				value = row[index]
				break
				
		return value
	
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
		
	def createTableHeadings(self, sampleName1, sampleName2):
		oneMinAlphaStr = str(self.oneMinusAlpha()*100)
		
		self.tableHeadings = list(self.profile.hierarchyHeadings)
		self.tableHeadings += [sampleName1, sampleName2]
		self.tableHeadings += [sampleName1 + ': parent seq. count', sampleName2 + ': parent seq. count']
		self.tableHeadings += [sampleName1 + ': rel. freq. (%)', sampleName2 + ': rel. freq. (%)']
		self.tableHeadings += ['p-values','p-values (corrected)']
		self.tableHeadings += ['Effect size']
		self.tableHeadings += [oneMinAlphaStr + '% lower CI']
		self.tableHeadings += [oneMinAlphaStr + '% upper CI']
		self.tableHeadings += ['Note']
	
	def contingencyTable(self, bActiveFeaturesOnly = False):
		if self.profile == None:
			return
		
		tables = []
		for feature in self.profile.getFeatures():
			if bActiveFeaturesOnly and (feature not in self.getActiveFeatures()):
				continue
			
			tables.append([feature] + self.profile.getTableData(feature))
			
		return tables
	
	def performMultCompCorrection(self, multCompCorrection): 
		self.multCompCorrection = multCompCorrection
		
		index = self.dataHeadings['pValuesCorrected']
		
		pValues = self.getColumn('pValues', False)
		pValuesCorrected = multCompCorrection.correct(pValues, self.alpha)
		
		for i in xrange(0, len(self.data)):
			self.data[i][index] = pValuesCorrected[i]
			
		self.multCompCorrectionInfo = multCompCorrection.additionalInfo()
	
	def filterFeatures(self, signLevelFilter, 
													seqFilter, sample1Filter, sample2Filter,
													parentSeqFilter, parentSample1Filter, parentSample2Filter,
													effectSizeMeasure1, minEffectSize1, effectSizeOperator,
													effectSizeMeasure2, minEffectSize2):
		
		self.activeData = []

		for row in self.data:
			feature = row[self.dataHeadings['Features']]
			pValue = row[self.dataHeadings['pValuesCorrected']]
			seq1, seq2, parentSeq1, parentSeq2 = self.profile.getTableData(feature)
			
			# feature must be in the selected list
			if feature not in self.selectedFeatures:
				continue
			
			# p-value filter
			if signLevelFilter != None and pValue > signLevelFilter:
				continue
			
			# sequence filter
			if seqFilter == 'maximum':
				bPassSeqFilter = (max(seq1, seq2) >= sample1Filter)
			elif seqFilter == 'minimum':
				bPassSeqFilter = (min(seq1, seq2) >= sample1Filter)
			elif seqFilter == 'independent':
				bPassSeqFilter = (seq1 >= sample1Filter and seq2 >= sample2Filter)
			else:
				bPassSeqFilter = True # sequence filter is disabled
				
			if not bPassSeqFilter:
				continue
				
			# parent sequence filter
			if parentSeqFilter == 'maximum':
				bPassParentSeqFilter = (max(parentSeq1, parentSeq2) >= parentSample1Filter)
			elif parentSeqFilter == 'minimum':
				bPassParentSeqFilter = (min(parentSeq1, parentSeq2) >= parentSample1Filter)
			elif parentSeqFilter == 'independent':
				bPassParentSeqFilter = (parentSeq1 >= parentSample1Filter and parentSeq2 >= parentSample2Filter)
			else:
				bPassParentSeqFilter = True	# parent sequence filter is disabled
				
			if not bPassParentSeqFilter:
				continue
			
			# effect size filters
			if effectSizeMeasure1 != None:
				effectSizeA = effectSizeMeasure1.run(seq1, seq2, parentSeq1, parentSeq2)
				effectSizeB = effectSizeMeasure1.run(seq2, seq1, parentSeq2, parentSeq1)
				effectSize1 = max(effectSizeA, effectSizeB)
				
			if effectSizeMeasure2 != None:
				effectSizeA = effectSizeMeasure2.run(seq1, seq2, parentSeq1, parentSeq2)
				effectSizeB = effectSizeMeasure2.run(seq2, seq1, parentSeq2, parentSeq1)
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
	
class SampleStatsTests(object):
	'''
	Perform statistical tests.
	'''
	
	def __init__(self, preferences):
		self.results = SampleStatTestResults(preferences)
		 
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
		for feature in profile.getFeatures():
			if progress == 'Verbose':
				print '    ' + feature
			elif progress != None:
				if progress.wasCanceled():
					self.results.data = []
					return

				index += 1
				progress.setValue(index)

			seq1, seq2, parentSeq1, parentSeq2 = profile.getTableData(feature)

			# Difference between proportions test
			pValueOneSided, pValueTwoSided, note = statTest.hypothesisTest(seq1, seq2, parentSeq1, parentSeq2)
			
			if testType == 'One-sided':
				pValue = pValueOneSided
			elif testType == 'Two-sided':
				pValue = pValueTwoSided
			else:
				print 'Error: Unknown test type.'
			
			# Confidence interval
			lowerCI, upperCI, effectSize, ciNote = confIntervMethod.run(seq1, seq2, parentSeq1, parentSeq2, coverage)
			
			if ciNote != '':
				if note != '':
					note += '; ' + ciNote
				else:
					note = ciNote
 
			self.results.data.append([feature,seq1,seq2,parentSeq1,parentSeq2,
															(float(seq1))/max(parentSeq1,1) * 100, (float(seq2))/max(parentSeq2,1) * 100,\
															float(pValue),float(pValue),\
															float(effectSize),float(lowerCI),float(upperCI), note])
															
		self.results.createTableHeadings(profile.sampleNames[0], profile.sampleNames[1])
										
		if len(self.results.data) >= 1:
			# sort results according to p-values
			self.results.data = TableHelper.SortTable(self.results.data, [self.results.dataHeadings['pValues']])
		
		if progress != None and progress != 'Verbose':
			index += 1
			progress.setValue(index)


