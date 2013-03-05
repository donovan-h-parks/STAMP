#=======================================================================
# Author: Donovan Parks
#
# Stores profile information for two samples.
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

class SampleProfileEntry:
  def __init__(self):
    self.hierarchy = []
    self.featureCounts = []
    self.parentCounts = []
  
class SampleProfile:
  def __init__(self):
    self.hierarchyHeadings = []
    self.sampleNames = []

    self.parentHeading = None
    self.profileHeading = None
    
    self.profileDict = {}
    
    self.numParentCategories = 0
        
  def getFeatures(self):
    return self.profileDict.keys()
    
  def getNumFeatures(self):
    return len(self.profileDict)
  
  def getNumParentCategories(self):
    return self.numParentCategories
    
  def getData(self, feature):
    return self.profileDict[feature]
    
  def getTableData(self, feature):
    data = self.profileDict[feature]
    seq1, seq2 = data.featureCounts
    parentSeq1, parentSeq2 = data.parentCounts
    return [seq1, seq2, parentSeq1, parentSeq2]
  
  def getLabeledTables(self):
    tables = [[feature] + self.getTableData(feature) for feature in self.profileDict.keys()]     
    return tables

  def getFeatureCounts(self, feature):
    return self.profileDict[feature].featureCounts
  
  def getParentCounts(self, feature):
    return self.profileDict[feature].parentCounts
  
  def getHierarchy(self, feature):
    return self.profileDict[feature].hierarchy
  
  def getSequenceCounts(self, sampleNum, bParentSeqCout = False):     
    data = []
    if bParentSeqCout:
      for feature in self.profileDict:
        data.append(self.profileDict[feature].parentCounts[sampleNum])
    else:
      for feature in self.profileDict:
        data.append(self.profileDict[feature].featureCounts[sampleNum])
           
    return data
    
    