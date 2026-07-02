#=======================================================================
# Author: Donovan Parks
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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.	See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.	If not, see <http://www.gnu.org/licenses/>.
#=======================================================================

from PyQt6 import QtCore, QtGui, QtWidgets

from stamp.metagenomics.TableHelper import SortTableStrCol
from stamp.metagenomics.TableHelper import SortTableNumericStrCol
from stamp.metagenomics.StringHelper import isNumber

class GenericTable(QtCore.QAbstractTableModel): 
	def __init__(self, data, headers, parent=None, *args): 
		QtCore.QAbstractTableModel.__init__(self, parent, *args) 
		self.arraydata = data
		self.headerdata = headers
	
	def rowCount(self, parent): 
		return len(self.arraydata) 
	
	def columnCount(self, parent): 
		if len(self.arraydata) > 0:
			return len(self.arraydata[0]) 
		else:
			return -1
	
	def data(self, index, role): 
		if index.isValid() and role == QtCore.Qt.ItemDataRole.DisplayRole: 
			return self.arraydata[index.row()][index.column()]

		return None
	
	def headerData(self, col, orientation, role):
		if orientation == QtCore.Qt.Orientation.Horizontal and role == QtCore.Qt.ItemDataRole.DisplayRole:
			return self.headerdata[col]
		return None
	
	def sort(self, Ncol, order):
		'''
		Sort table by given column number.
		'''
		if len(self.arraydata) == 0:
			return
		
		self.layoutAboutToBeChanged.emit()
			
		dataIsNumeric = isNumber(self.arraydata[0][Ncol])
		
		if dataIsNumeric:
			self.arraydata = SortTableNumericStrCol(self.arraydata, Ncol)
		else:
			self.arraydata = SortTableStrCol(self.arraydata, Ncol)
				
		if order == QtCore.Qt.SortOrder.DescendingOrder:
			self.arraydata.reverse()
		self.layoutChanged.emit()
		
	def save(self, filename):
		fout = open(filename, 'w')
		for header in self.headerdata:
			fout.write(str(header) + '\t')
		fout.write('\n')
			
		for row in self.arraydata:
			for item in row:
				fout.write(str(item) + '\t')
			fout.write('\n')
			
		fout.close()
