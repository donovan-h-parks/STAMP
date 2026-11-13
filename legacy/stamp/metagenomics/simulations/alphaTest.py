import random

from stamp.metagenomics.plugins.statisticalTests.Fishers import Fishers
from stamp.metagenomics.plugins.statisticalTests.GTest import GTest
from stamp.metagenomics.plugins.statisticalTests.GTestYates import GTestYates
from stamp.metagenomics.plugins.statisticalTests.DiffBetweenProp import DiffBetweenProp

fishers = Fishers()
gTest = GTest()
gTestYates = GTestYates()
diffBetweenProp = DiffBetweenProp()

numReplicates = 10000             # number of samples to draw

popSize = 1000000       # size of underlying population
propInSS = [0.1]       # proportion of sequences in subsystem (SS) of interest

sampleSize1 = [1000]    # sample size for community 1
sampleSize2 = [2000]    # sample size for community 2

for sampleSizeIndex in xrange(0, len(sampleSize1)):
  print 'Considering communities with a sample size of: ' + str(sampleSize1[sampleSizeIndex]) + ' and ' + str(sampleSize2[sampleSizeIndex])
  for prop in propInSS:
    print '  Proportion of sequences in subsystem of interest: ' + str(prop)
          
    totalSamples1 = sampleSize1[sampleSizeIndex]
    totalSamples2 = sampleSize2[sampleSizeIndex]
    
    fishersResults = []
    gTestResults = []
    gTestYatesResults = []
    diffBetweenPropResults = []
    for r in xrange(0, numReplicates):
      if r % 1000 == 0:
        print r
      # draw samples w/o replacement for community 1
      samplesInSS = popSize*prop
      samplesNotInSS = popSize - samplesInSS
      inSS1 = 0
      for s in xrange(0, sampleSize1[sampleSizeIndex]):
        rnd = random.random()
        
        if rnd <= float(samplesInSS) / popSize:
          samplesInSS -= 1
          inSS1 += 1
        else:
          samplesNotInSS -= 1          
        
      # draw samples w/o replacement for community 2
      samplesInSS = popSize*prop
      samplesNotInSS = popSize - samplesInSS      
      inSS2 = 0
      for s in xrange(0, sampleSize2[sampleSizeIndex]):
        rnd = random.random()
        
        if rnd <= float(samplesInSS) / popSize:
          samplesInSS -= 1
          inSS2 += 1
        else:
          samplesNotInSS -= 1
                    
      fishersOneSided, fishersTwoSided = fishers.hypothesisTest(inSS1, inSS2, totalSamples1, totalSamples2)        
      gTestOneSided, gTestTwoSided = gTest.hypothesisTest(inSS1, inSS2, totalSamples1, totalSamples2)
      gTestYatesOneSided, gTestYatesTwoSided = gTestYates.hypothesisTest(inSS1, inSS2, totalSamples1, totalSamples2)
      diffBetweenPropOneSided, diffBetweenPropTwoSided = diffBetweenProp.hypothesisTest(inSS1, inSS2, totalSamples1, totalSamples2)
      
      fishersResults.append(fishersTwoSided)
      gTestResults.append(gTestTwoSided)
      gTestYatesResults.append(gTestYatesTwoSided)
      diffBetweenPropResults.append(diffBetweenPropTwoSided)   
      
    pValues = [fishersResults,gTestResults,gTestYatesResults,diffBetweenPropResults] 
      
    bins = {}
    numBins = 20
    binSize = 1.0 / numBins
    for i in xrange(0,numBins):
      bins[i] = []
      for j in xrange(0, len(pValues)):
        bins[i].append(0)
      
    for i in xrange(0, len(pValues)):
      for value in pValues[i]:
        binIndex = int(value / binSize)
        
        # values of exactly 1 are problematic
        if binIndex == numBins:
          binIndex = numBins - 1
          value = 0.9999999
          
        midBin = binSize*binIndex + 0.5*binSize
        
        offset = value - midBin
        
        if offset < 0:
          if binIndex - 1 >= 0:
            bins[binIndex][i] += 1.0 - (midBin - value) / (binSize)
            bins[binIndex - 1][i] += (midBin - value) / (binSize)
          else:
            bins[binIndex][i] += 1.0
        else:
          if binIndex + 1 < numBins:
            bins[binIndex][i] += 1.0 - (value - midBin) / (binSize)
            bins[binIndex + 1][i] += (value - midBin) / (binSize)
          else:
            bins[binIndex][i] += 1.0
    
    fout = open('alphaTest_' + str(prop) + '_' + str(sampleSize1[sampleSizeIndex]) +  '.csv' ,'w')
    fout.write('Bin,Fishers,G-test,G-test w/ Yates, Diff. b/w prop.\n')
    for i in xrange(0,numBins):
      fout.write(str(binSize*0.5 + i*binSize))
      for j in xrange(0, len(pValues)):
        fout.write(',' + str(float(bins[i][j]) / len(pValues[j])))
      fout.write('\n')
    fout.close()
      
print 'Done'
      