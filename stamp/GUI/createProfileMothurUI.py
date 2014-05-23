# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'createProfileMothur.ui'
#
# Created: Fri Dec 16 13:39:59 2011
#      by: PyQt4 UI code generator 4.8.5
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileMothurDlg(object):
    def setupUi(self, CreateProfileMothurDlg):
        CreateProfileMothurDlg.setObjectName(_fromUtf8("CreateProfileMothurDlg"))
        CreateProfileMothurDlg.resize(396, 144)
        CreateProfileMothurDlg.setWindowTitle(QtGui.QApplication.translate("CreateProfileMothurDlg", "Create profile", None, QtGui.QApplication.UnicodeUTF8))
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/programIcon.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        CreateProfileMothurDlg.setWindowIcon(icon)
        self.verticalLayout_2 = QtGui.QVBoxLayout(CreateProfileMothurDlg)
        self.verticalLayout_2.setObjectName(_fromUtf8("verticalLayout_2"))
        self.formLayout = QtGui.QFormLayout()
        self.formLayout.setObjectName(_fromUtf8("formLayout"))
        self.lblTaxonomyFile = QtGui.QLabel(CreateProfileMothurDlg)
        self.lblTaxonomyFile.setText(QtGui.QApplication.translate("CreateProfileMothurDlg", "Taxonomy file:", None, QtGui.QApplication.UnicodeUTF8))
        self.lblTaxonomyFile.setObjectName(_fromUtf8("lblTaxonomyFile"))
        self.formLayout.setWidget(0, QtGui.QFormLayout.LabelRole, self.lblTaxonomyFile)
        self.horizontalLayout = QtGui.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.txtTaxonomyFile = QtGui.QLineEdit(CreateProfileMothurDlg)
        self.txtTaxonomyFile.setObjectName(_fromUtf8("txtTaxonomyFile"))
        self.horizontalLayout.addWidget(self.txtTaxonomyFile)
        self.btnTaxonomyFile = QtGui.QPushButton(CreateProfileMothurDlg)
        self.btnTaxonomyFile.setText(QtGui.QApplication.translate("CreateProfileMothurDlg", "Load", None, QtGui.QApplication.UnicodeUTF8))
        self.btnTaxonomyFile.setObjectName(_fromUtf8("btnTaxonomyFile"))
        self.horizontalLayout.addWidget(self.btnTaxonomyFile)
        self.formLayout.setLayout(0, QtGui.QFormLayout.FieldRole, self.horizontalLayout)
        self.label_2 = QtGui.QLabel(CreateProfileMothurDlg)
        self.label_2.setText(QtGui.QApplication.translate("CreateProfileMothurDlg", "Groups file:", None, QtGui.QApplication.UnicodeUTF8))
        self.label_2.setObjectName(_fromUtf8("label_2"))
        self.formLayout.setWidget(1, QtGui.QFormLayout.LabelRole, self.label_2)
        self.horizontalLayout_2 = QtGui.QHBoxLayout()
        self.horizontalLayout_2.setObjectName(_fromUtf8("horizontalLayout_2"))
        self.txtGroupsFile = QtGui.QLineEdit(CreateProfileMothurDlg)
        self.txtGroupsFile.setObjectName(_fromUtf8("txtGroupsFile"))
        self.horizontalLayout_2.addWidget(self.txtGroupsFile)
        self.btnGroupsFile = QtGui.QPushButton(CreateProfileMothurDlg)
        self.btnGroupsFile.setText(QtGui.QApplication.translate("CreateProfileMothurDlg", "Load", None, QtGui.QApplication.UnicodeUTF8))
        self.btnGroupsFile.setObjectName(_fromUtf8("btnGroupsFile"))
        self.horizontalLayout_2.addWidget(self.btnGroupsFile)
        self.formLayout.setLayout(1, QtGui.QFormLayout.FieldRole, self.horizontalLayout_2)
        self.label_3 = QtGui.QLabel(CreateProfileMothurDlg)
        self.label_3.setText(QtGui.QApplication.translate("CreateProfileMothurDlg", "Names file (optional):", None, QtGui.QApplication.UnicodeUTF8))
        self.label_3.setObjectName(_fromUtf8("label_3"))
        self.formLayout.setWidget(2, QtGui.QFormLayout.LabelRole, self.label_3)
        self.horizontalLayout_3 = QtGui.QHBoxLayout()
        self.horizontalLayout_3.setObjectName(_fromUtf8("horizontalLayout_3"))
        self.txtNamesFile = QtGui.QLineEdit(CreateProfileMothurDlg)
        self.txtNamesFile.setObjectName(_fromUtf8("txtNamesFile"))
        self.horizontalLayout_3.addWidget(self.txtNamesFile)
        self.btnNamesFile = QtGui.QPushButton(CreateProfileMothurDlg)
        self.btnNamesFile.setText(QtGui.QApplication.translate("CreateProfileMothurDlg", "Load", None, QtGui.QApplication.UnicodeUTF8))
        self.btnNamesFile.setObjectName(_fromUtf8("btnNamesFile"))
        self.horizontalLayout_3.addWidget(self.btnNamesFile)
        self.formLayout.setLayout(2, QtGui.QFormLayout.FieldRole, self.horizontalLayout_3)
        self.horizontalLayout_4 = QtGui.QHBoxLayout()
        self.horizontalLayout_4.setObjectName(_fromUtf8("horizontalLayout_4"))
        self.btnCreateProfile = QtGui.QPushButton(CreateProfileMothurDlg)
        self.btnCreateProfile.setText(QtGui.QApplication.translate("CreateProfileMothurDlg", "Create STAMP profile", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.horizontalLayout_4.addWidget(self.btnCreateProfile)
        self.btnCancel = QtGui.QPushButton(CreateProfileMothurDlg)
        self.btnCancel.setText(QtGui.QApplication.translate("CreateProfileMothurDlg", "Cancel", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCancel.setDefault(True)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.horizontalLayout_4.addWidget(self.btnCancel)
        self.formLayout.setLayout(3, QtGui.QFormLayout.FieldRole, self.horizontalLayout_4)
        spacerItem = QtGui.QSpacerItem(40, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.formLayout.setItem(3, QtGui.QFormLayout.LabelRole, spacerItem)
        self.verticalLayout_2.addLayout(self.formLayout)

        self.retranslateUi(CreateProfileMothurDlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileMothurDlg)

    def retranslateUi(self, CreateProfileMothurDlg):
        pass

