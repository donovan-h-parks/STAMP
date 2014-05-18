#=======================================================================
# Author: Donovan Parks
#
# Calculate power of a hypothesis test.
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

import random
import math

from numpy import mean, std

class Power:
  
  def run(self, test, signLevel, statsResults, trials, bootstrapRep, progress):
     
    tableData = []
    index = 0
    for row in statsResults:                    
      feature = row[0]
      seq1 = row[1]
      seq2 = row[2]
      parentSeq1 = row[3]
      parentSeq2 = row[4]

      p1 = float(seq1) / parentSeq1
      p2 = float(seq2) / parentSeq2
    
      powerList = []  
      powerListLess5 = []  
      powerListGreater5 = []  
      for trial in xrange(0, trials): 
        if progress != '':
          index += 1
          progress.setValue(index)
          progress.setLabelText(feature + ' - Trial = ' + str(trial))   
          
        power = 0
        processedReplicates = 0
        for dummy in xrange(0, bootstrapRep):
          c1 = 0
          c2 = 0
          for dummy in xrange(0, parentSeq1):
            rnd = random.random()
            if rnd <= p1:
              c1 += 1
              
          for dummy in xrange(0, parentSeq2):
            rnd = random.random()
            if rnd <= p2:
              c2 += 1
      
          if c1 == 0 and c2 == 0:
            # This is a special case that many hypothesis test will not handle correctly
            # so we just ignore it. This will have little effect on the calculated power
            # of a test.
            continue
          
          processedReplicates += 1
          
          pValueOneSided, pValueTwoSided = test.hypothesisTest(c1, c2, parentSeq1, parentSeq2)
          if pValueTwoSided < signLevel:
            power += 1      
               
        if processedReplicates > 0:
          if min([seq1,seq2]) <= 5:
            powerListLess5.append(float(power) / processedReplicates)
          else:
            powerListGreater5.append(float(power) / processedReplicates)
            
          powerList.append(float(power) / processedReplicates)
  
      row = []
      row.append(feature)
      row.append(seq1)
      row.append(seq2)
      row.append(parentSeq1)
      row.append(parentSeq2)
      row.append(float(seq1) / parentSeq1)
      row.append(float(seq2) / parentSeq2)
      row.append(mean(powerList))
      row.append(std(powerList))
      
      if math.isnan(mean(powerListLess5)):
        row.append('')
      else:
        row.append(mean(powerListLess5))
        
      if math.isnan(std(powerListLess5)):
        row.append('')
      else:
        row.append(std(powerListLess5))
        
      if math.isnan(mean(powerListGreater5)):
        row.append('')
      else:
        row.append(mean(powerListGreater5))
        
      if math.isnan(std(powerListGreater5)):
        row.append('')
      else:
        row.append(std(powerListGreater5))

      tableData.append(row)
      
    return tableData

 
