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

from PyQt6 import QtCore, QtGui, QtWidgets
from stamp.GUI.metadataTableDlgUI import Ui_MetadataTableDlg

from stamp.metagenomics.TableHelper import QTableWidgetNumericItem

class MetadataTableDlg(QtWidgets.QDockWidget):
	activeSamplesChanged = QtCore.pyqtSignal()

	def __init__(self, preferences, parent=None, info=None):
		QtWidgets.QDockWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_MetadataTableDlg()
		self.ui.setupUi(self)
		
		# setup signals
		self.ui.tbMetadataAddAll.clicked.connect(self.checkAll)
		self.ui.tbMetadataRemoveAll.clicked.connect(self.uncheckAll)
		self.ui.tbMetadataFilter.clicked.connect(self.filter)
		self.ui.cboMetadataField.currentIndexChanged.connect(self.setValues)
		
		self.preferences = preferences
		self.table = ''
		
		self.metadata = None
		
	def checkAll(self):
		for r in range(0, self.ui.tableMetadata.rowCount()):
			self.ui.tableMetadata.item(r,0).setCheckState(QtCore.Qt.CheckState.Checked)
		self.updateActiveSamples()
			
	def uncheckAll(self):
		for r in range(0, self.ui.tableMetadata.rowCount()):
			self.ui.tableMetadata.item(r,0).setCheckState(QtCore.Qt.CheckState.Unchecked)
		self.updateActiveSamples()
			
	def checkSpecifiedSamples(self, sampleIds):
		for r in range(0, self.ui.tableMetadata.rowCount()):
			if str(self.ui.tableMetadata.item(r,0).text()) in sampleIds:
				self.ui.tableMetadata.item(r,0).setCheckState(QtCore.Qt.CheckState.Checked)
		self.updateActiveSamples()
				
	def uncheckSpecifiedSamples(self, sampleIds):
		for r in range(0, self.ui.tableMetadata.rowCount()):
			if str(self.ui.tableMetadata.item(r,0).text()) in sampleIds:
				self.ui.tableMetadata.item(r,0).setCheckState(QtCore.Qt.CheckState.Unchecked)
		self.updateActiveSamples()
			
	def filter(self):
		addRemove = str(self.ui.cboMetadataAddRemove.currentText())
		field = str(self.ui.cboMetadataField.currentText())
		relationship = str(self.ui.cboMetadataRelationship.currentText())
		value = str(self.ui.cboMetadataValue.currentText())
		
		isNumeric = self.metadata.isNumericalData(field)
		
		sampleIds = []
		for sample in self.metadata.getSampleNames():
			if isNumeric:
				if relationship == '>' and float(self.metadata.getValue(sample, field)) > float(value):
					sampleIds.append(sample)
				elif relationship == '=' and float(self.metadata.getValue(sample, field)) == float(value):
					sampleIds.append(sample)
				elif relationship == '<' and float(self.metadata.getValue(sample, field)) < float(value):
					sampleIds.append(sample)
			else:
				if relationship == '>' and self.metadata.getValue(sample, field) > value:
					sampleIds.append(sample)
				elif relationship == '=' and self.metadata.getValue(sample, field) == value:
					sampleIds.append(sample)
				elif relationship == '<' and self.metadata.getValue(sample, field) < value:
					sampleIds.append(sample)
					
		if addRemove == 'Add':
			self.checkSpecifiedSamples(sampleIds)
		else:
			self.uncheckSpecifiedSamples(sampleIds)
		
	def setFields(self, fields):
		self.ui.cboMetadataField.clear()
		self.ui.cboMetadataField.addItems(fields)
		self.ui.cboMetadataField.updateGeometry()
		
	def setValues(self):
		field = str(self.ui.cboMetadataField.currentText())
		values = self.metadata.getUniqueValues(field)

		if self.metadata.isNumericalData(field):
			values.sort(key=lambda x: float(x))
		else:
			values.sort(key=lambda x: x.lower())

		self.ui.cboMetadataValue.clear()
		self.ui.cboMetadataValue.addItems(values)
		
		isNumeric = self.metadata.isNumericalData(field)
		
	def itemClicked(self, item):
		if item.column() == 0:
			self.updateActiveSamples()

	def updateActiveSamples(self):
		activeSamples = []
		for r in range(0, self.ui.tableMetadata.rowCount()):
			if self.ui.tableMetadata.item(r,0).checkState() == QtCore.Qt.CheckState.Checked:
				activeSamples.append(str(self.ui.tableMetadata.item(r,0).text()))
				
		self.metadata.activeSamples = activeSamples
		
		self.activeSamplesChanged.emit()

	def setTable(self, metadata):
		if metadata != None:
			try:
				self.ui.tableMetadata.itemClicked.disconnect(self.itemClicked)
			except TypeError:
				pass  # not connected yet (first call); PyQt6 raises instead of returning False
			
			self.metadata = metadata
			
			table, headers = metadata.getTableData()
			
			self.setFields(headers[1:])

			self.ui.tableMetadata.clear()
			self.ui.tableMetadata.horizontalHeader().show()
			self.ui.tableMetadata.setColumnCount(len(headers))
			self.ui.tableMetadata.setHorizontalHeaderLabels(headers)
			self.ui.tableMetadata.setRowCount(len(table))
			self.ui.tableMetadata.verticalHeader().hide()
			
			isNumeric = [False]
			for field in headers[1:]:
				isNumeric.append(self.metadata.isNumericalData(field))
			
			for i in range(0, len(table)):
				row = table[i]

				for j in range(0, len(row)):
					if isNumeric[j]:
						item = QTableWidgetNumericItem(row[j])
					else:
						item = QtWidgets.QTableWidgetItem(row[j])

					if j == 0:
						item.setTextAlignment(QtCore.Qt.AlignmentFlag.AlignLeft | QtCore.Qt.AlignmentFlag.AlignVCenter)
						item.setCheckState(QtCore.Qt.CheckState.Checked)
					else:
						item.setTextAlignment(QtCore.Qt.AlignmentFlag.AlignHCenter | QtCore.Qt.AlignmentFlag.AlignVCenter)
						
					self.ui.tableMetadata.setItem(i, j, item)
						
			self.ui.tableMetadata.resizeColumnsToContents()
			self.ui.tableMetadata.itemClicked.connect(self.itemClicked)

if __name__ == "__main__": 
	pass