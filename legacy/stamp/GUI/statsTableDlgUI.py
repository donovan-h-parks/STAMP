# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'statsTableDlg.ui'
#
# Created: Wed Jun 15 10:19:11 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_StatsTableDlg(object):
    def setupUi(self, StatsTableDlg):
        StatsTableDlg.setObjectName(_fromUtf8("StatsTableDlg"))
        StatsTableDlg.resize(738, 470)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/table.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        StatsTableDlg.setWindowIcon(icon)
        StatsTableDlg.setFloating(True)
        StatsTableDlg.setFeatures(QtGui.QDockWidget.AllDockWidgetFeatures)
        self.dockWidgetContents = QtGui.QWidget()
        self.dockWidgetContents.setObjectName(_fromUtf8("dockWidgetContents"))
        self.verticalLayout = QtGui.QVBoxLayout(self.dockWidgetContents)
        self.verticalLayout.setMargin(9)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.tableStatisticalSummary = QtGui.QTableView(self.dockWidgetContents)
        self.tableStatisticalSummary.setAlternatingRowColors(True)
        self.tableStatisticalSummary.setShowGrid(True)
        self.tableStatisticalSummary.setSortingEnabled(True)
        self.tableStatisticalSummary.setObjectName(_fromUtf8("tableStatisticalSummary"))
        self.verticalLayout.addWidget(self.tableStatisticalSummary)
        self.horizontalLayout = QtGui.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.chkShowActiveFeatures = QtGui.QCheckBox(self.dockWidgetContents)
        self.chkShowActiveFeatures.setChecked(False)
        self.chkShowActiveFeatures.setObjectName(_fromUtf8("chkShowActiveFeatures"))
        self.horizontalLayout.addWidget(self.chkShowActiveFeatures)
        spacerItem = QtGui.QSpacerItem(40, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.horizontalLayout.addItem(spacerItem)
        self.btnSave = QtGui.QPushButton(self.dockWidgetContents)
        self.btnSave.setObjectName(_fromUtf8("btnSave"))
        self.horizontalLayout.addWidget(self.btnSave)
        self.verticalLayout.addLayout(self.horizontalLayout)
        StatsTableDlg.setWidget(self.dockWidgetContents)

        self.retranslateUi(StatsTableDlg)
        QtCore.QMetaObject.connectSlotsByName(StatsTableDlg)

    def retranslateUi(self, StatsTableDlg):
        StatsTableDlg.setWindowTitle(QtGui.QApplication.translate("StatsTableDlg", "Statistical summary", None, QtGui.QApplication.UnicodeUTF8))
        self.chkShowActiveFeatures.setText(QtGui.QApplication.translate("StatsTableDlg", "Show only active features", None, QtGui.QApplication.UnicodeUTF8))
        self.btnSave.setText(QtGui.QApplication.translate("StatsTableDlg", "Save", None, QtGui.QApplication.UnicodeUTF8))

import stamp.STAMP_rc
