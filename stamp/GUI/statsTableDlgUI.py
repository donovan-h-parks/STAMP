# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'statsTableDlg.ui'
#
# Created: Wed Jun 15 10:19:11 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt6 import QtCore, QtGui, QtWidgets

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_StatsTableDlg(object):
    def setupUi(self, StatsTableDlg):
        StatsTableDlg.setObjectName(_fromUtf8("StatsTableDlg"))
        StatsTableDlg.resize(738, 470)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("icons:table.png")), QtGui.QIcon.Mode.Normal, QtGui.QIcon.State.Off)
        StatsTableDlg.setWindowIcon(icon)
        StatsTableDlg.setFloating(True)
        StatsTableDlg.setFeatures(QtWidgets.QDockWidget.DockWidgetFeature.DockWidgetClosable | QtWidgets.QDockWidget.DockWidgetFeature.DockWidgetMovable | QtWidgets.QDockWidget.DockWidgetFeature.DockWidgetFloatable)
        self.dockWidgetContents = QtWidgets.QWidget()
        self.dockWidgetContents.setObjectName(_fromUtf8("dockWidgetContents"))
        self.verticalLayout = QtWidgets.QVBoxLayout(self.dockWidgetContents)
        self.verticalLayout.setContentsMargins(9, 9, 9, 9)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.tableStatisticalSummary = QtWidgets.QTableView(self.dockWidgetContents)
        self.tableStatisticalSummary.setAlternatingRowColors(True)
        self.tableStatisticalSummary.setShowGrid(True)
        self.tableStatisticalSummary.setSortingEnabled(True)
        self.tableStatisticalSummary.setObjectName(_fromUtf8("tableStatisticalSummary"))
        self.verticalLayout.addWidget(self.tableStatisticalSummary)
        self.horizontalLayout = QtWidgets.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.chkShowActiveFeatures = QtWidgets.QCheckBox(self.dockWidgetContents)
        self.chkShowActiveFeatures.setChecked(False)
        self.chkShowActiveFeatures.setObjectName(_fromUtf8("chkShowActiveFeatures"))
        self.horizontalLayout.addWidget(self.chkShowActiveFeatures)
        spacerItem = QtWidgets.QSpacerItem(40, 20, QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Minimum)
        self.horizontalLayout.addItem(spacerItem)
        self.btnSave = QtWidgets.QPushButton(self.dockWidgetContents)
        self.btnSave.setObjectName(_fromUtf8("btnSave"))
        self.horizontalLayout.addWidget(self.btnSave)
        self.verticalLayout.addLayout(self.horizontalLayout)
        StatsTableDlg.setWidget(self.dockWidgetContents)

        self.retranslateUi(StatsTableDlg)
        QtCore.QMetaObject.connectSlotsByName(StatsTableDlg)

    def retranslateUi(self, StatsTableDlg):
        StatsTableDlg.setWindowTitle(QtWidgets.QApplication.translate("StatsTableDlg", "Statistical summary", None))
        self.chkShowActiveFeatures.setText(QtWidgets.QApplication.translate("StatsTableDlg", "Show only active features", None))
        self.btnSave.setText(QtWidgets.QApplication.translate("StatsTableDlg", "Save", None))

import stamp.STAMP_rc
