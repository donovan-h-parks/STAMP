#=======================================================================
# Author: Donovan Parks
#
# Dock widget containing customizable group legend.
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

from PyQt4 import QtGui, QtCore
from groupLegendDlgUI import Ui_GroupLegendDlg

class GroupLegendDlg(QtGui.QDockWidget):
	def __init__(self, preferences, parent=None, info=None):
		QtGui.QWidget.__init__(self, parent)

		# initialize GUI
		self.ui = Ui_GroupLegendDlg()
		self.ui.setupUi(self)
		
		self.groupColours = [QtGui.QColor(128, 177, 211), QtGui.QColor(253, 180, 98),
								QtGui.QColor(179, 222, 105), QtGui.QColor(190, 186, 218), 
								QtGui.QColor(141, 211, 199), QtGui.QColor(251, 128, 114),
								QtGui.QColor(252, 205, 229),QtGui.QColor(127,127,127),
								QtGui.QColor(188, 128, 189),QtGui.QColor(204, 235, 197)]

		self.groupColourDict = {}
		self.groupColourButtonsDict = {}
		
		self.ui.legendLayout = QtGui.QVBoxLayout(self.ui.scrollLegend)
		self.ui.legendLayout.setSizeConstraint(QtGui.QLayout.SetMinAndMaxSize)
		
		self.preferences = preferences
		
		self.currentField = ''
		
	def deleteItems(self, layout): 
		if layout is not None: 
			while layout.count(): 
				item = layout.takeAt(0) 
				widget = item.widget() 
				if widget is not None: 
					widget.deleteLater() 
				else: 
					self.deleteItems(item.layout()) 
		
	def initLegend(self, profileTree, metadata, field):
		self.profileTree = profileTree
		self.metadata = metadata
		
		if field != self.currentField:
			self.metadata.setActiveField(field, self.profileTree)
		self.currentField = field
		
		# remove any previous widgets
		self.deleteItems(self.ui.legendLayout)
			
		# add widgets
		self.groupColourDict = {}
		
		# add combo box
		horizontalLayout = QtGui.QHBoxLayout()
		lblGroupField = QtGui.QLabel(self.ui.dockWidgetContents)
		lblGroupField.setText('Group field: ')
		horizontalLayout.addWidget(lblGroupField)
		cboGroupField = QtGui.QComboBox(self.ui.dockWidgetContents)
		cboGroupField.setSizeAdjustPolicy(QtGui.QComboBox.AdjustToContents)
		
		for f in sorted(self.metadata.getFeatures()):
			cboGroupField.addItem(f)
		cboGroupField.setCurrentIndex(cboGroupField.findText(field))
			
		self.connect(cboGroupField, QtCore.SIGNAL('currentIndexChanged(QString)'), self.newGroupField)
		
		horizontalLayout.addWidget(cboGroupField)
		horizontalLayout.addStretch()
		self.ui.legendLayout.addLayout(horizontalLayout)
		
		# add horizontal line
		line = QtGui.QFrame(self.ui.dockWidgetContents)
		line.setFrameShape(QtGui.QFrame.HLine)
		line.setFrameShadow(QtGui.QFrame.Sunken)
		self.ui.legendLayout.addWidget(line)
		
		# add legend items
		index = 0
		for name in sorted(self.profileTree.groupDict.keys()):
			samples = set(sorted(self.profileTree.groupDict[name]))
			samples = list(samples.intersection(set(self.metadata.activeSamples)))
			
			tooltip = ''
			for i in xrange(0, len(samples)):
				tooltip += samples[i]
				if i != len(samples)-1:
					tooltip += ', '
			
			horizontalLayout = QtGui.QHBoxLayout()
			
			# force groups with no active samples to be inactive
			if len(samples) == 0:
				self.profileTree.groupActive[name] = False
			
			chkGroupActive = QtGui.QCheckBox(self.ui.dockWidgetContents)
			chkGroupActive.setChecked(self.profileTree.groupActive[name])
			chkGroupActive.setObjectName(name)
			chkGroupActive.setToolTip(tooltip)
			self.connect(chkGroupActive, QtCore.SIGNAL('toggled(bool)'), self.setGroupActive)
			horizontalLayout.addWidget(chkGroupActive)
			
			tbGroupColour = QtGui.QToolButton(self.ui.dockWidgetContents)
			tbGroupColour.setMinimumSize(QtCore.QSize(22, 22))
			tbGroupColour.setMaximumSize(QtCore.QSize(22, 22))
			tbGroupColour.setObjectName(name)
			tbGroupColour.setToolTip(tooltip)
			self.connect(tbGroupColour, QtCore.SIGNAL('clicked()'), self.setColour)
			horizontalLayout.addWidget(tbGroupColour)
			
			lblGroupName = QtGui.QLabel(self.ui.dockWidgetContents)
			lblGroupName.setText(name + ' (' + str(len(samples)) + ')')
			lblGroupName.setToolTip(tooltip)
			horizontalLayout.addWidget(lblGroupName)
			
			horizontalLayout.addStretch()
			self.ui.legendLayout.addLayout(horizontalLayout)
			
			if index == len(self.groupColours):
				self.groupColours.append(QtGui.QColor(0, 0, 0))
			
			colour = self.groupColours[index]
			self.groupColourDict[name] = colour
			self.groupColourButtonsDict[name] = tbGroupColour
			colourStr = str(colour.red()) + ',' + str(colour.green()) + ',' + str(colour.blue())
			tbGroupColour.setStyleSheet('* { background-color: rgb(' + colourStr + ') }')

			index += 1
			
		self.ui.legendLayout.addStretch()

		self.layout().update()
		
	def setGroupActive(self):
		sender = self.sender()
		if sender.toolTip() == '':	# make sure group has at least one sample
			self.disconnect(sender, QtCore.SIGNAL('toggled(bool)'), self.setGroupActive)
			sender.setChecked(False)
			self.connect(sender, QtCore.SIGNAL('toggled(bool)'), self.setGroupActive)
			QtGui.QMessageBox.information(None, 'Empty group', 'Groups with no active samples cannot be made active.', QtGui.QMessageBox.Warning)
			return

		self.profileTree.groupActive[str(sender.objectName())] = sender.isChecked()
		self.emit(QtCore.SIGNAL('legendActiveGroupsChanged()'))
		
	def setColour(self):
		sender = self.sender()
		
		colour = QtGui.QColorDialog.getColor(self.groupColourDict[str(sender.objectName())], self, 'Set group colour')
		
		colourMapIndex = sorted(self.profileTree.groupDict.keys()).index(str(sender.objectName()))
		self.groupColours[colourMapIndex] = colour
		
		if colour.isValid():
			self.updateLegend(str(sender.objectName()), colour)
			
	def updateLegend(self, groupName, colour):
		if self.groupColourDict:
			colourStr = str(colour.red()) + ',' + str(colour.green()) + ',' + str(colour.blue())
			self.groupColourButtonsDict[groupName].setStyleSheet('* { background-color: rgb(' + colourStr + ') }')
			
			self.groupColourDict[groupName] = colour
			self.preferences['Group colours'] = self.groupColourDict
			
			self.emit(QtCore.SIGNAL('legendItemChanged()'))
			
	def newGroupField(self, field):
		self.initLegend(self.profileTree, self.metadata, str(field))
		self.preferences['Group colours'] = self.groupColourDict
		self.emit(QtCore.SIGNAL('legendFieldChanged()'))

if __name__ == "__main__": 
	pass