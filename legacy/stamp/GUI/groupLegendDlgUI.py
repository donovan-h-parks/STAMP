# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'groupLegendDlg.ui'
#
# Created: Wed May 04 15:56:58 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_GroupLegendDlg(object):
    def setupUi(self, GroupLegendDlg):
        GroupLegendDlg.setObjectName(_fromUtf8("GroupLegendDlg"))
        GroupLegendDlg.resize(94, 91)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Minimum, QtGui.QSizePolicy.Minimum)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(GroupLegendDlg.sizePolicy().hasHeightForWidth())
        GroupLegendDlg.setSizePolicy(sizePolicy)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/legend.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        GroupLegendDlg.setWindowIcon(icon)
        GroupLegendDlg.setFloating(True)
        GroupLegendDlg.setAllowedAreas(QtCore.Qt.LeftDockWidgetArea|QtCore.Qt.RightDockWidgetArea)
        self.dockWidgetContents = QtGui.QWidget()
        self.dockWidgetContents.setObjectName(_fromUtf8("dockWidgetContents"))
        self.verticalLayout = QtGui.QVBoxLayout(self.dockWidgetContents)
        self.verticalLayout.setMargin(0)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.scrollArea = QtGui.QScrollArea(self.dockWidgetContents)
        self.scrollArea.setFrameShape(QtGui.QFrame.NoFrame)
        self.scrollArea.setFrameShadow(QtGui.QFrame.Plain)
        self.scrollArea.setWidgetResizable(True)
        self.scrollArea.setObjectName(_fromUtf8("scrollArea"))
        self.scrollLegend = QtGui.QWidget()
        self.scrollLegend.setGeometry(QtCore.QRect(0, 0, 94, 69))
        self.scrollLegend.setObjectName(_fromUtf8("scrollLegend"))
        self.scrollArea.setWidget(self.scrollLegend)
        self.verticalLayout.addWidget(self.scrollArea)
        GroupLegendDlg.setWidget(self.dockWidgetContents)

        self.retranslateUi(GroupLegendDlg)
        QtCore.QMetaObject.connectSlotsByName(GroupLegendDlg)

    def retranslateUi(self, GroupLegendDlg):
        GroupLegendDlg.setWindowTitle(QtGui.QApplication.translate("GroupLegendDlg", "Group legend", None, QtGui.QApplication.UnicodeUTF8))

import stamp.STAMP_rc
