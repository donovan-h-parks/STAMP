#=======================================================================
# Author: Donovan Parks
#
# Unit tests for STAMP.
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

import unittest
import sys

# test tables (positive samples 1, positive samples 2, total samples 1, total samples 2)
table1 = [10, 8, 30, 40]
table2 = [4000, 5000, 500000, 1000000]

# preferences for statistical tests
preferences = {}
preferences['Pseudocount'] = 0.5
preferences['Executable directory'] = sys.path[0]
preferences['Replicates'] = 1000

class VerifyPostHocTests(unittest.TestCase):
	def testGamesHowell(self):
		"""Verify computation of Games-Howell post-hoc test"""
		from plugins.multiGroups.postHoc.GamesHowell import GamesHowell
		gh = GamesHowell(preferences)

		# ground truth found with SPSS v19. Values are not exact since the critical Q value 
		# are interpolated from tables in the STAMP implementation.
		pValues, effectSize, lowerCI, upperCI, labels, _ = gh.run([[1,2,3,4,5],[10,20,30,40,50,60],[1,2,3,4,5,6,7]], 0.95, ['1', '2', '3'])
		self.assertEqual(labels[0], '1 : 2')
		self.assertAlmostEqual(effectSize[0], -32)
		self.assertAlmostEqual(lowerCI[0], -56.836534205367272) # SPSS = -56.80902338101632
		self.assertAlmostEqual(upperCI[0],  -7.163465794632728) # SPSS = -7.190976618983683
		self.assertEqual(pValues[0] == '< 0.02', True) # SPSS = 0.019165308600281317
		
		self.assertEqual(labels[1], '1 : 3')
		self.assertAlmostEqual(effectSize[1], -1.0)
		self.assertAlmostEqual(lowerCI[1], -3.9627938823820417) # SPSS = -3.962591041989213
		self.assertAlmostEqual(upperCI[1],  1.9627938823820417) # SPSS = 1.9625910419892132
		self.assertEqual(pValues[1] == '>= 0.1', True) # SPSS = 0.6372223228477465
		
		self.assertEqual(labels[2], '2 : 3')
		self.assertAlmostEqual(effectSize[2], 31)
		self.assertAlmostEqual(lowerCI[2], 6.1693311597445302) # SPSS = 6.2047330662731035
		self.assertAlmostEqual(upperCI[2], 55.83066884025547) # SPSS = 55.79526693372689
		self.assertEqual(pValues[2] == '< 0.05', True) # SPSS = 0.021640761239221984
		
	def testTukeyKramer(self):
		"""Verify computation of Tukey-Kramer post-hoc test"""
		from plugins.multiGroups.postHoc.TukeyKramer import TukeyKramer
		tk = TukeyKramer(preferences)
		
		# ground truth found with the anova1 and multcompare function in MATLAB v7.10.0 and SPSS v19
		pValues, effectSize, lowerCI, upperCI, labels, _ = tk.run([[1,2,3,4,5],[10,20,30,40,50,60],[1,2,3,4,5,6,7]], 0.95, ['1', '2', '3'])
		self.assertEqual(labels[0], '1 : 2')
		self.assertAlmostEqual(effectSize[0], -32)
		self.assertAlmostEqual(lowerCI[0], -49.172140035619407)
		self.assertAlmostEqual(upperCI[0],  -14.827859964380597)
		self.assertEqual(pValues[0] == '< 0.001', True) # 5.960611653675896E-4
		
		self.assertEqual(labels[1], '1 : 3')
		self.assertAlmostEqual(effectSize[1], -1.0)
		self.assertAlmostEqual(lowerCI[1], -17.605245738594071)
		self.assertAlmostEqual(upperCI[1],  15.605245738594071)
		self.assertEqual(pValues[1] == '>= 0.1', True) # 0.9866130284213506
		
		self.assertEqual(labels[2], '2 : 3')
		self.assertAlmostEqual(effectSize[2], 31)
		self.assertAlmostEqual(lowerCI[2], 15.222589067602378)
		self.assertAlmostEqual(upperCI[2], 46.777410932397622)
		self.assertEqual(pValues[2] == '< 0.001', True) # 3.593658536739097E-4

	def testScheffe(self):
		"""Verify computation of Scheffe post-hoc test"""
		from plugins.multiGroups.postHoc.Scheffe import Scheffe
		scheffe = Scheffe(preferences)
		
		# ground truth example taken from http://www.mathcs.duq.edu/larget/math225/notes18.html
		data = []
		data.append([19.65,20.05,20.65,20.85,21.65,21.65,21.65,21.85,21.85,21.85,22.05,22.05,22.05,22.05,22.05,22.05,22.05,22.05,22.05,22.05,22.25,22.25,22.25,22.25,22.25,22.25,22.25,22.25,22.45,22.45,22.45,22.65,22.65,22.85,22.85,22.85,22.85,23.05,23.25,23.25,23.45,23.65,23.85,24.25,24.45])
		data.append([21.05,21.85,22.05,22.45,22.65,23.25,23.25,23.25,23.45,23.45,23.65,23.85,24.05,24.05,24.05])
		data.append([20.85,21.65,22.05,22.85,23.05,23.05,23.05,23.05,23.45,23.85,23.85,23.85,24.05,25.05])
		data.append([21.05,21.85,22.05,22.05,22.05,22.25,22.45,22.45,22.65,23.05,23.05,23.05,23.05,23.05,23.25,23.85])
		data.append([21.05,21.85,21.85,21.85,22.05,22.45,22.65,23.05,23.05,23.25,23.45,24.05,24.05,24.05,24.85])
		data.append([19.85,20.05,20.25,20.85,20.85,20.85,21.05,21.05,21.05,21.25,21.45,22.05,22.05,22.05,22.25])
		
		pValues, effectSize, lowerCI, upperCI, labels, _ = scheffe.run(data, 0.95, ['MeadowPipet', 'TreePipet', 'Sparrow', 'Robin', 'PiedWagtail', 'Wren'])
		
		self.assertEqual(labels[9], 'Sparrow : Robin')
		self.assertAlmostEqual(effectSize[9], 0.546428571)
		self.assertAlmostEqual(lowerCI[9], -0.58049475277666573)
		self.assertAlmostEqual(upperCI[9], 1.6733518956338074)
		self.assertEqual(pValues[9] > 0.05, True)
		
		self.assertEqual(labels[11], 'Sparrow : Wren')
		self.assertAlmostEqual(effectSize[11], 1.9914285714285711)
		self.assertAlmostEqual(lowerCI[11], 0.84710959211483861)
		self.assertAlmostEqual(upperCI[11], 3.1357475507423036)
		self.assertEqual(pValues[11] < 0.05, True)
		
		# ground truth found with the anova1 and multcompare function in MATLAB v7.10.0 and SPSS v19
		pValues, effectSize, lowerCI, upperCI, labels, _ = scheffe.run([[1,2,3,4,5],[10,20,30,40,50,60],[1,2,3,4,5,6,7]], 0.95, ['1', '2', '3'])
		self.assertEqual(labels[0], '1 : 2')
		self.assertAlmostEqual(effectSize[0], -32)
		self.assertAlmostEqual(lowerCI[0], -49.941123031784372)
		self.assertAlmostEqual(upperCI[0],  -14.058876968215628)
		self.assertAlmostEqual(pValues[0], 8.624781311637033E-4)
		
		self.assertEqual(labels[1], '1 : 3')
		self.assertAlmostEqual(effectSize[1], -1.0)
		self.assertAlmostEqual(lowerCI[1], -18.348842727299797)
		self.assertAlmostEqual(upperCI[1],  16.348842727299797)
		self.assertAlmostEqual(pValues[1], 0.9878500418301395)
		
		self.assertEqual(labels[2], '2 : 3')
		self.assertAlmostEqual(effectSize[2], 31)
		self.assertAlmostEqual(lowerCI[2], 14.51606322368572)
		self.assertAlmostEqual(upperCI[2], 47.48393677631428)
		self.assertAlmostEqual(pValues[2], 5.261333896968458E-4)

class VerifyStatisticalTests(unittest.TestCase): 
	def testANOVA(self):
		"""Verify computation of ANOVA"""
		from plugins.multiGroups.statisticalTests.ANOVA import ANOVA
		anova = ANOVA(preferences)
		
		# checked against http://turner.faculty.swau.edu/mathematics/math241/materials/anova/
		pValue, _ = anova.hypothesisTest([[5,4,6,4,3],[5,2,2,5,6,7],[1,2,3,4,5,6,7]])
		self.assertAlmostEqual(pValue, 0.88347274205)
		
		# checked against http://faculty.vassar.edu/lowry/anova1u.html
		pValue, _ = anova.hypothesisTest([[1,2,3,4,5],[10,20,30,40,50],[4,5,4], [5,5,5]])
		self.assertAlmostEqual(pValue, 0.0018740823031)
		
		pValue, _ = anova.hypothesisTest([[5,4,5,4,5],[6,5,6,5,6,5],[700,800,700]])
		self.assertAlmostEqual(pValue, 0.0)
		
		pValue, _ = anova.hypothesisTest([[1,2,3,4,5],[1,2,3,4,5],[1,2,3,4,5]])
		self.assertAlmostEqual(pValue, 1.0)
		
	def testKruskalWallis(self):
		"""Verify computation of Kruskal-Wallis H-test"""
		from plugins.multiGroups.statisticalTests.KruskalWallis import KruskalWallis
		kw = KruskalWallis(preferences)
		
		# checked against http://faculty.vassar.edu/lowry/kw3.html
		pValue, _ = kw.hypothesisTest([[5,4,6,4,3],[5,2,2,5,6,7],[1,2,3,4,5,6,7]])
		self.assertAlmostEqual(pValue, 0.88173680194259985)
		
		pValue, _ = kw.hypothesisTest([[1,2,3,4,5,6,7],[8,9,10,11,12,13,14,15],[16,17,18,19,20,21,22]])
		self.assertAlmostEqual(pValue, 8.8020161301173428e-05)
		
		pValue, _ = kw.hypothesisTest([[1,2,3,4,5],[1,2,3,4,5],[1,2,3,4,5]])
		self.assertAlmostEqual(pValue, 1.0)
		
	def testTTest(self):
		"""Verify computation of t-test (equal variance assumption) """
		from plugins.groups.statisticalTests.Ttest import Ttest
		ttest = Ttest(preferences)
		
		# ground truth found with t.test in R v2.13.0
		oneSided, twoSided, lowerCI, upperCI, effectSize, _ = ttest.run([5,4,6,4,3],[5,2,2,5,6,7], [1,1,1,1,1], [1,1,1,1,1,1], None, 0.95)
		self.assertAlmostEqual(oneSided, 0.537141726)
		self.assertAlmostEqual(twoSided, 0.925716547365)
		self.assertAlmostEqual(lowerCI, -245.935268272)
		self.assertAlmostEqual(upperCI, 225.935268272)
		self.assertAlmostEqual(effectSize, -10.0)
		
	def testWelchTest(self):
		"""Verify computation of Welsh's t-test"""
		from plugins.groups.statisticalTests.Welch import Welch
		ttest = Welch(preferences)
		
		# ground truth found with t.test in R v2.13.0
		oneSided, twoSided, lowerCI, upperCI, effectSize, _ = ttest.run([5,4,6,4,3],[5,2,2,5,6,7], [1,1,1,1,1], [1,1,1,1,1,1], None, 0.95)
		self.assertAlmostEqual(oneSided, 0.5390501783)
		self.assertAlmostEqual(twoSided, 0.9218996432)
		self.assertAlmostEqual(lowerCI, -238.023177152)
		self.assertAlmostEqual(upperCI, 218.023177152)
		self.assertAlmostEqual(effectSize, -10.0)
		
		oneSided, twoSided, lowerCI, upperCI, effectSize, _ = ttest.run([3.4,6.3,5.3,1.4,6.3,6.3],[3.5,6.4,5.2,1.3,6.4,6.2], [1,1,1,1,1,1], [1,1,1,1,1,1], None, 0.95)
		self.assertAlmostEqual(oneSided, 0.5)
		self.assertAlmostEqual(twoSided, 1.0)
		self.assertAlmostEqual(lowerCI, -262.6606201199)
		self.assertAlmostEqual(upperCI, 262.6606201199)
		self.assertAlmostEqual(effectSize, 0.0)
		
		oneSided, twoSided, lowerCI, upperCI, effectSize, _ = ttest.run([1,2,3,4,5,6,7,8,9,10],[10,20,30,40,50,60,70,80,90,100], [1,1,1,1,1,1,1,1,1,1], [1,1,1,1,1,1,1,1,1,1], None, 0.95)
		self.assertAlmostEqual(oneSided, 0.9997146330)
		self.assertAlmostEqual(twoSided, 0.0005707338)
		self.assertAlmostEqual(lowerCI, -7120.16500998)
		self.assertAlmostEqual(upperCI, -2779.83499002)
		self.assertAlmostEqual(effectSize, -4950.0)
		
	def testWhiteTest(self):
		"""Verify computation of White's non-parametric test"""
		from plugins.groups.statisticalTests.White import White
		white = White(preferences)
		
		# This is a fairly degenerate test since the non-deterministic nature of this test
		# makes it difficult to verify under more general conditions
		_, pValuesTwoSided, lowerCIs, upperCIs, effectSizes, _  = white.runAll([[5,5,5,5,5]], [[6,6,6,6,6,6,6,6]], [[10,10,10,10,10]], [[10,10,10,10,10,10,10,10]], "DP: bootstrap", 0.95, None)
		self.assertAlmostEqual(pValuesTwoSided[0], 0.0)
		self.assertAlmostEqual(lowerCIs[0], -10.0)
		self.assertAlmostEqual(upperCIs[0], -10.0)
		self.assertAlmostEqual(effectSizes[0], -10.0)
		
	#def testBarnard(self):
	#	"""Verify computation of Barnard's exact test"""
	#	from plugins.statisticalTests.Barnard import Barnard
	#	barnard = Barnard(preferences)

		# Ground truth obtained from StatXact v8.0.0
	#	oneSided, twoSided = barnard.hypothesisTest(table1[0], table1[1], table1[2], table1[3])
	#	self.assertEqual(oneSided, float('inf'))
	#	self.assertAlmostEqual(twoSided, 0.224594642210276)
		
	def testChiSquare(self):
		"""Verify computation of Chi-square test"""
		from plugins.samples.statisticalTests.ChiSquare import ChiSquare
		chiSquare = ChiSquare(preferences) 
		
		# Ground truth obtained from R version 2.10		
		oneSided, twoSided, _ = chiSquare.hypothesisTest(table1[0], table1[1], table1[2], table1[3])	 
		self.assertEqual(oneSided, float('inf'))
		self.assertAlmostEqual(twoSided, 0.206550401252)
		
		oneSided, twoSided, _ = chiSquare.hypothesisTest(table2[0], table2[1], table2[2], table2[3])	 
		self.assertEqual(oneSided, float('inf'))
		self.assertAlmostEqual(twoSided, 2.220446049e-16)
		
	def testChiSquareYates(self):
		"""Verify computation of Chi-square test with Yates' continuity correction"""
		from plugins.samples.statisticalTests.ChiSquareYates import ChiSquareYates
		chiSquareYates = ChiSquareYates(preferences)
		
		# Ground truth obtained from R version 2.10		
		oneSided, twoSided, _ = chiSquareYates.hypothesisTest(table1[0], table1[1], table1[2], table1[3])	 
		self.assertEqual(oneSided, float('inf'))
		self.assertAlmostEqual(twoSided, 0.323739196466)
		
		oneSided, twoSided, _ = chiSquareYates.hypothesisTest(table2[0], table2[1], table2[2], table2[3])	 
		self.assertEqual(oneSided, float('inf'))
		self.assertAlmostEqual(twoSided, 2.220446049e-16)
		
	def testDiffBetweenProp(self):
		"""Verify computation of Difference between proportions test"""
		from plugins.samples.statisticalTests.DiffBetweenProp import DiffBetweenProp
		diffBetweenProp = DiffBetweenProp(preferences)
		
		# Ground truth obtained from R version 2.10		
		oneSided, twoSided, _ = diffBetweenProp.hypothesisTest(table1[0], table1[1], table1[2], table1[3])	 
		self.assertAlmostEqual(oneSided, 0.103275200626)
		self.assertAlmostEqual(twoSided, 0.206550401252)
		
		oneSided, twoSided, _ = diffBetweenProp.hypothesisTest(table2[0], table2[1], table2[2], table2[3])	 
		self.assertAlmostEqual(oneSided, 2.220446049e-16)
		self.assertAlmostEqual(twoSided, 2.220446049e-16)

	def testFishers(self):
		"""Verify computation of Fisher's exact test (minimum-likelihood approach)"""
		from plugins.samples.statisticalTests.Fishers import Fishers
		fishers = Fishers(preferences)
		
		# Ground truth obtained from R version 2.10		
		oneSided, twoSided, _ = fishers.hypothesisTest(table1[0], table1[1], table1[2], table1[3])	 
		self.assertAlmostEqual(oneSided, 0.16187126209690825)
		self.assertAlmostEqual(twoSided, 0.2715543327789185)
		
		oneSided, twoSided, _ = fishers.hypothesisTest(table2[0], table2[1], table2[2], table2[3])	 
		self.assertAlmostEqual(oneSided, 2.220446049e-16)
		self.assertAlmostEqual(twoSided, 2.220446049e-16)
		
		oneSided, twoSided, _ = fishers.hypothesisTest(0.0, 0.0, 920852.999591, 953828.994346)
		self.assertAlmostEqual(oneSided, 1.0)
		self.assertAlmostEqual(twoSided, 1.0)
		
	def testGTest(self):
		"""Verify computation of G-test"""
		from plugins.samples.statisticalTests.GTest import GTest
		gTest = GTest(preferences)
		
		# Ground truth obtained from Peter L. Hurd's R script (http://www.psych.ualberta.ca/~phurd/cruft/g.test.r)	
		oneSided, twoSided, _ = gTest.hypothesisTest(table1[0], table1[1], table1[2], table1[3])	 
		self.assertEqual(oneSided, float('inf'))
		self.assertAlmostEqual(twoSided, 0.208248664458)
		
		oneSided, twoSided, _ = gTest.hypothesisTest(table2[0], table2[1], table2[2], table2[3])	 
		self.assertEqual(oneSided, float('inf'))
		self.assertAlmostEqual(twoSided, 2.220446049e-16)
		
	def testGTestYates(self):
		"""Verify computation of G-test with Yates' continuity correction"""
		from plugins.samples.statisticalTests.GTestYates import GTestYates
		gTestYates = GTestYates(preferences)
		
		# Ground truth obtained from Peter L. Hurd's R script (http://www.psych.ualberta.ca/~phurd/cruft/g.test.r)	
		oneSided, twoSided, _ = gTestYates.hypothesisTest(table1[0], table1[1], table1[2], table1[3])	 
		self.assertEqual(oneSided, float('inf'))
		self.assertAlmostEqual(twoSided, 0.325502240010)
		
		oneSided, twoSided, _ = gTestYates.hypothesisTest(table2[0], table2[1], table2[2], table2[3])	 
		self.assertEqual(oneSided, float('inf'))
		self.assertAlmostEqual(twoSided, 2.220446049e-16)
		
	def testHypergeometric(self):
		"""Verify computation of Hypergeometric test (Fisher's exact test with p-value doubling approach)"""
		from plugins.samples.statisticalTests.Hypergeometric import Hypergeometric
		hypergeometric = Hypergeometric(preferences)
		
		# Ground truth obtained using the phyper() and dyper() function in R version 2.10	 
		oneSided, twoSided, _ = hypergeometric.hypothesisTest(table1[0], table1[1], table1[2], table1[3])	 
		self.assertAlmostEqual(oneSided, 0.161871262097)
		self.assertAlmostEqual(twoSided, 2 * 0.161871262097)
		
		oneSided, twoSided, _ = hypergeometric.hypothesisTest(table2[0], table2[1], table2[2], table2[3])	 
		self.assertAlmostEqual(oneSided, 2.220446049e-16)
		self.assertAlmostEqual(twoSided, 2.220446049e-16)
		
class VerifyEffectSizeFilters(unittest.TestCase):
	def testEtaSquared(self):
		"""Verify computation of eta-squared effect size filter"""
		from plugins.multiGroups.effectSizeFilters.EtaSquared import EtaSquared
		etaSquared = EtaSquared(preferences)
		
		# ground truth taken from http://turner.faculty.swau.edu/mathematics/math241/materials/anova/
		value = etaSquared.run([[1,2,3,4],[2,3,4],[1,2,3,4]])
		self.assertAlmostEqual(value, 0.545454545 / 12.545454545)
		
		# ground truth taken from http://faculty.vassar.edu/lowry/anova1u.html
		value = etaSquared.run([[1,2,3,4,5],[10,20,30,40,50],[4,5,4], [5,5,5]])
		self.assertAlmostEqual(value, 2348.27083333 / 3358.9375)
		
	def testDiffBetweenProp(self):
		"""Verify computation of Difference between proportions effect size filter"""
		from plugins.samples.effectSizeFilters.DiffBetweenProp import DiffBetweenProp
		diffBetweenProp = DiffBetweenProp(preferences)
		
		# Ground truth calculated by hand
		value = diffBetweenProp.run(table1[0], table1[1], table1[2], table1[3])
		self.assertAlmostEqual(value, 13.333333333)
		
		value = diffBetweenProp.run(table2[0], table2[1], table2[2], table2[3])
		self.assertAlmostEqual(value, 0.3)
		
	def testOddsRatio(self):
		"""Verify computation of Odds ratio effect size filter"""
		from plugins.samples.effectSizeFilters.OddsRatio import OddsRatio
		oddsRatio = OddsRatio(preferences)
		
		# Ground truth calculated by hand
		value = oddsRatio.run(table1[0], table1[1], table1[2], table1[3])
		self.assertAlmostEqual(value, 2.0)
		
		value = oddsRatio.run(table2[0], table2[1], table2[2], table2[3])
		self.assertAlmostEqual(value, 1.60483870968)
		
	def testRatioProportions(self):
		"""Verify computation of ratio of proportions effect size filter"""
		from plugins.samples.effectSizeFilters.RatioProportions import RatioProportions
		ratioProportions = RatioProportions(preferences)
		
		# Ground truth calculated by hand
		value = ratioProportions.run(table1[0], table1[1], table1[2], table1[3])
		self.assertAlmostEqual(value, 1.66666666666666)
		
		value = ratioProportions.run(table2[0], table2[1], table2[2], table2[3])
		self.assertAlmostEqual(value, 1.6)
		
	def testDiffBetweenPropGroup(self):
		"""Verify computation of Difference between proportions group effect size filter"""
		from plugins.groups.effectSizeFilters.DiffBetweenProp import DiffBetweenProp
		diffBetweenProp = DiffBetweenProp(preferences)
		
		# Ground truth calculated by hand
		value = diffBetweenProp.run([1,2,3,4,5], [2,4,5,8,10])
		self.assertAlmostEqual(value, 15.0/5 - 29.0/5)
		
		value = diffBetweenProp.run([1],[1,1])
		self.assertAlmostEqual(value, 1.0/1 - 2.0/2)
		
	def testRatioProportionsGroup(self):
		"""Verify computation of ratio of proportions group effect size filter"""
		from plugins.groups.effectSizeFilters.RatioProportions import RatioProportions
		ratioProportions = RatioProportions(preferences)
		
		# Ground truth calculated by hand
		value = ratioProportions.run([1,2,3,4,5], [2,4,5,8,10])
		self.assertAlmostEqual(value, (15.0/5) / (29.0/5))
		
		value = ratioProportions.run([1],[1,1])
		self.assertAlmostEqual(value, (1.0/1) / (2.0/2))
		
class VerifyConfidenceIntervalMethods(unittest.TestCase):
	def testDiffBetweenPropAsymptotic(self):
		"""Verify computation of Difference between proportions asymptotic CI method"""
		from plugins.samples.confidenceIntervalMethods.DiffBetweenPropAsymptotic import DiffBetweenPropAsymptotic
		diffBetweenPropAsymptotic = DiffBetweenPropAsymptotic(preferences)
		
		lowerCI, upperCI, effectSize, _ = diffBetweenPropAsymptotic.run(table1[0], table1[1], table1[2], table1[3], 0.95)
		self.assertAlmostEqual(lowerCI, -7.60015319099813)
		self.assertAlmostEqual(upperCI, 34.2668198576648)
		self.assertAlmostEqual(effectSize, 13.333333333)
				
		lowerCI, upperCI, effectSize, _ = diffBetweenPropAsymptotic.run(table2[0], table2[1], table2[2], table2[3], 0.95)
		self.assertAlmostEqual(lowerCI, 0.271701079166334)
		self.assertAlmostEqual(upperCI, 0.328298920833666)
		self.assertAlmostEqual(effectSize, 0.3)
		
	def testDiffBetweenPropAsymptoticCC(self):
		"""Verify computation of Difference between proportions asymptotic CI method with continuity correction"""
		from plugins.samples.confidenceIntervalMethods.DiffBetweenPropAsymptoticCC import DiffBetweenPropAsymptoticCC
		diffBetweenPropAsymptoticCC = DiffBetweenPropAsymptoticCC(preferences)
		
		lowerCI, upperCI, effectSize, _ = diffBetweenPropAsymptoticCC.run(table1[0], table1[1], table1[2], table1[3], 0.95)
		self.assertAlmostEqual(lowerCI, -13.3167148125733)
		self.assertAlmostEqual(upperCI, 39.98338147924)
		self.assertAlmostEqual(effectSize, 13.333333333)
				
		lowerCI, upperCI, effectSize, _ = diffBetweenPropAsymptoticCC.run(table2[0], table2[1], table2[2], table2[3], 0.95)
		self.assertAlmostEqual(lowerCI, 0.271407084568653)
		self.assertAlmostEqual(upperCI, 0.328592915431347)
		self.assertAlmostEqual(effectSize, 0.3)
		
	def testNewcombeWilson(self):
		"""Verify computation of Newcombe-Wilson CI method"""
		from plugins.samples.confidenceIntervalMethods.NewcombeWilson import NewcombeWilson
		newcombeWilson = NewcombeWilson(preferences)
		
		lowerCI, upperCI, effectSize, _ = newcombeWilson.run(table1[0], table1[1], table1[2], table1[3], 0.95)		
		self.assertAlmostEqual(lowerCI, -7.07911677674112)
		self.assertAlmostEqual(upperCI, 33.5862638376494)
		self.assertAlmostEqual(effectSize, 13.333333333)
				
		lowerCI, upperCI, effectSize, _ = newcombeWilson.run(table2[0], table2[1], table2[2], table2[3], 0.95)
		self.assertAlmostEqual(lowerCI, 0.271932757939523)
		self.assertAlmostEqual(upperCI, 0.328541077116921)
		self.assertAlmostEqual(effectSize, 0.3)
		
	def testOddsRatio(self):
		"""Verify computation of Odds ratio CI method"""
		from plugins.samples.confidenceIntervalMethods.OddsRatio import OddsRatio
		oddsRatio = OddsRatio(preferences)
		
		# Ground truth calculated by hand
		lowerCI, upperCI, effectSize, _ = oddsRatio.run(table1[0], table1[1], table1[2], table1[3], 0.95)	
		self.assertAlmostEqual(lowerCI, 0.676046021596)
		self.assertAlmostEqual(upperCI, 5.91675695474)
		self.assertAlmostEqual(effectSize, 2.0)

		lowerCI, upperCI, effectSize, _ = oddsRatio.run(table2[0], table2[1], table2[2], table2[3], 0.95)
		self.assertAlmostEqual(lowerCI, 1.53926774059)
		self.assertAlmostEqual(upperCI, 1.6732029238)
		self.assertAlmostEqual(effectSize, 1.60483870968)
		
	def testRatioProportions(self):
		"""Verify computation of Ratio of proportions CI method"""
		from plugins.samples.confidenceIntervalMethods.RatioProportions import RatioProportions
		ratioProportions = RatioProportions(preferences)
		
		# Ground truth calculated by hand
		lowerCI, upperCI, effectSize, _ = ratioProportions.run(table1[0], table1[1], table1[2], table1[3], 0.95)	
		self.assertAlmostEqual(lowerCI, 0.748767825898)
		self.assertAlmostEqual(upperCI, 3.70979852726)
		self.assertAlmostEqual(effectSize, 1.66666666666666)
				
		lowerCI, upperCI, effectSize, _ = ratioProportions.run(table2[0], table2[1], table2[2], table2[3], 0.95)
		self.assertAlmostEqual(lowerCI, 1.53505365781)
		self.assertAlmostEqual(upperCI, 1.6676941467)
		self.assertAlmostEqual(effectSize, 1.6)
		
class VerifyMultipleComparisonCorrectionMethods(unittest.TestCase):	
	pValues = [1e-6, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1]
	
	def testBenjaminiHochbergFDR(self):
		"""Verify computation of Bejamini-Hochberg FDR method"""
		from plugins.common.multipleComparisonCorrections.BenjaminiHochbergFDR import BenjaminiHochbergFDR
		benjaminiHochbergFDR = BenjaminiHochbergFDR(preferences)
		
		# Ground truth calculated explicitly
		qValues = benjaminiHochbergFDR.correct(list(self.pValues), 0.05)
		modifier = 1
		for i in xrange(0, len(self.pValues)):
			self.assertAlmostEqual(qValues[i], self.pValues[i]*len(self.pValues) / modifier)
			modifier += 1
			
	def testBonferroni(self):
		"""Verify computation of Bonferroni method"""
		from plugins.common.multipleComparisonCorrections.Bonferroni import Bonferroni
		bonferroni = Bonferroni(preferences)
		
		# Ground truth calculated explicitly
		correctedValues = bonferroni.correct(list(self.pValues), 0.05)
		for i in xrange(0, len(self.pValues)):
			self.assertAlmostEqual(correctedValues[i], self.pValues[i]*len(self.pValues))
			
	def testHolmBonferroni(self):
		"""Verify computation of Holm-Bonferroni method"""
		from plugins.common.multipleComparisonCorrections.additional.HolmBonferroni import HolmBonferroni
		holmBonferroni = HolmBonferroni(preferences)
		
		# Ground truth calculated by hand
		correctedValues = holmBonferroni.correct(list(self.pValues), 0.05)
		self.assertAlmostEqual(correctedValues[0], self.pValues[0])
		self.assertAlmostEqual(correctedValues[1], self.pValues[1])
		self.assertAlmostEqual(correctedValues[2], self.pValues[2])
		self.assertAlmostEqual(correctedValues[3], self.pValues[3])
		self.assertAlmostEqual(correctedValues[4], self.pValues[4])
		self.assertEqual(correctedValues[5], float('inf'))
		
	def testNoCorrection(self):
		"""Verify computation of No multiple comparison correction method"""
		from plugins.common.multipleComparisonCorrections.NoCorrection import NoCorrection
		noCorrection = NoCorrection(preferences)
		
		# Ground truth calculated explicitly
		correctedValues = noCorrection.correct(list(self.pValues), 0.05)
		for i in xrange(0, len(self.pValues)):
			self.assertAlmostEqual(correctedValues[i], self.pValues[i])
			
	def testSidak(self):
		"""Verify computation of Sidak method"""
		from plugins.common.multipleComparisonCorrections.Sidak import Sidak
		sidak = Sidak(preferences)
		
		# Ground truth calculated explicitly
		correctedValues = sidak.correct(list(self.pValues), 0.05)
		for i in xrange(0, len(self.pValues)):
			self.assertAlmostEqual(correctedValues[i], 1.0 - (1.0 - self.pValues[i])**len(self.pValues))
			
	def testStoreyFDR(self):
		"""Verify computation of Storey FDR method"""

		# This method is based on a bootstrapping approach and as such does not always produce
		# identical results. It has been tested against the results given by the R plugin by
		# Alan Dadney and John Storey (http://cran.r-project.org/web/packages/qvalue/)
		pass
		
class VerifyOther(unittest.TestCase):
	def testNormalDist(self):
		"""Verify computation of normal distribution methods"""
		from metagenomics.stats.distributions.NormalDist import standardNormalCDF, zScore
		
		self.assertAlmostEqual(standardNormalCDF(-2), 0.022750131948179209)
		self.assertAlmostEqual(standardNormalCDF(-1), 0.15865525393145705)
		self.assertAlmostEqual(standardNormalCDF(0), 0.5)
		self.assertAlmostEqual(standardNormalCDF(1), 0.84134474606854293)
		self.assertAlmostEqual(standardNormalCDF(2), 0.97724986805182079)
		self.assertAlmostEqual(standardNormalCDF(-1e-6), 1.0 - standardNormalCDF(1e-6))
		self.assertAlmostEqual(standardNormalCDF(-1e-12), 1.0 - standardNormalCDF(1e-12))
		
		self.assertAlmostEqual(zScore(0.90), 1.6448536269514722)
		self.assertAlmostEqual(zScore(0.95), 1.959963984540054)
		self.assertAlmostEqual(zScore(0.98), 2.3263478740408408)
		self.assertAlmostEqual(zScore(0.99), 2.5758293035489004)
		self.assertAlmostEqual(zScore(0.80), 1.2815515655446004)

if __name__ == "__main__":
	unittest.main()