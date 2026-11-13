import math
import random

from stamp.plugins.statisticalTests.Fishers import Fishers
from stamp.plugins.statisticalTests.Hypergeometric import Hypergeometric

fishers = Fishers({})
hypergeometric = Hypergeometric({})

C1s = [1000, 10000, 100000]
maxX1 = 200

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
  
fout = open('FishersTest.txt','w')

random.seed()

for C1 in C1s:
  print C1
  totalSamples1 = C1
  totalSamples2 = C1*2

  relativelErrors = {}
  for a in xrange(0, maxX1):
    print a
    for b in xrange(a, maxX1):        
      # calculate p-values
      fishersOneSided, fishersTwoSided = fishers.hypothesisTest(a, b, totalSamples1, totalSamples2)
      hypergeometricOneSided, hypergeometricTwoSided = hypergeometric.hypothesisTest(a, b, totalSamples1, totalSamples2)
      relativeError = 100 * (hypergeometricTwoSided - fishersTwoSided) / fishersTwoSided
      if fishersTwoSided < 1e-6:
        l = relativelErrors.get(1e-6, [])
        l.append(relativeError)
        relativelErrors[1e-6] = l
      elif fishersTwoSided < 1e-5:
        l = relativelErrors.get(1e-5, [])
        l.append(relativeError)
        relativelErrors[1e-5] = l
      elif fishersTwoSided < 1e-4:
        l = relativelErrors.get(1e-4, [])
        l.append(relativeError)
        relativelErrors[1e-4] = l
      elif fishersTwoSided < 1e-3:
        l = relativelErrors.get(1e-3, [])
        l.append(relativeError)
        relativelErrors[1e-3] = l
      elif fishersTwoSided < 1e-2:
        l = relativelErrors.get(1e-2, [])
        l.append(relativeError)
        relativelErrors[1e-2] = l
      elif fishersTwoSided < 1e-1:
        l = relativelErrors.get(1e-1, [])
        l.append(relativeError)
        relativelErrors[1e-1] = l

  fout.write('N: ' + str(C1) + '\n')
  for key in sorted(relativelErrors.keys()):
    fout.write('Evaluated tables: ' + str(len(relativelErrors[key])) + '\n')        
    meanStr = "%.2f" % mean(relativelErrors[key])
    sdStr = "%.2f" % stdDev(relativelErrors[key])      
    minStr= "%.2f" % min(relativelErrors[key])
    maxStr = "%.2f" % max(relativelErrors[key])
      
    fout.write(str(key) + ': ' + meanStr + '+/-' + sdStr + '%[' + minStr + '%; ' + maxStr + '%]\n')
    fout.write('\n')

fout.close()
    