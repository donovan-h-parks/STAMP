# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'groupLegendDlg.ui'
#
# Created: Wed May 04 15:56:58 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt6 import QtCore, QtGui, QtWidgets

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_GroupLegendDlg(object):
    def setupUi(self, GroupLegendDlg):
        GroupLegendDlg.setObjectName(_fromUtf8("GroupLegendDlg"))
        GroupLegendDlg.resize(94, 91)
        sizePolicy = QtWidgets.QSizePolicy(QtWidgets.QSizePolicy.Policy.Minimum, QtWidgets.QSizePolicy.Policy.Minimum)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(GroupLegendDlg.sizePolicy().hasHeightForWidth())
        GroupLegendDlg.setSizePolicy(sizePolicy)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("icons:legend.png")), QtGui.QIcon.Mode.Normal, QtGui.QIcon.State.Off)
        GroupLegendDlg.setWindowIcon(icon)
        GroupLegendDlg.setFloating(True)
        GroupLegendDlg.setAllowedAreas(QtCore.Qt.DockWidgetArea.LeftDockWidgetArea|QtCore.Qt.DockWidgetArea.RightDockWidgetArea)
        self.dockWidgetContents = QtWidgets.QWidget()
        self.dockWidgetContents.setObjectName(_fromUtf8("dockWidgetContents"))
        self.verticalLayout = QtWidgets.QVBoxLayout(self.dockWidgetContents)
        self.verticalLayout.setContentsMargins(0, 0, 0, 0)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.scrollArea = QtWidgets.QScrollArea(self.dockWidgetContents)
        self.scrollArea.setFrameShape(QtWidgets.QFrame.Shape.NoFrame)
        self.scrollArea.setFrameShadow(QtWidgets.QFrame.Shadow.Plain)
        self.scrollArea.setWidgetResizable(True)
        self.scrollArea.setObjectName(_fromUtf8("scrollArea"))
        self.scrollLegend = QtWidgets.QWidget()
        self.scrollLegend.setGeometry(QtCore.QRect(0, 0, 94, 69))
        self.scrollLegend.setObjectName(_fromUtf8("scrollLegend"))
        self.scrollArea.setWidget(self.scrollLegend)
        self.verticalLayout.addWidget(self.scrollArea)
        GroupLegendDlg.setWidget(self.dockWidgetContents)

        self.retranslateUi(GroupLegendDlg)
        QtCore.QMetaObject.connectSlotsByName(GroupLegendDlg)

    def retranslateUi(self, GroupLegendDlg):
        GroupLegendDlg.setWindowTitle(QtWidgets.QApplication.translate("GroupLegendDlg", "Group legend", None))

import stamp.STAMP_rc
