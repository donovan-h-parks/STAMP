import math
import random

from stamp.metagenomics.plugins.statisticalTests.Fishers import Fishers
from stamp.metagenomics.plugins.statisticalTests.GTest import GTest
from stamp.metagenomics.plugins.statisticalTests.GTestYates import GTestYates
from stamp.metagenomics.plugins.statisticalTests.DiffBetweenProp import DiffBetweenProp

fishers = Fishers()
gTest = GTest()
gTestYates = GTestYates()
diffBetweenProp = DiffBetweenProp()

N_Values = [250]
maxPositives = [250]

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
  
fout = open('temp.txt','w')

random.seed(0)

for N in N_Values:
  print N
  totalSamples1 = int(N/2)
  totalSamples2 = int(N/2)
  
  for maxP in maxPositives:
    print '  ' + str(maxP)
    if maxP > totalSamples1:
      continue
    
    fishersResults = []
    gTestResults = []
    gTestYatesResults = []
    diffBetweenPropResults = []
    for a in xrange(0, maxP+1):
      for b in xrange(a, maxP+1):        
        # calculate p-values
        fishersOneSided, fishersTwoSided = fishers.hypothesisTest(a, b, totalSamples1, totalSamples2)
        if (fishersTwoSided < 0.01 or fishersTwoSided > 0.1 or a <= 10 or b <= 10):
          continue
        
        gTestOneSided, gTestTwoSided = gTest.hypothesisTest(a, b, totalSamples1, totalSamples2)
        gTestYatesOneSided, gTestYatesTwoSided = gTestYates.hypothesisTest(a, b, totalSamples1, totalSamples2)
        diffBetweenPropOneSided, diffBetweenPropTwoSided = diffBetweenProp.hypothesisTest(a, b, totalSamples1, totalSamples2)
        
        # calculate p-values relative to Fisher's  
        fishersResults.append(fishersTwoSided)
        gTestResults.append(gTestTwoSided)
        gTestYatesResults.append(gTestYatesTwoSided)
        diffBetweenPropResults.append(diffBetweenPropTwoSided)      
        
    if len(gTestResults) > 0:
      fout.write('N: ' + str(N) + '\n')
      fout.write('Max positives: ' + str(maxP) + '\n')
      fout.write('Evaluated tables: ' + str(len(gTestResults)) + '\n')
          
      testResults = [gTestResults, gTestYatesResults, diffBetweenPropResults]
      method = ['G-test', 'G-test w/ Yates', 'Diff. between proportions']
      for i in xrange(0, len(testResults)):
        # calculate relative error and number of non-significant features
        results = []
        missedSignFeatures = 0
        for j in xrange(0, len(testResults[i])):
          results.append((testResults[i][j] - fishersResults[j]) / fishersResults[j])
          
          if fishersResults[j] < 0.05 and testResults[i][j] > 0.05:
            missedSignFeatures += 1
          
        meanStr = "%.2f" % mean(results)
        sdStr = "%.2f" % stdDev(results)      
        minStr= "%.2f" % min(results)
        maxStr = "%.2f" % max(results)
        
        fout.write(method[i] + ': ' + meanStr + '+/-' + sdStr + '[' + minStr + '; ' + maxStr + ']' + '; Missed sign. features = ' + str(missedSignFeatures) + '\n')
      
      fout.write('\n\n')

fout.close()
    