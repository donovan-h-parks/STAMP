# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'createProfileCoMet.ui'
#
# Created: Thu Jun 16 20:11:23 2011
#      by: PyQt4 UI code generator 4.8.4
#
# WARNING! All changes made in this file will be lost!

from PyQt6 import QtCore, QtGui, QtWidgets

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileCoMetDlg(object):
    def setupUi(self, CreateProfileCoMetDlg):
        CreateProfileCoMetDlg.setObjectName(_fromUtf8("CreateProfileCoMetDlg"))
        CreateProfileCoMetDlg.resize(554, 295)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("icons:programIcon.png")), QtGui.QIcon.Mode.Normal, QtGui.QIcon.State.Off)
        CreateProfileCoMetDlg.setWindowIcon(icon)
        self.verticalLayout_2 = QtWidgets.QVBoxLayout(CreateProfileCoMetDlg)
        self.verticalLayout_2.setObjectName(_fromUtf8("verticalLayout_2"))
        self.horizontalLayout_3 = QtWidgets.QHBoxLayout()
        self.horizontalLayout_3.setObjectName(_fromUtf8("horizontalLayout_3"))
        self.verticalLayout = QtWidgets.QVBoxLayout()
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.btnLoadProfiles = QtWidgets.QPushButton(CreateProfileCoMetDlg)
        self.btnLoadProfiles.setObjectName(_fromUtf8("btnLoadProfiles"))
        self.verticalLayout.addWidget(self.btnLoadProfiles)
        spacerItem = QtWidgets.QSpacerItem(20, 40, QtWidgets.QSizePolicy.Policy.Minimum, QtWidgets.QSizePolicy.Policy.Expanding)
        self.verticalLayout.addItem(spacerItem)
        self.btnCreateProfile = QtWidgets.QPushButton(CreateProfileCoMetDlg)
        self.btnCreateProfile.setEnabled(False)
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.verticalLayout.addWidget(self.btnCreateProfile)
        self.btnCancel = QtWidgets.QPushButton(CreateProfileCoMetDlg)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.verticalLayout.addWidget(self.btnCancel)
        self.horizontalLayout_3.addLayout(self.verticalLayout)
        self.lstSelectedProfiles = QtWidgets.QListWidget(CreateProfileCoMetDlg)
        self.lstSelectedProfiles.setObjectName(_fromUtf8("lstSelectedProfiles"))
        self.horizontalLayout_3.addWidget(self.lstSelectedProfiles)
        self.verticalLayout_2.addLayout(self.horizontalLayout_3)

        self.retranslateUi(CreateProfileCoMetDlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileCoMetDlg)

    def retranslateUi(self, CreateProfileCoMetDlg):
        CreateProfileCoMetDlg.setWindowTitle(QtWidgets.QApplication.translate("CreateProfileCoMetDlg", "Create profile", None))
        self.btnLoadProfiles.setText(QtWidgets.QApplication.translate("CreateProfileCoMetDlg", "Load profiles", None))
        self.btnCreateProfile.setText(QtWidgets.QApplication.translate("CreateProfileCoMetDlg", "Create STAMP profile", None))
        self.btnCancel.setText(QtWidgets.QApplication.translate("CreateProfileCoMetDlg", "Cancel", None))
        self.lstSelectedProfiles.setSortingEnabled(True)

