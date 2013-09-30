#=======================================================================
# Author: Donovan Parks
#
# Calculate coverage of a confidence interval.
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

import math

from numpy import mean, std
from numpy.random import binomial

class ConfIntervCoverage:
  
  def run(self, confIntervMethod, coverage, tables, trials, bootstrapRep, progress):
  
    tableData = []
    index = 0
    for row in tables:                    
      feature = row[0]
      seq1 = row[1]
      seq2 = row[2]
      parentSeq1 = row[3]
      parentSeq2 = row[4]
    
      lowerCI, upperCI, obsEffectSize = confIntervMethod.run(seq1, seq2, parentSeq1, parentSeq2, coverage) 
    
      p1 = float(seq1) / parentSeq1
      p2 = float(seq2) / parentSeq2
    
      coverageList = []  
      coverageListLess5 = []  
      coverageListGreater5 = []  
      for trial in xrange(0, trials): 
        if progress != '':
          index += 1
          progress.setValue(index)
          progress.setLabelText(feature + ' - Trial = ' + str(trial))  
          
        containedRep = 0
        for dummy in xrange(0, bootstrapRep):
          c1 = binomial(parentSeq1, p1)
          c2 = binomial(parentSeq2, p2)
      
          lowerCI, upperCI, effectSize = confIntervMethod.run(c1, c2, parentSeq1, parentSeq2, coverage)
          if obsEffectSize >= lowerCI and obsEffectSize <= upperCI:
            containedRep += 1        
               
        if min([seq1,seq2]) <= 5:
          coverageListLess5.append(float(containedRep) / bootstrapRep)
        else:
          coverageListGreater5.append(float(containedRep) / bootstrapRep)
          
        coverageList.append(float(containedRep) / bootstrapRep)
  
      row = []
      row.append(feature)
      row.append(seq1)
      row.append(seq2)
      row.append(parentSeq1)
      row.append(parentSeq2)
      row.append(float(seq1) / parentSeq1)
      row.append(float(seq2) / parentSeq2)
      row.append(mean(coverageList))
      row.append(std(coverageList))
      
      if math.isnan(mean(coverageListLess5)):
        row.append('')
      else:
        row.append(mean(coverageListLess5))
        
      if math.isnan(std(coverageListLess5)):
        row.append('')
      else:
        row.append(std(coverageListLess5))
        
      if math.isnan(mean(coverageListGreater5)):
        row.append('')
      else:
        row.append(mean(coverageListGreater5))
        
      if math.isnan(std(coverageListGreater5)):
        row.append('')
      else:
        row.append(std(coverageListGreater5))

      tableData.append(row)
      
    return tableData

 
