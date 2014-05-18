#=======================================================================
# Author: Donovan Parks
#
# Provides a command-line interface to STAMP.
#
# Example usage:
#   python STAMP.py --typeOfTest "Multiple groups" --profile ./examples/EnterotypesArumugam/Enterotypes.profile.spf --metadata ./examples/EnterotypesArumugam/Enterotypes.metadata.tsv --field Gender --statTest "ANOVA"
#   python STAMP.py --typeOfTest "Two groups" --profile ./examples/EnterotypesArumugam/Enterotypes.profile.spf --metadata ./examples/EnterotypesArumugam/Enterotypes.metadata.tsv --field Gender --name1 F --name2 M --statTest "t-test (equal variance)" --CI "DP: t-test inverted"
#   python STAMP.py --typeOfTest "Two samples" --profile ./examples/IronMineEdwards/EdwardsIronMine.spf  --name1 Red --name2 Black --statTest "Fisher's exact test" --CI "DP: Newcombe-Wilson"
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

import sys, os
from optparse import OptionParser

from metagenomics.fileIO.StampIO import StampIO
from metagenomics.fileIO.MetadataIO import MetadataIO

from metagenomics.stats.SampleStatsTests import SampleStatsTests
from metagenomics.stats.GroupStatsTests import GroupStatsTests
from metagenomics.stats.MultiGroupStatsTests import MultiGroupStatsTests
from metagenomics.TableHelper import SortTableStrCol

from plugins.PluginManager import PluginManager

from metagenomics.DirectoryHelper import getMainDir

class CommandLineParser():
	def __init__(self, preferences):
		self.preferences = preferences

		# load statistical technique plugins
		pluginManager = PluginManager(self.preferences)
		self.multCompDict = pluginManager.loadPlugins('plugins/common/multipleComparisonCorrections/')
		
		self.sampleEffectSizeDict = pluginManager.loadPlugins('plugins/samples/effectSizeFilters/')
		self.sampleStatTestDict = pluginManager.loadPlugins('plugins/samples/statisticalTests/')
		self.sampleConfIntervMethodDict = pluginManager.loadPlugins('plugins/samples/confidenceIntervalMethods/')
		
		self.groupEffectSizeDict = pluginManager.loadPlugins('plugins/groups/effectSizeFilters/')
		self.groupStatTestDict = pluginManager.loadPlugins('plugins/groups/statisticalTests/')
		
		self.multiGroupEffectSizeDict = pluginManager.loadPlugins('plugins/multiGroups/effectSizeFilters/')
		self.multiGroupStatTestDict = pluginManager.loadPlugins('plugins/multiGroups/statisticalTests/')
		self.multiGroupPostHoc = pluginManager.loadPlugins('plugins/multiGroups/postHoc/')

	def run(self):
		# *** Parse command line
		parser = OptionParser(usage='%prog [--typeOfTest] [--profile] [--statTest] [--CI]', version='STAMP command line interface v2.0.0')

		# profile properties
		parser.add_option('', '--typeOfTest', action='store', type='string', dest='typeOfTest', default='Two samples', help='Either Multiple groups, Two groups, or Two samples')
		
		parser.add_option('', '--profile', action='store', type='string', dest='profileFile', default='', help='STAMP profile to process')
		parser.add_option('', '--metadata', action='store', type='string', dest='metadataFile', default='', help='Group metadata file')
		parser.add_option('', '--field', action='store', type='string', dest='metadataField', default='', help='Metadata field used to define groups')
		
		parser.add_option('', '--name1', action='store', type='string', dest='name1', default='', help='Name of sample 1 or group 1')
		parser.add_option('', '--name2', action='store', type='string', dest='name2', default='', help='Name of sample 2 or group 2')
		parser.add_option('', '--profLevel', action='store', type='string', dest='profLevel', default='[Lowest level in hierarchy]', help='Hierarchical level to perform statistical analysis upon')
		parser.add_option('', '--parentLevel', action='store', type='string', dest='parentLevel', default='Entire sample', help='Parental level used to calculate relative proportions')
		parser.add_option('', '--unclassifiedTreatment', action='store', type='string', dest='unclassifiedTreatment', default='Retain unclassified reads', help='Specify treatment of unclassified fragments')

		# statistical testing properties
		parser.add_option('', '--statTest', action='store', type='string', dest='statTest', default='', help='Statistical hypothesis test to use')
		parser.add_option('', '--sided', action='store', type='string', dest='sided', default='Two-sided', help="Perform either a One-sided or Two-sided test")
		parser.add_option('', '--CI', action='store', type='string', dest='ciMethod', default='', help='Confidence interval method to use')
		parser.add_option('', '--effectSizeMeasure', action='store', type='string', dest='effectSizeMeasure', default='Eta-squared', help='Effect size measure used by multiple groups test')
		parser.add_option('', '--coverage', action='store', type='float', dest='coverage', default='0.95', help='Coverage of confidence interval')
		parser.add_option('', '--multComp', action='store', type='string', dest='multComp', default='No correction', help='Multiple comparison method to use')

		# filtering properties
		parser.add_option('', '--pValueFilter', action='store', type='float', dest='pValueFilter', default=0.05, help='Filter out features with a p-value greater than this value')

		parser.add_option('', '--seqFilter', action='store', type='string', dest='seqFilter', default='', help='Filter to apply to counts in profile level')
		parser.add_option('', '--sample1Filter', action='store', type='int', dest='sample1Filter', default=0, help='Filter criteria for sample 1')
		parser.add_option('', '--sample2Filter', action='store', type='int', dest='sample2Filter', default=0, help='Filter criteria for sample 2')

		parser.add_option('', '--parentSeqFilter', action='store', type='string', dest='parentSeqFilter', default='', help='Filter to apply to counts in parent level')
		parser.add_option('', '--parentSample1Filter', action='store', type='int', dest='parentSample1Filter', default=0, help='Filter to apply to counts in parent level')
		parser.add_option('', '--parentSample2Filter', action='store', type='int', dest='parentSample2Filter', default=0, help='Filter to apply to counts in parent level')

		parser.add_option('', '--effectSizeMeasure1Filter', action='store', type='string', dest='effectSizeMeasure1Filter', default='', help='Effect size measure to use')
		parser.add_option('', '--minEffectSize1Filter', action='store', type='float', dest='minEffectSize1Filter', default=0, help='Minimum required effect size')

		parser.add_option('', '--effectSizeMeasure2Filter', action='store', type='string', dest='effectSizeMeasure2Filter', default='', help='Effect size measure to use')
		parser.add_option('', '--minEffectSize2Filter', action='store', type='float', dest='minEffectSize2Filter', default=0, help='Minimum required effect size')

		parser.add_option('', '--effectSizeOperator', action='store', type='int', dest='effectSizeOperator', default=0, help='Logical operator to apply to effect size filters (0 - OR, 1 - AND)')

		# output properties
		parser.add_option('', '--outputTable', action='store', type='string', dest='tableFile', default='results.tsv', help='Filename for table')

		# misc. properties
		parser.add_option('', '--verbose', action='store', type='int', dest='verbose', default=1, help='Print progress information (1) or suppress all output (0)')

		(options, args) = parser.parse_args()

		if options.profileFile == '':
			parser.error('A STAMP profile must be specified (--profile).')
			sys.exit()
			

		if options.typeOfTest != "Two samples" and options.typeOfTest != "Two groups" and options.typeOfTest != "Multiple groups":
			parser.error('Valid values for --typeOfTest are "Multiple groups", "Two groups", and "Two samples".')
			sys.exit()

		if options.typeOfTest == "Two samples" or options.typeOfTest == "Two groups":
			if options.name1 == '' or options.name2 == '':
				parser.error('Sample/group names must be specified (--name1 and --name2).')
				sys.exit()
			
		if options.typeOfTest == "Two groups" or options.typeOfTest == "Multiple groups":
			if options.metadataFile == '':
				parser.error('Must specify a metadata file (--metadata).')
				sys.exit()
				
			if options.metadataField == '':
				parser.error('Must specified a metadata field which will be used to define groups (--field)')
				sys.exit()

		if not (options.sided == '' or options.sided == 'Two-sided' or options.sided == 'One-sided'):
			parser.error('Valid values for --sided are \'Two-sided\' and \'One-sided\'.')
			sys.exit()

		self.bVerbose = (options.verbose != 0)

		# *** Load profile file and create desired profile
		if self.bVerbose:
			print 'Creating desired profile.'
		profile = self.createProfile(options.typeOfTest, options.profileFile, options.metadataFile, options.metadataField, 
																	options.name1, options.name2, options.parentLevel, options.profLevel, 
																	options.unclassifiedTreatment)

		# *** Run statistical test		
		if self.bVerbose:
			print 'Performing statistical analysis.'

		statsTestResults = self.runStatTest(profile, options.typeOfTest, options.statTest, options.sided, options.ciMethod, options.coverage, options.multComp, options.effectSizeMeasure)

		# *** Filter features
		if self.bVerbose:
			print 'Filtering features.'

		self.filterFeatures(statsTestResults, options.typeOfTest, options.pValueFilter, options.seqFilter, options.sample1Filter, options.sample2Filter,
														options.parentSeqFilter, options.parentSample1Filter, options.parentSample2Filter,
														options.effectSizeMeasure1Filter, options.minEffectSize1Filter, options.effectSizeOperator,
														options.effectSizeMeasure2Filter, options.minEffectSize2Filter)

		if self.bVerbose:
			print 'Active features: ' + str(len(statsTestResults.getActiveFeatures()))

		# *** Create output table
		if self.bVerbose:
			print 'Saving results to ' + options.tableFile + '.'

		self.saveSummaryTable(options.tableFile, statsTestResults)

		if self.bVerbose:
			print 'Done.'

	def createProfile(self, typeOfTest, profileFile, metadataFile, metadataField, name1, name2, parentLevel, profLevel, unclassifiedTreatment):
		# load profile tree from file
		try:
			stampIO = StampIO(self.preferences)
			profileTree, errMsg = stampIO.read(profileFile)

			if errMsg != None:
				print errMsg
				sys.exit()
		except:
			print 'Unknown error while reading STAMP profile: ' + profileFile
			sys.exit()
			
		# load metadata file from file
		if typeOfTest == "Two groups" or typeOfTest == "Multiple groups":
			try:
				metadataIO = MetadataIO(self.preferences)
				metadata, warningMsg = metadataIO.read(metadataFile, profileTree)
				metadata.setActiveField(metadataField, profileTree)
					
				if warningMsg != None:
					print warningMsg
					sys.exit()
			except:
				print 'Unknown error while reading metadata file: ' + metadataFile
				sys.exit()

		# setup desired level in hierarchy
		if profLevel == '[Lowest level in hierarchy]':
			profLevel = profileTree.hierarchyHeadings[-1]

		# create profile for desired samples at the desired hierarchical levels
		errMsg = ''
		if parentLevel != 'Entire sample' and parentLevel not in profileTree.hierarchyHeadings:
			errMsg = 'Hierarchical level ' + '\'' + parentLevel + '\'' + ' could not be found in the input file.'
		elif profLevel not in profileTree.hierarchyHeadings:
			errMsg = 'Hierarchical level ' + '\'' + profLevel + '\'' + ' could not be found in the input file.'

		depthTest = list(['Entire sample'] + profileTree.hierarchyHeadings)
		if depthTest.index(parentLevel) >= depthTest.index(profLevel):
			errMsg = 'Specified parent level is at the same level or lower than the specified profile level.'

		if errMsg != '':
			print errMsg
			sys.exit()

		# create profile
		if typeOfTest == "Two samples":
			profile = profileTree.createSampleProfile(name1, name2, parentLevel, profLevel, unclassifiedTreatment)
		elif typeOfTest == "Two groups":
			profile = profileTree.createGroupProfile(name1, name2, parentLevel, profLevel, metadata, unclassifiedTreatment)
		elif typeOfTest == "Multiple groups":
			profile = profileTree.createMultiGroupProfile(profileTree.groupDict.keys(), parentLevel, profLevel, metadata, unclassifiedTreatment)

		return profile

	def runStatTest(self, profile, typeOfTest, statTest, sided, ciMethod, coverage, multComp, effectSizeMeasure):
		if typeOfTest == "Two samples":
			test = self.sampleStatTestDict[statTest]
			multCompMethod = self.multCompDict[multComp]
			confIntervMethod = self.sampleConfIntervMethodDict[ciMethod]

			statsTest = SampleStatsTests(self.preferences)
			progressIndicator = 'Verbose'
			if self.bVerbose == False:
				progressIndicator = None
			statsTest.run(test, sided, confIntervMethod, coverage, profile, progressIndicator)
			statsTest.results.performMultCompCorrection(multCompMethod)
			statsTest.results.selectAllFeautres()
			
		elif typeOfTest == "Two groups":
			test = self.groupStatTestDict[statTest]
			multCompMethod = self.multCompDict[multComp]

			statsTest = GroupStatsTests(self.preferences)
			progressIndicator = 'Verbose'
			if self.bVerbose == False:
				progressIndicator = None
			statsTest.run(test, sided, ciMethod, coverage, profile, progressIndicator)
			statsTest.results.performMultCompCorrection(multCompMethod)
			statsTest.results.selectAllFeautres()
		
		elif typeOfTest == 'Multiple groups':
			test = self.multiGroupStatTestDict[statTest]
			multCompMethod = self.multCompDict[multComp]
			esMeasure = self.multiGroupEffectSizeDict[effectSizeMeasure]
			
			statsTest = MultiGroupStatsTests(self.preferences)
			progressIndicator = 'Verbose'
			if self.bVerbose == False:
				progressIndicator = None
			statsTest.run(test, esMeasure, profile, progressIndicator)
			statsTest.results.performMultCompCorrection(multCompMethod)
			statsTest.results.selectAllFeautres()
			
		return statsTest.results
		
	def filterFeatures(self, statsTestResults, typeOfTest, signLevelFilter, seqFilter, sample1Filter, sample2Filter,
											parentSeqFilter, parentSample1Filter, parentSample2Filter,
											effectSizeMeasure1Filter, minEffectSize1Filter, effectSizeOperator,
											effectSizeMeasure2Filter, minEffectSize2Filter):
		# perform filtering
		if signLevelFilter >= 1.0:
			signLevelFilter = None

		# perform filtering
		if seqFilter == '':
			seqFilter = None
			sample1Filter = None
			sample2Filter = None

		if parentSeqFilter == '':
			parentSeqFilter = None
			parentSample1Filter = None
			parentSample2Filter = None

		if effectSizeOperator == 0:
			effectSizeOperator = 'OR'
		else:
			effectSizeOperator = 'AND'

		if typeOfTest == 'Two samples':
			# effect size filters
			if effectSizeMeasure1Filter != '':
				effectSizeMeasure1Filter = self.sampleEffectSizeDict[effectSizeMeasure1Filter]
			else:
				effectSizeMeasure1Filter = None
				minEffectSize1Filter = None

			if effectSizeMeasure2Filter != '':
				effectSizeMeasure2Filter = self.sampleEffectSizeDict[effectSizeMeasure2Filter]
			else:
				effectSizeMeasure2Filter = None
				minEffectSize2Filter = None
			
			statsTestResults.filterFeatures(signLevelFilter, seqFilter, sample1Filter, sample2Filter,
											parentSeqFilter, parentSample1Filter, parentSample2Filter,
											effectSizeMeasure1Filter, minEffectSize1Filter, effectSizeOperator,
											effectSizeMeasure2Filter, minEffectSize2Filter)
		elif typeOfTest == 'Two groups':
			# effect size filters
			if effectSizeMeasure1Filter != '':
				effectSizeMeasure1Filter = self.groupEffectSizeDict[effectSizeMeasure1Filter]
			else:
				effectSizeMeasure1Filter = None
				minEffectSize1Filter = None

			if effectSizeMeasure2Filter != '':
				effectSizeMeasure2Filter = self.groupEffectSizeDict[effectSizeMeasure2Filter]
			else:
				effectSizeMeasure2Filter = None
				minEffectSize2Filter = None
				
			statsTestResults.filterFeatures(signLevelFilter, seqFilter, sample1Filter, sample2Filter,
											parentSeqFilter, parentSample1Filter, parentSample2Filter,
											effectSizeMeasure1Filter, minEffectSize1Filter, effectSizeOperator,
											effectSizeMeasure2Filter, minEffectSize2Filter)
		elif typeOfTest == 'Multiple groups':
			# effect size filters
			if effectSizeMeasure1Filter != '':
				effectSizeMeasure1Filter = self.multiGroupEffectSizeDict[effectSizeMeasure1Filter]
			else:
				effectSizeMeasure1Filter = None
				minEffectSize1Filter = None
				
			statsTestResults.filterFeatures(signLevelFilter, effectSizeMeasure1Filter, minEffectSize1Filter)

	def saveSummaryTable(self, filename, statsTestResults):
		tableData, tableHeadings = statsTestResults.tableData(True)

		tableData = SortTableStrCol(tableData, 0)

		fout = open(filename, 'w')
		for heading in tableHeadings:
			fout.write(heading + '\t')
		fout.write('\n')

		for row in tableData:
			for entry in row:
				fout.write(str(entry) + '\t')
			fout.write('\n')

		fout.close()

if __name__ == "__main__":
	# change the current working directory
	os.chdir(getMainDir())

	# initialize preferences
	preferences = {}
	preferences['Pseudocount'] = 0.5
	preferences['Executable directory'] = sys.path[0]

	commandLineParser = CommandLineParser(preferences)
	commandLineParser.run()
	sys.exit()

