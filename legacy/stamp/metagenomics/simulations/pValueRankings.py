from stamp.metagenomics.plugins.statisticalTests.Fishers import Fishers
from stamp.metagenomics.plugins.statisticalTests.ChiSquare import ChiSquare
from stamp.metagenomics.plugins.statisticalTests.ChiSquareYates import ChiSquareYates
from stamp.metagenomics.plugins.statisticalTests.GTest import GTest
from stamp.metagenomics.plugins.statisticalTests.GTestYates import GTestYates
from stamp.metagenomics.plugins.statisticalTests.Permutation import Permutation
from stamp.metagenomics.plugins.statisticalTests.DiffBetweenProp import DiffBetweenProp
from stamp.metagenomics.plugins.statisticalTests.Bootstrap import Bootstrap 

from metagenomics.stats.tests.Spearman import Spearman

fishers = Fishers()
chiSquare = ChiSquare()
chiSquareYates = ChiSquareYates()
gTest = GTest()
gTestYates = GTestYates()
permutation = Permutation()
diffBetweenProp = DiffBetweenProp()
bootstrap = Bootstrap()

# Create paired profile for Red-black dataset
fin = open('RedBlack.csv')
data = fin.readlines()
fin.close()

totalSamples1 = 0
totalSamples2 = 0
featureDict = {}
for i in xrange(1, len(data)):
  lineSplit = data[i].split(',')
  
  feature = lineSplit[2]
  seq1 = int(lineSplit[3])
  seq2 = int(lineSplit[4])
  
  if feature in featureDict:
    curSeqs = featureDict[feature]
    featureDict[feature] = [curSeqs[0] + seq1, curSeqs[1] + seq2]
  else:
    featureDict[feature] = [seq1, seq2]
  
  totalSamples1 += seq1
  totalSamples2 += seq2
  
# Calculate p-values for each feature
fishersDict = {}
chiSquareDict = {}
chiSquareYatesDict = {}
gTestDict = {}
gTestYatesDict = {}
permutationDict = {}
diffBetweenPropDict = {}
bootstrapDict = {}
for feature in featureDict:
  print feature
  
  a = featureDict[feature][0]
  b = featureDict[feature][1]
  
  if a <= 10 or b <= 10:
    continue
  
  diff = (float(a) / totalSamples1 - float(b) / totalSamples2) * 100
  
  fishersOneSided, fishersTwoSided = fishers.hypothesisTest(a, b, totalSamples1, totalSamples2)
  if (fishersTwoSided > 0.05 or abs(diff) < 0.5):
    continue
    
  chiSquareOneSided, chiSquareTwoSided = chiSquare.hypothesisTest(a, b, totalSamples1, totalSamples2)     
  chiSquareYatesOneSided, chiSquareYatesTwoSided = chiSquareYates.hypothesisTest(a, b, totalSamples1, totalSamples2) 
  gTestOneSided, gTestTwoSided = gTest.hypothesisTest(a, b, totalSamples1, totalSamples2)
  gTestYatesOneSided, gTestYatesTwoSided = gTestYates.hypothesisTest(a, b, totalSamples1, totalSamples2)
  permutationOneSided, permutationTwoSided = permutation.hypothesisTest(a, b, totalSamples1, totalSamples2)
  diffBetweenPropOneSided, diffBetweenPropTwoSided = diffBetweenProp.hypothesisTest(a, b, totalSamples1, totalSamples2)
  bootstrapOneSided, bootstrapTwoSided = bootstrap.hypothesisTest(a, b, totalSamples1, totalSamples2)
  
  if fishersTwoSided < 1e-15:
    fishersTwoSided = 1e-15
    
  if chiSquareTwoSided < 1e-15:
    chiSquareTwoSided = 1e-15
    
  if chiSquareYatesTwoSided < 1e-15:
    chiSquareYatesTwoSided = 1e-15
    
  if gTestTwoSided < 1e-15:
    gTestTwoSided = 1e-15
    
  if gTestYatesTwoSided < 1e-15:
    gTestYatesTwoSided = 1e-15
    
  if permutationTwoSided < 1e-15:
    permutationTwoSided = 1e-15
    
  if diffBetweenPropTwoSided < 1e-15:
    diffBetweenPropTwoSided = 1e-15
    
  if bootstrapTwoSided < 1e-15:
    bootstrapTwoSided = 1e-15

  fishersDict[feature] = fishersTwoSided
  chiSquareDict[feature] = chiSquareTwoSided
  chiSquareYatesDict[feature] = chiSquareYatesTwoSided
  gTestDict[feature] = gTestTwoSided
  gTestYatesDict[feature] = gTestYatesTwoSided
  permutationDict[feature] = permutationTwoSided
  diffBetweenPropDict[feature] = diffBetweenPropTwoSided
  bootstrapDict[feature] = bootstrapTwoSided
  
# Calculate Spearman's for each pair of tests for those features where at least one of the tests has a p-value < 0.05
testDict = [fishersDict,chiSquareDict,chiSquareYatesDict, gTestDict, gTestYatesDict, permutationDict, diffBetweenPropDict, bootstrapDict]
testNames = ['Fisher\'s', 'Chi-square','Chi-square w/ Yates','G-test','G-test', 'G-test w/ Yates', 'Permutation', 'Diff b/w prop', 'Bootstrap']

fout = open('SpearmanRankings.csv', 'w')
for name in testNames:
  fout.write(',' + name)
fout.write('\n')

spearman = Spearman()
for i in xrange(0, len(testDict)):
  fout.write(testNames[i])
  for j in xrange(0, len(testDict)):
    test1 = testDict[i]
    test2 = testDict[j]
    
    # get all features with a p-value < 0.05
    data1 = []
    data2 = [] 
    for feature in test1:
      if test1[feature] < 0.05 or test2[feature] < 0.05:
        data1.append(test1[feature])
        data2.append(test2[feature])
            
    Rs = spearman.compute(data1, data2)
    
    fout.write(',' + str(Rs))
    
  fout.write('\n')
  
fout.close()
    