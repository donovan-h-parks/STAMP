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

# =======================================================================
# Author: Donovan Parks / Python 3 conversion
# =======================================================================

from PyQt5 import QtGui, QtCore, QtWidgets
from stamp.GUI.groupLegendDlgUI import Ui_GroupLegendDlg


class GroupLegendDlg(QtWidgets.QDockWidget):
	# Signals MUST be defined at the class level in PyQt5
	legendItemChanged = QtCore.pyqtSignal()
	legendFieldChanged = QtCore.pyqtSignal()
	legendActiveGroupsChanged = QtCore.pyqtSignal()

	def __init__(self, preferences, parent=None, info=None):
		# Correctly call QDockWidget (the parent class) init
		super(GroupLegendDlg, self).__init__(parent)

		# Initialize GUI
		self.ui = Ui_GroupLegendDlg()
		self.ui.setupUi(self)

		self.groupColours = [
			QtGui.QColor(128, 177, 211), QtGui.QColor(253, 180, 98),
			QtGui.QColor(179, 222, 105), QtGui.QColor(190, 186, 218),
			QtGui.QColor(141, 211, 199), QtGui.QColor(251, 128, 114),
			QtGui.QColor(252, 205, 229), QtGui.QColor(127, 127, 127),
			QtGui.QColor(188, 128, 189), QtGui.QColor(204, 235, 197)
		]

		self.groupColourDict = {}
		self.groupColourButtonsDict = {}

		# Setup layout for scroll area
		self.ui.legendLayout = QtWidgets.QVBoxLayout(self.ui.scrollLegend)
		self.ui.legendLayout.setSizeConstraint(QtWidgets.QLayout.SetMinAndMaxSize)

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

		# reset dictionaries
		self.groupColourDict = {}
		self.groupColourButtonsDict = {}

		# Group field selection layout
		horizontalLayout = QtWidgets.QHBoxLayout()
		lblGroupField = QtWidgets.QLabel(self.ui.dockWidgetContents)
		lblGroupField.setText('Group field: ')
		horizontalLayout.addWidget(lblGroupField)

		cboGroupField = QtWidgets.QComboBox(self.ui.dockWidgetContents)
		cboGroupField.setSizeAdjustPolicy(QtWidgets.QComboBox.AdjustToContents)

		for f in sorted(self.metadata.getFeatures()):
			cboGroupField.addItem(f)
		cboGroupField.setCurrentIndex(cboGroupField.findText(field))

		# PyQt5 New-style connection
		cboGroupField.currentTextChanged.connect(self.newGroupField)

		horizontalLayout.addWidget(cboGroupField)
		horizontalLayout.addStretch()
		self.ui.legendLayout.addLayout(horizontalLayout)

		# Horizontal line
		line = QtWidgets.QFrame(self.ui.dockWidgetContents)
		line.setFrameShape(QtWidgets.QFrame.HLine)
		line.setFrameShadow(QtWidgets.QFrame.Sunken)
		self.ui.legendLayout.addWidget(line)

		# Add legend items
		index = 0
		sorted_groups = sorted(self.profileTree.groupDict.keys())

		for name in sorted_groups:
			samples = set(self.profileTree.groupDict[name])
			samples = list(samples.intersection(set(self.metadata.activeSamples)))

			tooltip = ', '.join(samples)

			horizontalLayout = QtWidgets.QHBoxLayout()

			# force groups with no active samples to be inactive
			if len(samples) == 0:
				self.profileTree.groupActive[name] = False

			chkGroupActive = QtWidgets.QCheckBox(self.ui.dockWidgetContents)
			chkGroupActive.setChecked(self.profileTree.groupActive.get(name, False))
			chkGroupActive.setObjectName(name)
			chkGroupActive.setToolTip(tooltip)
			# New-style connection
			chkGroupActive.toggled.connect(self.setGroupActive)
			horizontalLayout.addWidget(chkGroupActive)

			tbGroupColour = QtWidgets.QToolButton(self.ui.dockWidgetContents)
			tbGroupColour.setMinimumSize(QtCore.QSize(22, 22))
			tbGroupColour.setMaximumSize(QtCore.QSize(22, 22))
			tbGroupColour.setObjectName(name)
			tbGroupColour.setToolTip(tooltip)
			# New-style connection
			tbGroupColour.clicked.connect(self.setColour)
			horizontalLayout.addWidget(tbGroupColour)

			lblGroupName = QtWidgets.QLabel(self.ui.dockWidgetContents)
			lblGroupName.setText(f"{name} ({len(samples)})")
			lblGroupName.setToolTip(tooltip)
			horizontalLayout.addWidget(lblGroupName)

			horizontalLayout.addStretch()
			self.ui.legendLayout.addLayout(horizontalLayout)

			if index >= len(self.groupColours):
				self.groupColours.append(QtGui.QColor(0, 0, 0))

			colour = self.groupColours[index]
			self.groupColourDict[name] = colour
			self.groupColourButtonsDict[name] = tbGroupColour

			colourStr = f"{colour.red()},{colour.green()},{colour.blue()}"
			tbGroupColour.setStyleSheet(f"background-color: rgb({colourStr});")

			index += 1

		self.ui.legendLayout.addStretch()
		self.layout().update()

	def setGroupActive(self):
		sender = self.sender()
		if not sender.toolTip():  # check if tooltip is empty (no samples)
			sender.blockSignals(True)
			sender.setChecked(False)
			sender.blockSignals(False)
			QtWidgets.QMessageBox.warning(None, 'Empty group', 'Groups with no active samples cannot be made active.')
			return

		group_name = sender.objectName()
		self.profileTree.groupActive[group_name] = sender.isChecked()
		self.legendActiveGroupsChanged.emit()

	def setColour(self):
		sender = self.sender()
		group_name = sender.objectName()

		initial_color = self.groupColourDict.get(group_name, QtGui.QColor(0, 0, 0))
		colour = QtWidgets.QColorDialog.getColor(initial_color, self, 'Set group colour')

		if colour.isValid():
			# Update the list and the dict
			sorted_groups = sorted(self.profileTree.groupDict.keys())
			if group_name in sorted_groups:
				colourMapIndex = sorted_groups.index(group_name)
				self.groupColours[colourMapIndex] = colour
				self.updateLegend(group_name, colour)

	def updateLegend(self, groupName, colour):
		if self.groupColourDict:
			colourStr = f"{colour.red()},{colour.green()},{colour.blue()}"
			self.groupColourButtonsDict[groupName].setStyleSheet(f"background-color: rgb({colourStr});")

			self.groupColourDict[groupName] = colour
			self.preferences['Group colours'] = self.groupColourDict
			self.legendItemChanged.emit()

	def newGroupField(self, field):
		self.initLegend(self.profileTree, self.metadata, field)
		self.preferences['Group colours'] = self.groupColourDict
		self.legendFieldChanged.emit()


if __name__ == "__main__":
	pass