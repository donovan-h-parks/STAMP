#=======================================================================
# Author: Donovan Parks
#
# Dialog box used to set program preferences.
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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP. If not, see <http://www.gnu.org/licenses/>.
#=======================================================================

from PyQt4 import QtGui, QtCore
from statsTableDlgUI import Ui_StatsTableDlg

from stamp.metagenomics.GenericTable import GenericTable

class StatsTableDlg(QtGui.QDockWidget):
	def __init__(self, preferences, parent=None, info=None):
		QtGui.QDockWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_StatsTableDlg()
		self.ui.setupUi(self)
		
		self.preferences = preferences
		self.table = ''
		
		# signals
		self.connect(self.ui.btnSave, QtCore.SIGNAL("clicked()"), self.saveTable)
		self.connect(self.ui.chkShowActiveFeatures, QtCore.SIGNAL("clicked()"), self.__updateTable)
		
	def updateTable(self, statsTest):
		self.statsTest = statsTest
		self.__updateTable()
		
	def __updateTable(self):
		if self.statsTest.results.profile != None:
			tableData, tableHeadings = self.statsTest.results.tableData(self.ui.chkShowActiveFeatures.isChecked())
			
			self.table = GenericTable(tableData, tableHeadings, self)
			self.table.sort(0,QtCore.Qt.AscendingOrder) # start with features in alphabetical order

			self.ui.tableStatisticalSummary.setModel(self.table)
			self.ui.tableStatisticalSummary.verticalHeader().setVisible(False)
			
			# resize columns to fit context by sampling first 100 rows
			#self.ui.tableStatisticalSummary.resizeColumnsToContents()
			for colIndex in xrange(0, self.table.columnCount(None)):
				fm = self.ui.tableStatisticalSummary.fontMetrics()
				maxWidth = fm.width(tableHeadings[colIndex]) + 10
				
				for i in xrange(0, 100): # sample first 100 rows to estimate column width, this is strictly for efficiency	
					width = fm.width(self.ui.tableStatisticalSummary.model().data(self.ui.tableStatisticalSummary.model().index(i,colIndex), QtCore.Qt.DisplayRole).toString()) + 10
					if  width > maxWidth:
						maxWidth = width
				
				self.ui.tableStatisticalSummary.setColumnWidth(colIndex, maxWidth)
		
	def saveTable(self):
		filename = QtGui.QFileDialog.getSaveFileName(self, 'Save table...', self.preferences['Last directory'],
									'Tab-separated values (*.tsv);;' +
									'Text file (*.txt);;' +
									'All files (*.*)')
		if filename != '':
			self.preferences['Last directory'] = filename[0:filename.lastIndexOf('/')]
			try:
				if self.table != '':
					self.table.save(filename)
			except IOError:
				QtGui.QMessageBox.information(self, 'Failed to save table', 'Write permission for file denied.', QtGui.QMessageBox.Ok)
		
if __name__ == "__main__": 
	pass