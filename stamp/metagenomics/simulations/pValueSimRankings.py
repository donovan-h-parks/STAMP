import math
import random

from stamp.metagenomics.plugins.statisticalTests.Fishers import Fishers
from stamp.metagenomics.plugins.statisticalTests.GTest import GTest
from stamp.metagenomics.plugins.statisticalTests.GTestYates import GTestYates
from stamp.metagenomics.plugins.statisticalTests.DiffBetweenProp import DiffBetweenProp

from stamp.metagenomics.stats.tests.Spearman import Spearman

fishers = Fishers()
gTest = GTest()
gTestYates = GTestYates()
diffBetweenProp = DiffBetweenProp()

N_Values = [2000, 20000, 200000]
maxPositives = [5, 10, 20, 50, 100, 200, 500]

def mean(x):
  if len(x) == 0:
    return 0
    
  sum = 0.0
  for i in xrange(0, len(x)):
    sum += x[i]
  return sum / len(x)
  
def stdDev(x):
  if len(x) == 0:
    return 0
    
  m = mean(x)
  sumsq = 0.0
  for i in xrange(0, len(x)):
    sumsq += (x[i] - m)*(x[i] - m)
  return math.sqrt(sumsq / len(x))
  
fout = open('pValueSimRankings.txt','w')

random.seed(0)
replicates = 100

fishersDict = {}
gTestDict = {}
gTestYatesDict = {}
diffBetweenPropDict = {}
for r in xrange(0, replicates):
  print r
  a = random.randint(0,10000)
  b = int(a + random.gauss(0,0.5*a))
  if b < 0:
    print 'here'
    b = 0
    
  totalSamples1 = random.randint(max(a,b), 100000)
  totalSamples2 = random.randint(max(a,b), 100000)
     
  # calculate p-values
  fishersOneSided, fishersTwoSided = fishers.hypothesisTest(a, b, totalSamples1, totalSamples2)
  if fishersTwoSided < 1e-10:
    continue
  
  gTestOneSided, gTestTwoSided = gTest.hypothesisTest(a, b, totalSamples1, totalSamples2)
  gTestYatesOneSided, gTestYatesTwoSided = gTestYates.hypothesisTest(a, b, totalSamples1, totalSamples2)
  diffBetweenPropOneSided, diffBetweenPropTwoSided = diffBetweenProp.hypothesisTest(a, b, totalSamples1, totalSamples2)
 
  fishersDict[r] = fishersTwoSided
  gTestDict[r] = gTestTwoSided
  gTestYatesDict[r] = gTestYatesTwoSided
  diffBetweenPropDict[r] = diffBetweenPropTwoSided
  
# Calculate Spearman's for each pair of tests for those features where at least one of the tests has a p-value < 0.05
testDict = [fishersDict, gTestDict, gTestYatesDict, diffBetweenPropDict]
testNames = ['Fisher\'s', 'G-test', 'G-test w/ Yates', 'Diff b/w prop']

fout = open('RankingsSim.csv', 'w')
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
            
    print len(data1)
    Rs = spearman.compute(data1, data2)
    
    fout.write(',' + str(Rs))
    
  fout.write('\n')
  
fout.close()
    