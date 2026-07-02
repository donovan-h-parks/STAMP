# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'createProfileMothur.ui'
#
# Created: Fri Dec 16 13:39:59 2011
#      by: PyQt4 UI code generator 4.8.5
#
# WARNING! All changes made in this file will be lost!

from PyQt6 import QtCore, QtGui, QtWidgets

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileMothurDlg(object):
    def setupUi(self, CreateProfileMothurDlg):
        CreateProfileMothurDlg.setObjectName(_fromUtf8("CreateProfileMothurDlg"))
        CreateProfileMothurDlg.resize(396, 144)
        CreateProfileMothurDlg.setWindowTitle(QtWidgets.QApplication.translate("CreateProfileMothurDlg", "Create profile", None))
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("icons:programIcon.png")), QtGui.QIcon.Mode.Normal, QtGui.QIcon.State.Off)
        CreateProfileMothurDlg.setWindowIcon(icon)
        self.verticalLayout_2 = QtWidgets.QVBoxLayout(CreateProfileMothurDlg)
        self.verticalLayout_2.setObjectName(_fromUtf8("verticalLayout_2"))
        self.formLayout = QtWidgets.QFormLayout()
        self.formLayout.setObjectName(_fromUtf8("formLayout"))
        self.lblTaxonomyFile = QtWidgets.QLabel(CreateProfileMothurDlg)
        self.lblTaxonomyFile.setText(QtWidgets.QApplication.translate("CreateProfileMothurDlg", "Taxonomy file:", None))
        self.lblTaxonomyFile.setObjectName(_fromUtf8("lblTaxonomyFile"))
        self.formLayout.setWidget(0, QtWidgets.QFormLayout.ItemRole.LabelRole, self.lblTaxonomyFile)
        self.horizontalLayout = QtWidgets.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.txtTaxonomyFile = QtWidgets.QLineEdit(CreateProfileMothurDlg)
        self.txtTaxonomyFile.setObjectName(_fromUtf8("txtTaxonomyFile"))
        self.horizontalLayout.addWidget(self.txtTaxonomyFile)
        self.btnTaxonomyFile = QtWidgets.QPushButton(CreateProfileMothurDlg)
        self.btnTaxonomyFile.setText(QtWidgets.QApplication.translate("CreateProfileMothurDlg", "Load", None))
        self.btnTaxonomyFile.setObjectName(_fromUtf8("btnTaxonomyFile"))
        self.horizontalLayout.addWidget(self.btnTaxonomyFile)
        self.formLayout.setLayout(0, QtWidgets.QFormLayout.ItemRole.FieldRole, self.horizontalLayout)
        self.label_2 = QtWidgets.QLabel(CreateProfileMothurDlg)
        self.label_2.setText(QtWidgets.QApplication.translate("CreateProfileMothurDlg", "Groups file:", None))
        self.label_2.setObjectName(_fromUtf8("label_2"))
        self.formLayout.setWidget(1, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label_2)
        self.horizontalLayout_2 = QtWidgets.QHBoxLayout()
        self.horizontalLayout_2.setObjectName(_fromUtf8("horizontalLayout_2"))
        self.txtGroupsFile = QtWidgets.QLineEdit(CreateProfileMothurDlg)
        self.txtGroupsFile.setObjectName(_fromUtf8("txtGroupsFile"))
        self.horizontalLayout_2.addWidget(self.txtGroupsFile)
        self.btnGroupsFile = QtWidgets.QPushButton(CreateProfileMothurDlg)
        self.btnGroupsFile.setText(QtWidgets.QApplication.translate("CreateProfileMothurDlg", "Load", None))
        self.btnGroupsFile.setObjectName(_fromUtf8("btnGroupsFile"))
        self.horizontalLayout_2.addWidget(self.btnGroupsFile)
        self.formLayout.setLayout(1, QtWidgets.QFormLayout.ItemRole.FieldRole, self.horizontalLayout_2)
        self.label_3 = QtWidgets.QLabel(CreateProfileMothurDlg)
        self.label_3.setText(QtWidgets.QApplication.translate("CreateProfileMothurDlg", "Names file (optional):", None))
        self.label_3.setObjectName(_fromUtf8("label_3"))
        self.formLayout.setWidget(2, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label_3)
        self.horizontalLayout_3 = QtWidgets.QHBoxLayout()
        self.horizontalLayout_3.setObjectName(_fromUtf8("horizontalLayout_3"))
        self.txtNamesFile = QtWidgets.QLineEdit(CreateProfileMothurDlg)
        self.txtNamesFile.setObjectName(_fromUtf8("txtNamesFile"))
        self.horizontalLayout_3.addWidget(self.txtNamesFile)
        self.btnNamesFile = QtWidgets.QPushButton(CreateProfileMothurDlg)
        self.btnNamesFile.setText(QtWidgets.QApplication.translate("CreateProfileMothurDlg", "Load", None))
        self.btnNamesFile.setObjectName(_fromUtf8("btnNamesFile"))
        self.horizontalLayout_3.addWidget(self.btnNamesFile)
        self.formLayout.setLayout(2, QtWidgets.QFormLayout.ItemRole.FieldRole, self.horizontalLayout_3)
        self.horizontalLayout_4 = QtWidgets.QHBoxLayout()
        self.horizontalLayout_4.setObjectName(_fromUtf8("horizontalLayout_4"))
        self.btnCreateProfile = QtWidgets.QPushButton(CreateProfileMothurDlg)
        self.btnCreateProfile.setText(QtWidgets.QApplication.translate("CreateProfileMothurDlg", "Create STAMP profile", None))
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.horizontalLayout_4.addWidget(self.btnCreateProfile)
        self.btnCancel = QtWidgets.QPushButton(CreateProfileMothurDlg)
        self.btnCancel.setText(QtWidgets.QApplication.translate("CreateProfileMothurDlg", "Cancel", None))
        self.btnCancel.setDefault(True)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.horizontalLayout_4.addWidget(self.btnCancel)
        self.formLayout.setLayout(3, QtWidgets.QFormLayout.ItemRole.FieldRole, self.horizontalLayout_4)
        spacerItem = QtWidgets.QSpacerItem(40, 20, QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Minimum)
        self.formLayout.setItem(3, QtWidgets.QFormLayout.ItemRole.LabelRole, spacerItem)
        self.verticalLayout_2.addLayout(self.formLayout)

        self.retranslateUi(CreateProfileMothurDlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileMothurDlg)

    def retranslateUi(self, CreateProfileMothurDlg):
        pass

