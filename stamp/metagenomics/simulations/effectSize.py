
from stamp.plugins.statisticalTests.Fishers import Fishers

preferences = {}
fishers = Fishers(preferences)

sampleSizes = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000]

fout = open('effectSizeTest.txt' ,'w')

for sampleSize in sampleSizes:
  print sampleSize
  a = sampleSize / 2
  b = sampleSize / 2
  
  pValueTwoSided = 1.0
  while pValueTwoSided > 0.05:
    pValueOneSided, pValueTwoSided = fishers.hypothesisTest(a, b, sampleSize, sampleSize)
    a += 1
    
  p1 = float(a) / sampleSize
  p2 = float(b) / sampleSize
  
  fout.write(str(sampleSize) + ',' + str(p1-p2) + ',' + str(p1/p2) + ',' + str(pValueTwoSided) + ',' + str(a) + '\n')
