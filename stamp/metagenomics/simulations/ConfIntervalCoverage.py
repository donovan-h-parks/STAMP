'''
Evaluate performance of different CI methods.

@author: Donovan Parks
'''

from numpy import mean, std
from numpy.random import binomial

from stamp.metagenomics.plugins.confidenceIntervalMethods.DiffBetweenPropAsymptotic import DiffBetweenPropAsymptotic
from stamp.metagenomics.plugins.confidenceIntervalMethods.DiffBetweenPropAsymptoticCC import DiffBetweenPropAsymptoticCC
from stamp.metagenomics.plugins.confidenceIntervalMethods.NewcombeWilson import NewcombeWilson
from stamp.metagenomics.plugins.confidenceIntervalMethods.OddsRatio import OddsRatio
from stamp.metagenomics.plugins.confidenceIntervalMethods.NewcombeWilson import NewcombeWilson
from stamp.metagenomics.plugins.confidenceIntervalMethods.RatioProportions import RatioProportions

aMin = 5
aMax = 10
bMin = 1
bMax = 10
N1 = 10000
N2 = 20000

replicates = 1000
coverage = 0.95

preferences = {}
preferences['Pseudocount'] = 0.5

diffBetweenPropAsymptotic = DiffBetweenPropAsymptotic(preferences)
diffBetweenPropAsymptoticCC = DiffBetweenPropAsymptoticCC(preferences)
newcombeWilson = NewcombeWilson(preferences)
gart = OddsRatio(preferences)
ratioProportions = RatioProportions(preferences)

preferences2 = {}
preferences2['Pseudocount'] = 0.0
woolf = OddsRatio(preferences2)

# randomly create populations
fout = open('coverage_results.txt', 'w')

coverageListDP = [] 
coverageListDPCC = [] 
coverageListNW = [] 
coverageListWoolf = [] 
coverageListGart = [] 
coverageListRP = [] 

ciLengthDP = []
ciLengthDPCC = []
ciLengthNW = []
ciLengthWoolf = []
ciLengthGart = []
ciLengthRP = []
for a in xrange(aMin, aMax+1):
  print a
  
  for b in xrange(bMin, bMax+1):
    print '  ' + str(b)
    p1 = float(a) / N1
    p2 = float(b) / N2
    
    lowerCI, upperCI, obsDP = diffBetweenPropAsymptotic.run(a, b, N1, N2, coverage) 
    lowerCI, upperCI, obsDPCC = diffBetweenPropAsymptoticCC.run(a, b, N1, N2, coverage) 
    lowerCI, upperCI, obsNW = newcombeWilson.run(a, b, N1, N2, coverage) 
    lowerCI, upperCI, obsWoolf = woolf.run(a, b, N1, N2, coverage) 
    lowerCI, upperCI, obsGart = gart.run(a, b, N1, N2, coverage) 
    lowerCI, upperCI, obsRP = ratioProportions.run(a, b, N1, N2, coverage) 
        
    containedRepDP = 0    
    containedRepDPCC = 0   
    containedRepNW = 0   
    containedRepWoolf = 0   
    containedRepGart = 0   
    containedRepRP = 0   
    for r in xrange(0, replicates):
      c1 = binomial(N1, p1)
      while c1 == 0 or c1 == N1:
        c1 = binomial(N1, p1)
        
      c2 = binomial(N2, p2)
      while c2 == 0 or c2 == N2:
        c2 = binomial(N2, p2)
        
      # calculate CIs
      lowerCI, upperCI, effectSize = diffBetweenPropAsymptotic.run(c1, c2, N1, N2, coverage)
      if obsDP >= lowerCI and obsDP <= upperCI:
        containedRepDP += 1  
      ciLengthDP.append(upperCI - lowerCI)
      
      lowerCI, upperCI, effectSize = diffBetweenPropAsymptoticCC.run(c1, c2, N1, N2, coverage)
      if obsDPCC >= lowerCI and obsDPCC <= upperCI:
        containedRepDPCC += 1  
      ciLengthDPCC.append(upperCI - lowerCI)
      
      lowerCI, upperCI, effectSize = newcombeWilson.run(c1, c2, N1, N2, coverage)
      if obsNW >= lowerCI and obsNW <= upperCI:
        containedRepNW += 1  
      ciLengthNW.append(upperCI - lowerCI)
      
      lowerCI, upperCI, effectSize = woolf.run(c1, c2, N1, N2, coverage)
      if obsWoolf >= lowerCI and obsWoolf <= upperCI:
        containedRepWoolf += 1  
      ciLengthWoolf.append(upperCI - lowerCI)
      
      lowerCI, upperCI, effectSize = gart.run(c1, c2, N1, N2, coverage)
      if obsGart >= lowerCI and obsGart <= upperCI:
        containedRepGart += 1  
      ciLengthGart.append(upperCI - lowerCI)
      
      lowerCI, upperCI, effectSize = ratioProportions.run(c1, c2, N1, N2, coverage)
      if obsRP >= lowerCI and obsRP <= upperCI:
        containedRepRP += 1  
      ciLengthRP.append(upperCI - lowerCI)
            
    coverageListDP.append(float(containedRepDP) / replicates)
    coverageListDPCC.append(float(containedRepDPCC) / replicates)
    coverageListNW.append(float(containedRepNW) / replicates)
    coverageListWoolf.append(float(containedRepWoolf) / replicates)
    coverageListGart.append(float(containedRepGart) / replicates)
    coverageListRP.append(float(containedRepRP) / replicates)
     
results = [coverageListDP, coverageListDPCC, coverageListNW, coverageListWoolf, coverageListGart, coverageListRP]  
lengths = [ciLengthDP,ciLengthDPCC,ciLengthNW,ciLengthWoolf,ciLengthGart,ciLengthRP]   
methodNames = ['DP: Asymptotic', 'DP: Asymptotic-CC', 'Newcombe-Wilson', 'Woolf', 'Gart', 'RP: Asympototic']

for i in xrange(0, len(results)):
  coverageMeanStr = '%.2f' % mean(results[i])
  coverageSdStr = '%.2f' % std(results[i])
  coverageMinStr = '%.2f' % min(results[i])
  coverageMaxStr = '%.2f' % max(results[i])
  
  lengthMeanStr = '%.2f' % mean(lengths[i])
  lengthSdStr = '%.2f' % std(lengths[i])
     
  fout.write(methodNames[i] + '\n')
  fout.write(coverageMeanStr + '+/-' + coverageSdStr + '[' + coverageMinStr + ';' + coverageMaxStr + ']\n')
  fout.write(lengthMeanStr + '+/-' + lengthSdStr+ '\n')
  fout.write('\n')
  
fout.close()

    
    