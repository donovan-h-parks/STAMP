# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'createProfileMgRast.ui'
#
# Created: Thu Jun 16 10:44:44 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt6 import QtCore, QtGui, QtWidgets

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileMgRastDlg(object):
    def setupUi(self, CreateProfileMgRastDlg):
        CreateProfileMgRastDlg.setObjectName(_fromUtf8("CreateProfileMgRastDlg"))
        CreateProfileMgRastDlg.resize(203, 128)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("icons:createProfile.png")), QtGui.QIcon.Mode.Normal, QtGui.QIcon.State.Off)
        CreateProfileMgRastDlg.setWindowIcon(icon)
        self.verticalLayout = QtWidgets.QVBoxLayout(CreateProfileMgRastDlg)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.btnLoadProfiles = QtWidgets.QPushButton(CreateProfileMgRastDlg)
        self.btnLoadProfiles.setObjectName(_fromUtf8("btnLoadProfiles"))
        self.verticalLayout.addWidget(self.btnLoadProfiles)
        self.btnCustomizeHeadings = QtWidgets.QPushButton(CreateProfileMgRastDlg)
        self.btnCustomizeHeadings.setEnabled(False)
        self.btnCustomizeHeadings.setObjectName(_fromUtf8("btnCustomizeHeadings"))
        self.verticalLayout.addWidget(self.btnCustomizeHeadings)
        self.btnCreateProfile = QtWidgets.QPushButton(CreateProfileMgRastDlg)
        self.btnCreateProfile.setEnabled(False)
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.verticalLayout.addWidget(self.btnCreateProfile)
        self.btnCancel = QtWidgets.QPushButton(CreateProfileMgRastDlg)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.verticalLayout.addWidget(self.btnCancel)

        self.retranslateUi(CreateProfileMgRastDlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileMgRastDlg)

    def retranslateUi(self, CreateProfileMgRastDlg):
        CreateProfileMgRastDlg.setWindowTitle(QtWidgets.QApplication.translate("CreateProfileMgRastDlg", "Create profile", None))
        self.btnLoadProfiles.setText(QtWidgets.QApplication.translate("CreateProfileMgRastDlg", "Load profile", None))
        self.btnCustomizeHeadings.setText(QtWidgets.QApplication.translate("CreateProfileMgRastDlg", "Customize headings", None))
        self.btnCreateProfile.setText(QtWidgets.QApplication.translate("CreateProfileMgRastDlg", "Create STAMP profile", None))
        self.btnCancel.setText(QtWidgets.QApplication.translate("CreateProfileMgRastDlg", "Cancel", None))

import stamp.STAMP_rc
