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

import operator
import math

from PyQt6 import QtCore, QtGui, QtWidgets

def SortTable(table, cols, bAscending = True, bAbsoluteValue = False, bLog = False):
	''' 
	Sort a table by multiple columns.
			table: a list of lists (or tuple of tuples) where each inner list 
						 represents a row
			cols:	a list (or tuple) specifying the column numbers to sort by
						 e.g. (1,0) would sort by column 1, then by column 0
			bAscending: flag indicating if column should be sorted in ascending or
									descending order
			bAbsoluteValue: flag indicating if sorting should be based on the absolute
											value of each element in the column
			bLog: flag indicating if log of value should be taken before sorting
	'''
	
	for col in reversed(cols):
		# py2 used sorted(table, cmp, key=itemgetter(col)); the cmp compared the
		# key-extracted column values, so each is equivalent to ordering by that
		# monotonic transform of the column value.
		if bLog and bAbsoluteValue:
			keyFunc = lambda row, c=col: abs(math.log10(row[c]))
		elif bLog and not bAbsoluteValue:
			keyFunc = lambda row, c=col: math.log10(row[c])
		elif not bLog and bAbsoluteValue:
			keyFunc = lambda row, c=col: abs(row[c])
		else:
			keyFunc = operator.itemgetter(col)
		table = sorted(table, key = keyFunc, reverse = (not bAscending))
			
	return table

def SortTableStrCol(table, col, bAscending = True):
	''' 
	Sort a table column so it is in alphabetical order
			table: a list of lists (or tuple of tuples) where each inner list 
						 represents a row
			col:	a column numbers to sort by
			bAscending: flag indicating if column should be sorted in ascending or
									descending order
	'''
	
	table = sorted(table, key = lambda row: row[col].lower(), reverse = (not bAscending))
			
	return table
	
def GetStrNumber(numberStr):
	try:
		num = float(numberStr)
		return num
	except ValueError:
		return 0.0
	
def SortTableNumericStrCol(table, col, bAscending = True):
	''' 
	Sort a table column so it is in alphabetical order
			table: a list of lists (or tuple of tuples) where each inner list 
						 represents a row
			col:	a column numbers to sort by
			bAscending: flag indicating if column should be sorted in ascending or
									descending order
	'''
	table = sorted(table, key = lambda row: GetStrNumber(row[col]), reverse = (not bAscending))
			
	return table

class QTableWidgetNumericItem(QtWidgets.QTableWidgetItem):
	'''
	Sort table columns in numeric order.
	'''
	
	def __init__(self, text):
		QtWidgets.QTableWidgetItem.__init__(self, text, QtWidgets.QTableWidgetItem.ItemType.UserType)

	def __lt__(self, other):
		return float(self.text()) < float(other.text())
