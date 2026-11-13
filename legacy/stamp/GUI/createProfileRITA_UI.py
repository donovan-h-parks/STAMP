# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'createProfileRITA.ui'
#
# Created: Thu Jun 16 20:11:06 2011
#      by: PyQt4 UI code generator 4.8.4
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileRITADlg(object):
    def setupUi(self, CreateProfileRITADlg):
        CreateProfileRITADlg.setObjectName(_fromUtf8("CreateProfileRITADlg"))
        CreateProfileRITADlg.resize(595, 295)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/programIcon.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        CreateProfileRITADlg.setWindowIcon(icon)
        self.horizontalLayout = QtGui.QHBoxLayout(CreateProfileRITADlg)
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.verticalLayout = QtGui.QVBoxLayout()
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.btnLoadProfiles = QtGui.QPushButton(CreateProfileRITADlg)
        self.btnLoadProfiles.setObjectName(_fromUtf8("btnLoadProfiles"))
        self.verticalLayout.addWidget(self.btnLoadProfiles)
        self.groupBox = QtGui.QGroupBox(CreateProfileRITADlg)
        self.groupBox.setObjectName(_fromUtf8("groupBox"))
        self.verticalLayout_2 = QtGui.QVBoxLayout(self.groupBox)
        self.verticalLayout_2.setObjectName(_fromUtf8("verticalLayout_2"))
        self.chkNB_DBLASTN = QtGui.QCheckBox(self.groupBox)
        self.chkNB_DBLASTN.setChecked(True)
        self.chkNB_DBLASTN.setObjectName(_fromUtf8("chkNB_DBLASTN"))
        self.verticalLayout_2.addWidget(self.chkNB_DBLASTN)
        self.chkDBLASTN = QtGui.QCheckBox(self.groupBox)
        self.chkDBLASTN.setChecked(True)
        self.chkDBLASTN.setObjectName(_fromUtf8("chkDBLASTN"))
        self.verticalLayout_2.addWidget(self.chkDBLASTN)
        self.chkNB_BLASTN = QtGui.QCheckBox(self.groupBox)
        self.chkNB_BLASTN.setChecked(True)
        self.chkNB_BLASTN.setObjectName(_fromUtf8("chkNB_BLASTN"))
        self.verticalLayout_2.addWidget(self.chkNB_BLASTN)
        self.chkBLASTN = QtGui.QCheckBox(self.groupBox)
        self.chkBLASTN.setChecked(True)
        self.chkBLASTN.setObjectName(_fromUtf8("chkBLASTN"))
        self.verticalLayout_2.addWidget(self.chkBLASTN)
        self.chkNB_BLASTX = QtGui.QCheckBox(self.groupBox)
        self.chkNB_BLASTX.setChecked(True)
        self.chkNB_BLASTX.setObjectName(_fromUtf8("chkNB_BLASTX"))
        self.verticalLayout_2.addWidget(self.chkNB_BLASTX)
        self.chkBLASTX = QtGui.QCheckBox(self.groupBox)
        self.chkBLASTX.setChecked(True)
        self.chkBLASTX.setObjectName(_fromUtf8("chkBLASTX"))
        self.verticalLayout_2.addWidget(self.chkBLASTX)
        self.chkNB = QtGui.QCheckBox(self.groupBox)
        self.chkNB.setChecked(True)
        self.chkNB.setObjectName(_fromUtf8("chkNB"))
        self.verticalLayout_2.addWidget(self.chkNB)
        self.verticalLayout.addWidget(self.groupBox)
        self.btnCreateProfile = QtGui.QPushButton(CreateProfileRITADlg)
        self.btnCreateProfile.setEnabled(False)
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.verticalLayout.addWidget(self.btnCreateProfile)
        self.btnCancel = QtGui.QPushButton(CreateProfileRITADlg)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.verticalLayout.addWidget(self.btnCancel)
        self.horizontalLayout.addLayout(self.verticalLayout)
        self.lstSelectedProfiles = QtGui.QListWidget(CreateProfileRITADlg)
        self.lstSelectedProfiles.setObjectName(_fromUtf8("lstSelectedProfiles"))
        self.horizontalLayout.addWidget(self.lstSelectedProfiles)

        self.retranslateUi(CreateProfileRITADlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileRITADlg)

    def retranslateUi(self, CreateProfileRITADlg):
        CreateProfileRITADlg.setWindowTitle(QtGui.QApplication.translate("CreateProfileRITADlg", "Create profile", None, QtGui.QApplication.UnicodeUTF8))
        self.btnLoadProfiles.setText(QtGui.QApplication.translate("CreateProfileRITADlg", "Load profiles", None, QtGui.QApplication.UnicodeUTF8))
        self.groupBox.setTitle(QtGui.QApplication.translate("CreateProfileRITADlg", "Groups to retain", None, QtGui.QApplication.UnicodeUTF8))
        self.chkNB_DBLASTN.setText(QtGui.QApplication.translate("CreateProfileRITADlg", "NB and D-BLASTN", None, QtGui.QApplication.UnicodeUTF8))
        self.chkDBLASTN.setText(QtGui.QApplication.translate("CreateProfileRITADlg", "D-BLASTN ratio", None, QtGui.QApplication.UnicodeUTF8))
        self.chkNB_BLASTN.setText(QtGui.QApplication.translate("CreateProfileRITADlg", "NB and BLASTN", None, QtGui.QApplication.UnicodeUTF8))
        self.chkBLASTN.setText(QtGui.QApplication.translate("CreateProfileRITADlg", "BLASTN ratio", None, QtGui.QApplication.UnicodeUTF8))
        self.chkNB_BLASTX.setText(QtGui.QApplication.translate("CreateProfileRITADlg", "NB and BLASTX", None, QtGui.QApplication.UnicodeUTF8))
        self.chkBLASTX.setText(QtGui.QApplication.translate("CreateProfileRITADlg", "BLASTX ratio", None, QtGui.QApplication.UnicodeUTF8))
        self.chkNB.setText(QtGui.QApplication.translate("CreateProfileRITADlg", "NB ratio", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCreateProfile.setText(QtGui.QApplication.translate("CreateProfileRITADlg", "Create STAMP profile", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCancel.setText(QtGui.QApplication.translate("CreateProfileRITADlg", "Cancel", None, QtGui.QApplication.UnicodeUTF8))
        self.lstSelectedProfiles.setSortingEnabled(True)

