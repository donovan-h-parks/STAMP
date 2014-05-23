# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'createProfileCoMet.ui'
#
# Created: Thu Jun 16 20:11:23 2011
#      by: PyQt4 UI code generator 4.8.4
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileCoMetDlg(object):
    def setupUi(self, CreateProfileCoMetDlg):
        CreateProfileCoMetDlg.setObjectName(_fromUtf8("CreateProfileCoMetDlg"))
        CreateProfileCoMetDlg.resize(554, 295)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/programIcon.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        CreateProfileCoMetDlg.setWindowIcon(icon)
        self.verticalLayout_2 = QtGui.QVBoxLayout(CreateProfileCoMetDlg)
        self.verticalLayout_2.setObjectName(_fromUtf8("verticalLayout_2"))
        self.horizontalLayout_3 = QtGui.QHBoxLayout()
        self.horizontalLayout_3.setObjectName(_fromUtf8("horizontalLayout_3"))
        self.verticalLayout = QtGui.QVBoxLayout()
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.btnLoadProfiles = QtGui.QPushButton(CreateProfileCoMetDlg)
        self.btnLoadProfiles.setObjectName(_fromUtf8("btnLoadProfiles"))
        self.verticalLayout.addWidget(self.btnLoadProfiles)
        spacerItem = QtGui.QSpacerItem(20, 40, QtGui.QSizePolicy.Minimum, QtGui.QSizePolicy.Expanding)
        self.verticalLayout.addItem(spacerItem)
        self.btnCreateProfile = QtGui.QPushButton(CreateProfileCoMetDlg)
        self.btnCreateProfile.setEnabled(False)
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.verticalLayout.addWidget(self.btnCreateProfile)
        self.btnCancel = QtGui.QPushButton(CreateProfileCoMetDlg)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.verticalLayout.addWidget(self.btnCancel)
        self.horizontalLayout_3.addLayout(self.verticalLayout)
        self.lstSelectedProfiles = QtGui.QListWidget(CreateProfileCoMetDlg)
        self.lstSelectedProfiles.setObjectName(_fromUtf8("lstSelectedProfiles"))
        self.horizontalLayout_3.addWidget(self.lstSelectedProfiles)
        self.verticalLayout_2.addLayout(self.horizontalLayout_3)

        self.retranslateUi(CreateProfileCoMetDlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileCoMetDlg)

    def retranslateUi(self, CreateProfileCoMetDlg):
        CreateProfileCoMetDlg.setWindowTitle(QtGui.QApplication.translate("CreateProfileCoMetDlg", "Create profile", None, QtGui.QApplication.UnicodeUTF8))
        self.btnLoadProfiles.setText(QtGui.QApplication.translate("CreateProfileCoMetDlg", "Load profiles", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCreateProfile.setText(QtGui.QApplication.translate("CreateProfileCoMetDlg", "Create STAMP profile", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCancel.setText(QtGui.QApplication.translate("CreateProfileCoMetDlg", "Cancel", None, QtGui.QApplication.UnicodeUTF8))
        self.lstSelectedProfiles.setSortingEnabled(True)

