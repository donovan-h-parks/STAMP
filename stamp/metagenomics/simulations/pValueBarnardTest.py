import math
import random

from stamp.metagenomics.plugins.statisticalTests.Fishers import Fishers
from stamp.metagenomics.plugins.statisticalTests.ChiSquare import ChiSquare
from stamp.metagenomics.plugins.statisticalTests.ChiSquareYates import ChiSquareYates
from stamp.metagenomics.plugins.statisticalTests.GTest import GTest
from stamp.metagenomics.plugins.statisticalTests.GTestYates import GTestYates
from stamp.metagenomics.plugins.statisticalTests.Permutation import Permutation
from stamp.metagenomics.plugins.statisticalTests.DiffBetweenProp import DiffBetweenProp
from stamp.metagenomics.plugins.statisticalTests.Bootstrap import Bootstrap 
from stamp.metagenomics.plugins.statisticalTests.Barnard import Barnard 

fishers = Fishers()
chiSquare = ChiSquare()
chiSquareYates = ChiSquareYates()
gTest = GTest()
gTestYates = GTestYates()
permutation = Permutation()
diffBetweenProp = DiffBetweenProp()
bootstrap = Bootstrap()
barnard = Barnard()

sampleSizes = [5, 10, 15, 25]

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
  
fout = open('pValueTest_Barnard.txt','w')

random.seed(0)

for N in sampleSizes:
  print N
  totalSamples1 = N
  totalSamples2 = 2*N
  
  maxPositiveSeqs = N
    
  fishersResults = []
  chiSquareResults = []
  chiSquareYatesResults = []
  gTestResults = []
  gTestYatesResults = []
  permutationResults = []
  diffBetweenPropResults = []
  bootstrapResults = []
  for a in xrange(0, maxPositiveSeqs+1):
    for b in xrange(a, maxPositiveSeqs+1):        
      # calculate p-values
      barnardOneSided, barnardTwoSided = barnard.hypothesisTest(a, b, totalSamples1, totalSamples2)
      if (barnardTwoSided < 0.01 or barnardTwoSided > 0.1):
        continue
      
      fishersOneSided, fishersTwoSided = fishers.hypothesisTest(a, b, totalSamples1, totalSamples2)
      chiSquareOneSided, chiSquareTwoSided = chiSquare.hypothesisTest(a, b, totalSamples1, totalSamples2)     
      chiSquareYatesOneSided, chiSquareYatesTwoSided = chiSquareYates.hypothesisTest(a, b, totalSamples1, totalSamples2)    
      gTestOneSided, gTestTwoSided = gTest.hypothesisTest(a, b, totalSamples1, totalSamples2)
      gTestYatesOneSided, gTestYatesTwoSided = gTestYates.hypothesisTest(a, b, totalSamples1, totalSamples2)    
      diffBetweenPropOneSided, diffBetweenPropTwoSided = diffBetweenProp.hypothesisTest(a, b, totalSamples1, totalSamples2)
      permutationOneSided, permutationTwoSided = permutation.hypothesisTest(a, b, totalSamples1, totalSamples2)
      bootstrapOneSided, bootstrapTwoSided = bootstrap.hypothesisTest(a, b, totalSamples1, totalSamples2)
      
      # calculate p-values relative to Barnard's  
      fishersResults.append(100 * (fishersTwoSided - barnardTwoSided) / barnardTwoSided)
      chiSquareResults.append(100 * (chiSquareTwoSided - barnardTwoSided) / barnardTwoSided)
      chiSquareYatesResults.append(100 * (chiSquareYatesTwoSided - barnardTwoSided) / barnardTwoSided)
      gTestResults.append(100 * (gTestTwoSided - barnardTwoSided) / barnardTwoSided)
      gTestYatesResults.append(100 * (gTestYatesTwoSided - barnardTwoSided) / barnardTwoSided)
      permutationResults.append(100 * (permutationTwoSided - barnardTwoSided) / barnardTwoSided)
      diffBetweenPropResults.append(100 * (diffBetweenPropTwoSided - barnardTwoSided) / barnardTwoSided)      
      bootstrapResults.append(100 * (bootstrapTwoSided - barnardTwoSided) / barnardTwoSided)
      
  fout.write('N: ' + str(N) + '\n')
  fout.write('Evaluated tables: ' + str(len(gTestResults)) + '\n')
      
  testResults = [fishersResults, chiSquareResults,chiSquareYatesResults, gTestResults, gTestYatesResults, permutationResults, diffBetweenPropResults, bootstrapResults]
  method = ['Fisher\'s exact test', 'Chi-square','Chi-square w/ Yates','G-test', 'G-test w/ Yates', 'Permutation', 'Diff. between proportions', 'Bootstrap']
  
  for i in xrange(0, len(testResults)):
    results = testResults[i]
    meanStr = "%.2f" % mean(results)
    sdStr = "%.2f" % stdDev(results)      
    minStr= "%.2f" % min(results)
    maxStr = "%.2f" % max(results)
    
    fout.write(method[i] + ': ' + meanStr + '+/-' + sdStr + '%[' + minStr + '%; ' + maxStr + '%]' + '\n')
    
  fout.write('\n\n')

fout.close()
    