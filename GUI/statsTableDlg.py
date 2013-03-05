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

from metagenomics.GenericTable import GenericTable

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
			self.ui.tableStatisticalSummary.resizeColumnsToContents()
		
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