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
fishersResults = []
chiSquareResults = []
chiSquareYatesResults = []
gTestResults = []
gTestYatesResults = []
permutationResults = []
diffBetweenPropResults = []
bootstrapResults = []
signFeatures = []

for feature in featureDict:
  print feature
  
  a = featureDict[feature][0]
  b = featureDict[feature][1]
  diff = (float(a) / totalSamples1 - float(b) / totalSamples2) * 100
  
  fishersOneSided, fishersTwoSided = fishers.hypothesisTest(a, b, totalSamples1, totalSamples2)
  if (fishersTwoSided > 0.05 or abs(diff) < 0.5 or a <= 10 or b <= 10):
        continue
      
  signFeatures.append(feature)
  
  chiSquareOneSided, chiSquareTwoSided = chiSquare.hypothesisTest(a, b, totalSamples1, totalSamples2)     
  chiSquareYatesOneSided, chiSquareYatesTwoSided = chiSquareYates.hypothesisTest(a, b, totalSamples1, totalSamples2)    
  gTestOneSided, gTestTwoSided = gTest.hypothesisTest(a, b, totalSamples1, totalSamples2)
  gTestYatesOneSided, gTestYatesTwoSided = gTestYates.hypothesisTest(a, b, totalSamples1, totalSamples2)    
  diffBetweenPropOneSided, diffBetweenPropTwoSided = diffBetweenProp.hypothesisTest(a, b, totalSamples1, totalSamples2)
  #permutationOneSided, permutationTwoSided = permutation.hypothesisTest(a, b, totalSamples1, totalSamples2)
  #bootstrapOneSided, bootstrapTwoSided = bootstrap.hypothesisTest(a, b, totalSamples1, totalSamples2)

  # calculate p-values relative to Fisher's  
  fishersResults.append(fishersTwoSided)
  chiSquareResults.append(chiSquareTwoSided)
  chiSquareYatesResults.append(chiSquareYatesTwoSided)
  gTestResults.append(gTestTwoSided)
  gTestYatesResults.append(gTestYatesTwoSided)
  diffBetweenPropResults.append(diffBetweenPropTwoSided)   
  #permutationResults.append(permutationTwoSided)   
  #bootstrapResults.append(bootstrapTwoSided)

testResults = [fishersResults, chiSquareResults,chiSquareYatesResults, gTestResults, gTestYatesResults, diffBetweenPropResults]#, permutationResults, bootstrapResults]
methods = ['Fisher\'s exact test', 'Chi-square','Chi-square w/ Yates','G-test', 'G-test w/ Yates', 'Diff. between proportions', 'Permutation', 'Bootstrap']
      
fout = open('pValueTest_RedBlack.csv','w')
for method in methods:
  fout.write(',' + method)
fout.write('\n')

for i in xrange(0, len(signFeatures)):
  fout.write(signFeatures[i])
        
  for j in xrange(0, len(testResults)):   
    testResultStr = '%.2e' % testResults[j][i]
    
    if j == 0: 
      fout.write(',' + testResultStr)  
    else:
      if testResults[0][i] != 0:
        relErrorStr = "%.1f" % (((testResults[j][i] - testResults[0][i]) / testResults[0][i]) * 100)
      else:
        relErrorStr = 'NA'
      fout.write(',' + testResultStr + ' (' + relErrorStr + ')')  
  fout.write('\n')

fout.close()
    