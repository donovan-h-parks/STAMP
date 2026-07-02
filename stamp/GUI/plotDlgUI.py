# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'plotDlg.ui'
#
# Created: Mon Jan 02 11:48:31 2012
#      by: PyQt4 UI code generator 4.8.5
#
# WARNING! All changes made in this file will be lost!

from PyQt6 import QtCore, QtGui, QtWidgets

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_PlotDlg(object):
    def setupUi(self, PlotDlg):
        PlotDlg.setObjectName(_fromUtf8("PlotDlg"))
        PlotDlg.resize(600, 400)
        sizePolicy = QtWidgets.QSizePolicy(QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Expanding)
        sizePolicy.setHorizontalStretch(1)
        sizePolicy.setVerticalStretch(1)
        sizePolicy.setHeightForWidth(PlotDlg.sizePolicy().hasHeightForWidth())
        PlotDlg.setSizePolicy(sizePolicy)
        PlotDlg.setMinimumSize(QtCore.QSize(80, 91))
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("icons:legend.png")), QtGui.QIcon.Mode.Normal, QtGui.QIcon.State.Off)
        PlotDlg.setWindowIcon(icon)
        PlotDlg.setFloating(True)
        PlotDlg.setAllowedAreas(QtCore.Qt.DockWidgetArea.AllDockWidgetAreas)
        PlotDlg.setWindowTitle(QtWidgets.QApplication.translate("PlotDlg", "Plot", None))
        self.dockWidgetContents = QtWidgets.QWidget()
        sizePolicy = QtWidgets.QSizePolicy(QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Expanding)
        sizePolicy.setHorizontalStretch(1)
        sizePolicy.setVerticalStretch(1)
        sizePolicy.setHeightForWidth(self.dockWidgetContents.sizePolicy().hasHeightForWidth())
        self.dockWidgetContents.setSizePolicy(sizePolicy)
        self.dockWidgetContents.setObjectName(_fromUtf8("dockWidgetContents"))
        self.verticalLayout = QtWidgets.QVBoxLayout(self.dockWidgetContents)
        self.verticalLayout.setSizeConstraint(QtWidgets.QLayout.SizeConstraint.SetNoConstraint)
        self.verticalLayout.setContentsMargins(0, 0, 0, 0)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.scrollArea = QtWidgets.QScrollArea(self.dockWidgetContents)
        sizePolicy = QtWidgets.QSizePolicy(QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Expanding)
        sizePolicy.setHorizontalStretch(1)
        sizePolicy.setVerticalStretch(1)
        sizePolicy.setHeightForWidth(self.scrollArea.sizePolicy().hasHeightForWidth())
        self.scrollArea.setSizePolicy(sizePolicy)
        self.scrollArea.setFrameShape(QtWidgets.QFrame.Shape.NoFrame)
        self.scrollArea.setFrameShadow(QtWidgets.QFrame.Shadow.Plain)
        self.scrollArea.setWidgetResizable(True)
        self.scrollArea.setAlignment(QtCore.Qt.AlignmentFlag.AlignCenter)
        self.scrollArea.setObjectName(_fromUtf8("scrollArea"))
        self.scrollLegend = QtWidgets.QWidget()
        self.scrollLegend.setGeometry(QtCore.QRect(0, 0, 600, 378))
        sizePolicy = QtWidgets.QSizePolicy(QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Expanding)
        sizePolicy.setHorizontalStretch(1)
        sizePolicy.setVerticalStretch(1)
        sizePolicy.setHeightForWidth(self.scrollLegend.sizePolicy().hasHeightForWidth())
        self.scrollLegend.setSizePolicy(sizePolicy)
        self.scrollLegend.setObjectName(_fromUtf8("scrollLegend"))
        self.scrollArea.setWidget(self.scrollLegend)
        self.verticalLayout.addWidget(self.scrollArea)
        PlotDlg.setWidget(self.dockWidgetContents)

        self.retranslateUi(PlotDlg)
        QtCore.QMetaObject.connectSlotsByName(PlotDlg)

    def retranslateUi(self, PlotDlg):
        pass

import stamp.STAMP_rc
