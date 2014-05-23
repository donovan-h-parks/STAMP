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

sampleSize = 50

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
  
fout = open('pValuePlots.csv','w')

random.seed(0)

totalSamples1 = 50
totalSamples2 = totalSamples1*2

maxPositiveSeqs = totalSamples1-10
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
  print a
  for b in xrange(a, maxPositiveSeqs):    
        
    # calculate p-values
    fishersOneSided, fishersTwoSided = fishers.hypothesisTest(a, b, totalSamples1, totalSamples2)        
    chiSquareOneSided, chiSquareTwoSided = chiSquare.hypothesisTest(a, b, totalSamples1, totalSamples2)     
    chiSquareYatesOneSided, chiSquareYatesTwoSided = chiSquareYates.hypothesisTest(a, b, totalSamples1, totalSamples2)    
    gTestOneSided, gTestTwoSided = gTest.hypothesisTest(a, b, totalSamples1, totalSamples2)
    gTestYatesOneSided, gTestYatesTwoSided = gTestYates.hypothesisTest(a, b, totalSamples1, totalSamples2)    
    diffBetweenPropOneSided, diffBetweenPropTwoSided = diffBetweenProp.hypothesisTest(a, b, totalSamples1, totalSamples2)
    permutationOneSided, permutationTwoSided = permutation.hypothesisTest(a, b, totalSamples1, totalSamples2)
    bootstrapOneSided, bootstrapTwoSided = bootstrap.hypothesisTest(a, b, totalSamples1, totalSamples2)
    
    fishersResults.append(fishersTwoSided)
    chiSquareResults.append(chiSquareTwoSided)
    chiSquareYatesResults.append(chiSquareYatesTwoSided)
    gTestResults.append(gTestTwoSided)
    gTestYatesResults.append(gTestYatesTwoSided)
    permutationResults.append(permutationTwoSided)
    diffBetweenPropResults.append(diffBetweenPropTwoSided)      
    bootstrapResults.append(bootstrapTwoSided)
           
testResults = [fishersResults, chiSquareResults,chiSquareYatesResults, gTestResults, gTestYatesResults, permutationResults, diffBetweenPropResults, bootstrapResults]
methods = ['Fisher\'s exact test', 'Chi-square','Chi-square w/ Yates','G-test', 'G-test w/ Yates', 'Permutation', 'Diff. between proportions', 'Bootstrap']

for method in methods:
  fout.write(method + ',') 
fout.write('\n')

for i in xrange(0, len(testResults[0])):
  for j in xrange(0, len(testResults)):        
    fout.write(str(testResults[j][i]) + ',')      
  fout.write('\n')

fout.close()
    