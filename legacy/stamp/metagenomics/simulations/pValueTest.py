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

fishers = Fishers()
chiSquare = ChiSquare()
chiSquareYates = ChiSquareYates()
gTest = GTest()
gTestYates = GTestYates()
permutation = Permutation()
diffBetweenProp = DiffBetweenProp()
bootstrap = Bootstrap()

sampleSize = [30, 50, 100, 200, 500, 1000, 10000, 100000]

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
  
fout = open('pValueTest.txt','w')

random.seed()

for N in sampleSize:
  print N
  totalSamples1 = N
  totalSamples2 = N*2
  
  maxPositiveSeqs = N-10
  if maxPositiveSeqs > 200:
    maxPositiveSeqs = 200

  fishersResults = []
  chiSquareResults = []
  chiSquareYatesResults = []
  gTestResults = []
  gTestYatesResults = []
  permutationResults = []
  diffBetweenPropResults = []
  bootstrapResults = []
  for a in xrange(11, maxPositiveSeqs):
    for b in xrange(a, maxPositiveSeqs):        
      # calculate p-values
      fishersOneSided, fishersTwoSided = fishers.hypothesisTest(a, b, totalSamples1, totalSamples2)
      if (fishersTwoSided < 0.01 or fishersTwoSided > 0.1):
        continue
      
      chiSquareOneSided, chiSquareTwoSided = chiSquare.hypothesisTest(a, b, totalSamples1, totalSamples2)     
      chiSquareYatesOneSided, chiSquareYatesTwoSided = chiSquareYates.hypothesisTest(a, b, totalSamples1, totalSamples2)    
      gTestOneSided, gTestTwoSided = gTest.hypothesisTest(a, b, totalSamples1, totalSamples2)
      gTestYatesOneSided, gTestYatesTwoSided = gTestYates.hypothesisTest(a, b, totalSamples1, totalSamples2)    
      diffBetweenPropOneSided, diffBetweenPropTwoSided = diffBetweenProp.hypothesisTest(a, b, totalSamples1, totalSamples2)
      
      if N <= 200:
        permutationOneSided, permutationTwoSided = permutation.hypothesisTest(a, b, totalSamples1, totalSamples2)
        bootstrapOneSided, bootstrapTwoSided = bootstrap.hypothesisTest(a, b, totalSamples1, totalSamples2)
      else:
        permutationOneSided, permutationTwoSided = [0,0]
        bootstrapOneSided, bootstrapTwoSided = [0,0]
      
      fishersResults.append(fishersTwoSided)
      chiSquareResults.append(chiSquareTwoSided)
      chiSquareYatesResults.append(chiSquareYatesTwoSided)
      gTestResults.append(gTestTwoSided)
      gTestYatesResults.append(gTestYatesTwoSided)
      permutationResults.append(permutationTwoSided)
      diffBetweenPropResults.append(diffBetweenPropTwoSided)      
      bootstrapResults.append(bootstrapTwoSided)
      
  if len(gTestResults) > 0:
    fout.write('N: ' + str(N) + '\n')
    fout.write('Evaluated tables: ' + str(len(gTestResults)) + '\n')
        
    testResults = [chiSquareResults,chiSquareYatesResults, gTestResults, gTestYatesResults, permutationResults, diffBetweenPropResults, bootstrapResults]
    method = ['Chi-square','Chi-square w/ Yates','G-test', 'G-test w/ Yates', 'Permutation', 'Diff. between proportions', 'Bootstrap']
    for i in xrange(0, len(testResults)):
      # calculate relative error and number of non-significant features
      results = []
      missedSignFeatures = 0
      for j in xrange(0, len(testResults[i])):
        results.append(100 * (testResults[i][j] - fishersResults[j]) / fishersResults[j])
        
        if fishersResults[j] < 0.05 and testResults[i][j] > 0.05:
          missedSignFeatures += 1
        
      meanStr = "%.2f" % mean(results)
      sdStr = "%.2f" % stdDev(results)      
      minStr= "%.2f" % min(results)
      maxStr = "%.2f" % max(results)
      
      fout.write(method[i] + ': ' + meanStr + '+/-' + sdStr + '%[' + minStr + '%; ' + maxStr + '%]' + '; Missed sign. features = ' + str(missedSignFeatures) + '\n')
    
    fout.write('\n\n')

fout.close()
    