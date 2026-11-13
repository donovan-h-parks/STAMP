# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'plotDlg.ui'
#
# Created: Mon Jan 02 11:48:31 2012
#      by: PyQt4 UI code generator 4.8.5
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_PlotDlg(object):
    def setupUi(self, PlotDlg):
        PlotDlg.setObjectName(_fromUtf8("PlotDlg"))
        PlotDlg.resize(600, 400)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Expanding)
        sizePolicy.setHorizontalStretch(1)
        sizePolicy.setVerticalStretch(1)
        sizePolicy.setHeightForWidth(PlotDlg.sizePolicy().hasHeightForWidth())
        PlotDlg.setSizePolicy(sizePolicy)
        PlotDlg.setMinimumSize(QtCore.QSize(80, 91))
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/legend.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        PlotDlg.setWindowIcon(icon)
        PlotDlg.setFloating(True)
        PlotDlg.setAllowedAreas(QtCore.Qt.AllDockWidgetAreas)
        PlotDlg.setWindowTitle(QtGui.QApplication.translate("PlotDlg", "Plot", None, QtGui.QApplication.UnicodeUTF8))
        self.dockWidgetContents = QtGui.QWidget()
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Expanding)
        sizePolicy.setHorizontalStretch(1)
        sizePolicy.setVerticalStretch(1)
        sizePolicy.setHeightForWidth(self.dockWidgetContents.sizePolicy().hasHeightForWidth())
        self.dockWidgetContents.setSizePolicy(sizePolicy)
        self.dockWidgetContents.setObjectName(_fromUtf8("dockWidgetContents"))
        self.verticalLayout = QtGui.QVBoxLayout(self.dockWidgetContents)
        self.verticalLayout.setSizeConstraint(QtGui.QLayout.SetNoConstraint)
        self.verticalLayout.setMargin(0)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.scrollArea = QtGui.QScrollArea(self.dockWidgetContents)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Expanding)
        sizePolicy.setHorizontalStretch(1)
        sizePolicy.setVerticalStretch(1)
        sizePolicy.setHeightForWidth(self.scrollArea.sizePolicy().hasHeightForWidth())
        self.scrollArea.setSizePolicy(sizePolicy)
        self.scrollArea.setFrameShape(QtGui.QFrame.NoFrame)
        self.scrollArea.setFrameShadow(QtGui.QFrame.Plain)
        self.scrollArea.setWidgetResizable(True)
        self.scrollArea.setAlignment(QtCore.Qt.AlignCenter)
        self.scrollArea.setObjectName(_fromUtf8("scrollArea"))
        self.scrollLegend = QtGui.QWidget()
        self.scrollLegend.setGeometry(QtCore.QRect(0, 0, 600, 378))
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Expanding)
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
