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
from metadataTableDlgUI import Ui_MetadataTableDlg

from stamp.metagenomics.TableHelper import QTableWidgetNumericItem

class MetadataTableDlg(QtGui.QDockWidget):
	def __init__(self, preferences, parent=None, info=None):
		QtGui.QDockWidget.__init__(self, parent)
		
		# initialize GUI
		self.ui = Ui_MetadataTableDlg()
		self.ui.setupUi(self)
		
		# setup signals
		self.connect(self.ui.tbMetadataAddAll, QtCore.SIGNAL('clicked(bool)'), self.checkAll)
		self.connect(self.ui.tbMetadataRemoveAll, QtCore.SIGNAL('clicked(bool)'), self.uncheckAll)
		self.connect(self.ui.tbMetadataFilter, QtCore.SIGNAL('clicked(bool)'), self.filter)
		self.connect(self.ui.cboMetadataField, QtCore.SIGNAL('currentIndexChanged(int)'), self.setValues)
		
		self.preferences = preferences
		self.table = ''
		
		self.metadata = None
		
	def checkAll(self):
		for r in xrange(0, self.ui.tableMetadata.rowCount()):
			self.ui.tableMetadata.item(r,0).setCheckState(QtCore.Qt.Checked)
		self.updateActiveSamples()
			
	def uncheckAll(self):
		for r in xrange(0, self.ui.tableMetadata.rowCount()):
			self.ui.tableMetadata.item(r,0).setCheckState(QtCore.Qt.Unchecked)
		self.updateActiveSamples()
			
	def checkSpecifiedSamples(self, sampleIds):
		for r in xrange(0, self.ui.tableMetadata.rowCount()):
			if str(self.ui.tableMetadata.item(r,0).text()) in sampleIds:
				self.ui.tableMetadata.item(r,0).setCheckState(QtCore.Qt.Checked)
		self.updateActiveSamples()
				
	def uncheckSpecifiedSamples(self, sampleIds):
		for r in xrange(0, self.ui.tableMetadata.rowCount()):
			if str(self.ui.tableMetadata.item(r,0).text()) in sampleIds:
				self.ui.tableMetadata.item(r,0).setCheckState(QtCore.Qt.Unchecked)
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
			values.sort(lambda x, y: cmp(float(x), float(y)))
		else:
			values.sort(lambda x, y: cmp(x.lower(),y.lower()))

		self.ui.cboMetadataValue.clear()
		self.ui.cboMetadataValue.addItems(values)
		
		isNumeric = self.metadata.isNumericalData(field)
		
	def itemClicked(self, item):
		if item.column() == 0:
			self.updateActiveSamples()

	def updateActiveSamples(self):
		activeSamples = []
		for r in xrange(0, self.ui.tableMetadata.rowCount()):
			if self.ui.tableMetadata.item(r,0).checkState() == QtCore.Qt.Checked:
				activeSamples.append(str(self.ui.tableMetadata.item(r,0).text()))
				
		self.metadata.activeSamples = activeSamples
		
		self.emit(QtCore.SIGNAL('activeSamplesChanged()'))

	def setTable(self, metadata):
		if metadata != None:
			self.disconnect(self.ui.tableMetadata, QtCore.SIGNAL('itemClicked(QTableWidgetItem*)'), self.itemClicked)
			
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
			
			for i in xrange(0, len(table)):
				row = table[i]

				for j in xrange(0, len(row)):
					if isNumeric[j]:
						item = QTableWidgetNumericItem(row[j])
					else:
						item = QtGui.QTableWidgetItem(row[j])

					if j == 0:
						item.setTextAlignment(QtCore.Qt.AlignLeft | QtCore.Qt.AlignVCenter)
						item.setCheckState(QtCore.Qt.Checked)
					else:
						item.setTextAlignment(QtCore.Qt.AlignHCenter | QtCore.Qt.AlignVCenter)
						
					self.ui.tableMetadata.setItem(i, j, item)
						
			self.ui.tableMetadata.resizeColumnsToContents()
			self.connect(self.ui.tableMetadata, QtCore.SIGNAL('itemClicked(QTableWidgetItem*)'), self.itemClicked)

if __name__ == "__main__": 
	pass