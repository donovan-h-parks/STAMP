# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'createProfileMgRast.ui'
#
# Created: Thu Jun 16 10:44:44 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileMgRastDlg(object):
    def setupUi(self, CreateProfileMgRastDlg):
        CreateProfileMgRastDlg.setObjectName(_fromUtf8("CreateProfileMgRastDlg"))
        CreateProfileMgRastDlg.resize(203, 128)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/createProfile.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        CreateProfileMgRastDlg.setWindowIcon(icon)
        self.verticalLayout = QtGui.QVBoxLayout(CreateProfileMgRastDlg)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.btnLoadProfiles = QtGui.QPushButton(CreateProfileMgRastDlg)
        self.btnLoadProfiles.setObjectName(_fromUtf8("btnLoadProfiles"))
        self.verticalLayout.addWidget(self.btnLoadProfiles)
        self.btnCustomizeHeadings = QtGui.QPushButton(CreateProfileMgRastDlg)
        self.btnCustomizeHeadings.setEnabled(False)
        self.btnCustomizeHeadings.setObjectName(_fromUtf8("btnCustomizeHeadings"))
        self.verticalLayout.addWidget(self.btnCustomizeHeadings)
        self.btnCreateProfile = QtGui.QPushButton(CreateProfileMgRastDlg)
        self.btnCreateProfile.setEnabled(False)
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.verticalLayout.addWidget(self.btnCreateProfile)
        self.btnCancel = QtGui.QPushButton(CreateProfileMgRastDlg)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.verticalLayout.addWidget(self.btnCancel)

        self.retranslateUi(CreateProfileMgRastDlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileMgRastDlg)

    def retranslateUi(self, CreateProfileMgRastDlg):
        CreateProfileMgRastDlg.setWindowTitle(QtGui.QApplication.translate("CreateProfileMgRastDlg", "Create profile", None, QtGui.QApplication.UnicodeUTF8))
        self.btnLoadProfiles.setText(QtGui.QApplication.translate("CreateProfileMgRastDlg", "Load profile", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCustomizeHeadings.setText(QtGui.QApplication.translate("CreateProfileMgRastDlg", "Customize headings", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCreateProfile.setText(QtGui.QApplication.translate("CreateProfileMgRastDlg", "Create STAMP profile", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCancel.setText(QtGui.QApplication.translate("CreateProfileMgRastDlg", "Cancel", None, QtGui.QApplication.UnicodeUTF8))

import stamp.STAMP_rc
