# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'loadDataDlg.ui'
#
# Created: Tue Apr 26 14:27:41 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt6 import QtCore, QtGui, QtWidgets

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_loadDataDlg(object):
    def setupUi(self, loadDataDlg):
        loadDataDlg.setObjectName(_fromUtf8("loadDataDlg"))
        loadDataDlg.resize(404, 103)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("icons:open.png")), QtGui.QIcon.Mode.Normal, QtGui.QIcon.State.Off)
        loadDataDlg.setWindowIcon(icon)
        self.verticalLayout = QtWidgets.QVBoxLayout(loadDataDlg)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.gridLayout = QtWidgets.QGridLayout()
        self.gridLayout.setObjectName(_fromUtf8("gridLayout"))
        self.label = QtWidgets.QLabel(loadDataDlg)
        self.label.setObjectName(_fromUtf8("label"))
        self.gridLayout.addWidget(self.label, 0, 0, 1, 1)
        self.horizontalLayout = QtWidgets.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.txtProfileFile = QtWidgets.QLineEdit(loadDataDlg)
        self.txtProfileFile.setObjectName(_fromUtf8("txtProfileFile"))
        self.horizontalLayout.addWidget(self.txtProfileFile)
        self.tbProfileFile = QtWidgets.QToolButton(loadDataDlg)
        self.tbProfileFile.setText(_fromUtf8(""))
        self.tbProfileFile.setIcon(icon)
        self.tbProfileFile.setObjectName(_fromUtf8("tbProfileFile"))
        self.horizontalLayout.addWidget(self.tbProfileFile)
        self.gridLayout.addLayout(self.horizontalLayout, 0, 1, 1, 1)
        self.label_2 = QtWidgets.QLabel(loadDataDlg)
        self.label_2.setObjectName(_fromUtf8("label_2"))
        self.gridLayout.addWidget(self.label_2, 1, 0, 1, 1)
        self.horizontalLayout_2 = QtWidgets.QHBoxLayout()
        self.horizontalLayout_2.setObjectName(_fromUtf8("horizontalLayout_2"))
        self.txtMetadataFile = QtWidgets.QLineEdit(loadDataDlg)
        self.txtMetadataFile.setObjectName(_fromUtf8("txtMetadataFile"))
        self.horizontalLayout_2.addWidget(self.txtMetadataFile)
        self.tbMetadataFile = QtWidgets.QToolButton(loadDataDlg)
        self.tbMetadataFile.setText(_fromUtf8(""))
        self.tbMetadataFile.setIcon(icon)
        self.tbMetadataFile.setObjectName(_fromUtf8("tbMetadataFile"))
        self.horizontalLayout_2.addWidget(self.tbMetadataFile)
        self.gridLayout.addLayout(self.horizontalLayout_2, 1, 1, 1, 1)
        self.verticalLayout.addLayout(self.gridLayout)
        self.buttonBox = QtWidgets.QDialogButtonBox(loadDataDlg)
        self.buttonBox.setStandardButtons(QtWidgets.QDialogButtonBox.StandardButton.Cancel|QtWidgets.QDialogButtonBox.StandardButton.Ok)
        self.buttonBox.setObjectName(_fromUtf8("buttonBox"))
        self.verticalLayout.addWidget(self.buttonBox)

        self.retranslateUi(loadDataDlg)
        self.buttonBox.accepted.connect(loadDataDlg.accept)
        self.buttonBox.rejected.connect(loadDataDlg.reject)
        QtCore.QMetaObject.connectSlotsByName(loadDataDlg)

    def retranslateUi(self, loadDataDlg):
        loadDataDlg.setWindowTitle(QtWidgets.QApplication.translate("loadDataDlg", "Load data", None))
        self.label.setText(QtWidgets.QApplication.translate("loadDataDlg", "Profile file:", None))
        self.label_2.setText(QtWidgets.QApplication.translate("loadDataDlg", "Group metadata file (optional):", None))

import stamp.STAMP_rc
