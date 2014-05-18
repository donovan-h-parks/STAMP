from metagenomics.plugins.statisticalTests.Fishers import Fishers
from metagenomics.plugins.statisticalTests.DiffBetweenProp import DiffBetweenProp
from metagenomics.plugins.statisticalTests.Bootstrap import Bootstrap 
from metagenomics.plugins.statisticalTests.Barnard import Barnard 

fishers = Fishers()
diffBetweenProp = DiffBetweenProp()
bootstrap = Bootstrap()
barnard = Barnard()

a = 5
b = 3
totalSamples1 = 7
totalSamples2 = 7

fishersOneSided, fishersTwoSided = fishers.hypothesisTest(a, b, totalSamples1, totalSamples2)
diffBetweenPropOneSided, diffBetweenPropTwoSided = diffBetweenProp.hypothesisTest(a, b, totalSamples1, totalSamples2)
bootstrapOneSided, bootstrapTwoSided = bootstrap.hypothesisTest(a, b, totalSamples1, totalSamples2)
barnardOneSided, barnardTwoSided = barnard.hypothesisTest(a, b, totalSamples1, totalSamples2)

print diffBetweenPropTwoSided, bootstrapTwoSided, fishersTwoSided, barnardTwoSided
    
    