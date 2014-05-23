# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'loadDataDlg.ui'
#
# Created: Tue Apr 26 14:27:41 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_loadDataDlg(object):
    def setupUi(self, loadDataDlg):
        loadDataDlg.setObjectName(_fromUtf8("loadDataDlg"))
        loadDataDlg.resize(404, 103)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/open.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        loadDataDlg.setWindowIcon(icon)
        self.verticalLayout = QtGui.QVBoxLayout(loadDataDlg)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.gridLayout = QtGui.QGridLayout()
        self.gridLayout.setObjectName(_fromUtf8("gridLayout"))
        self.label = QtGui.QLabel(loadDataDlg)
        self.label.setObjectName(_fromUtf8("label"))
        self.gridLayout.addWidget(self.label, 0, 0, 1, 1)
        self.horizontalLayout = QtGui.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.txtProfileFile = QtGui.QLineEdit(loadDataDlg)
        self.txtProfileFile.setObjectName(_fromUtf8("txtProfileFile"))
        self.horizontalLayout.addWidget(self.txtProfileFile)
        self.tbProfileFile = QtGui.QToolButton(loadDataDlg)
        self.tbProfileFile.setText(_fromUtf8(""))
        self.tbProfileFile.setIcon(icon)
        self.tbProfileFile.setObjectName(_fromUtf8("tbProfileFile"))
        self.horizontalLayout.addWidget(self.tbProfileFile)
        self.gridLayout.addLayout(self.horizontalLayout, 0, 1, 1, 1)
        self.label_2 = QtGui.QLabel(loadDataDlg)
        self.label_2.setObjectName(_fromUtf8("label_2"))
        self.gridLayout.addWidget(self.label_2, 1, 0, 1, 1)
        self.horizontalLayout_2 = QtGui.QHBoxLayout()
        self.horizontalLayout_2.setObjectName(_fromUtf8("horizontalLayout_2"))
        self.txtMetadataFile = QtGui.QLineEdit(loadDataDlg)
        self.txtMetadataFile.setObjectName(_fromUtf8("txtMetadataFile"))
        self.horizontalLayout_2.addWidget(self.txtMetadataFile)
        self.tbMetadataFile = QtGui.QToolButton(loadDataDlg)
        self.tbMetadataFile.setText(_fromUtf8(""))
        self.tbMetadataFile.setIcon(icon)
        self.tbMetadataFile.setObjectName(_fromUtf8("tbMetadataFile"))
        self.horizontalLayout_2.addWidget(self.tbMetadataFile)
        self.gridLayout.addLayout(self.horizontalLayout_2, 1, 1, 1, 1)
        self.verticalLayout.addLayout(self.gridLayout)
        self.buttonBox = QtGui.QDialogButtonBox(loadDataDlg)
        self.buttonBox.setStandardButtons(QtGui.QDialogButtonBox.Cancel|QtGui.QDialogButtonBox.Ok)
        self.buttonBox.setObjectName(_fromUtf8("buttonBox"))
        self.verticalLayout.addWidget(self.buttonBox)

        self.retranslateUi(loadDataDlg)
        QtCore.QObject.connect(self.buttonBox, QtCore.SIGNAL(_fromUtf8("accepted()")), loadDataDlg.accept)
        QtCore.QObject.connect(self.buttonBox, QtCore.SIGNAL(_fromUtf8("rejected()")), loadDataDlg.reject)
        QtCore.QMetaObject.connectSlotsByName(loadDataDlg)

    def retranslateUi(self, loadDataDlg):
        loadDataDlg.setWindowTitle(QtGui.QApplication.translate("loadDataDlg", "Load data", None, QtGui.QApplication.UnicodeUTF8))
        self.label.setText(QtGui.QApplication.translate("loadDataDlg", "Profile file:", None, QtGui.QApplication.UnicodeUTF8))
        self.label_2.setText(QtGui.QApplication.translate("loadDataDlg", "Group metadata file (optional):", None, QtGui.QApplication.UnicodeUTF8))

import stamp.STAMP_rc
