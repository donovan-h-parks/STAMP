#=======================================================================
# Author: Donovan Parks
#
# Storey false discovery rate method.
#
# Copyright 2011 Donovan Parks
#
# This file is part of STAMP.
#
# STAMP is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# STAMP is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.  If not, see <http://www.gnu.org/licenses/>.
#=======================================================================

'''
Storey false discovery rate method.

Specifically, this implements the method discussed in Storey, 2003 except the proportion of features
that are truly null is estimated using the bootstrap procedure from Storey et al., 2004.

Storey JD and Tibshirani R. (2003) Statistical significance for genome-wide experiments. 
Proceeding of the National Academy of Sciences, 100: 9440-9445. 

Storey  JD,  Taylor  JE,  and  Siegmund  D.  (2004)  Strong  control,  conservative  point 
estimation, and simultaneous conservative consistency of false discovery rates: A unified 
approach. Journal of the Royal Statistical Society, Series B, 66:187-205. 
'''

from stamp.plugins.common.AbstractMultCompCorrection import AbstractMultCompCorrection

import numpy
import scipy

class StoreyFDR(AbstractMultCompCorrection):
  
  def __init__(self, preferences):
    AbstractMultCompCorrection.__init__(self, preferences)
    self.name = 'Storey FDR'
    self.method = 'False discovery rate'
    self.bCorrectedValues = True
    self.numSignFeatures = 0
    
  def correct(self, pValues, alpha):   
    numPvalues = len(pValues)

    # Find minimum pi0_hat value (i.e. proportion of features that are truly null)
    testPts = scipy.arange(0.0, 0.951, 0.05)
    min_pi0_hat = 1
    for testPt in testPts:
      numerator = [pValue for pValue in pValues if pValue > testPt]
      pi0 = float(len(numerator)) / (numPvalues*(1.0-testPt))
      if pi0 < min_pi0_hat:
        min_pi0_hat = pi0
        
    # Perform bootstrapping analyzes to estimate MSE of each pi0_hat
    bootstraps = 100
    minMSE = 1e100
    numpy.random.seed()
    for testPt in testPts:
      # calculate bootstrap pi0 values  
      mse = 0
      for dummy in xrange(0, bootstraps):
        bootstrapPvalues = []
        for i in xrange(0, numPvalues):
          rnd = numpy.random.randint(0, numPvalues)
          bootstrapPvalues.append(pValues[rnd])
        
        numerator = [pValue for pValue in bootstrapPvalues if pValue > testPt]
        bootstrap_pi0_hat = float(len(numerator)) / (numPvalues*(1.0-testPt))
        
        mse += (bootstrap_pi0_hat - min_pi0_hat)**2
        
      mse /= bootstraps
      
      if mse < minMSE:
        minMSE = mse
        selectedLambda = testPt
    
    numerator = [pValue for pValue in pValues if pValue > selectedLambda]
    self.estimated_pi0_hat = float(len(numerator)) / (numPvalues*(1.0-selectedLambda))
    
    # append an index identifier to each p-value
    indexedList = []
    index = 0
    for value in pValues:
      indexedList.append([value, index])
      index += 1
      
    # sort p-values in descending order
    indexedList.sort(reverse = True)

    # calculate q-values
    qValues = [None]*numPvalues
    qValues[indexedList[0][1]] = (self.estimated_pi0_hat * indexedList[0][0])
    
    for i in xrange(1,len(pValues)):
      qValues[indexedList[i][1]] =  min(self.estimated_pi0_hat*numPvalues*indexedList[i][0] / (numPvalues - i), qValues[indexedList[i-1][1]]) 
  
    self.numSignFeatures = len([x for x in qValues if x <= alpha])
    
    if self.estimated_pi0_hat < 0.01:
      try:
        from PyQt4 import QtGui
        QtGui.QMessageBox.information(None, 'Storey\'s FDR', 'P-values do not appear to be uniformly distributed. Consider using the Benjamini-Hochberg FDR approach.', QtGui.QMessageBox.Ok)
      except ImportError: 
        print 'Storey\'s FDR error: P-values do not appear to be uniformly distributed. Consider using the Benjamini-Hochberg FDR approach.'
        
    return qValues
  
  def additionalInfo(self):
    return [['Number of significant features', self.numSignFeatures],
            ['Estimated pi_0', self.estimated_pi0_hat]]
  
if __name__ == "__main__": 
  pass