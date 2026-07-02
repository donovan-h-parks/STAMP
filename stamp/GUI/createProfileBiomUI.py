# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'createProfileBiom.ui'
#
# Created: Thu Mar 06 08:05:50 2014
#      by: PyQt4 UI code generator 4.9.5
#
# WARNING! All changes made in this file will be lost!

from PyQt6 import QtCore, QtGui, QtWidgets

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileBiomDlg(object):
    def setupUi(self, CreateProfileBiomDlg):
        CreateProfileBiomDlg.setObjectName(_fromUtf8("CreateProfileBiomDlg"))
        CreateProfileBiomDlg.resize(396, 122)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("icons:programIcon.png")), QtGui.QIcon.Mode.Normal, QtGui.QIcon.State.Off)
        CreateProfileBiomDlg.setWindowIcon(icon)
        self.verticalLayout_2 = QtWidgets.QVBoxLayout(CreateProfileBiomDlg)
        self.verticalLayout_2.setObjectName(_fromUtf8("verticalLayout_2"))
        self.formLayout = QtWidgets.QFormLayout()
        self.formLayout.setFieldGrowthPolicy(QtWidgets.QFormLayout.FieldGrowthPolicy.AllNonFixedFieldsGrow)
        self.formLayout.setObjectName(_fromUtf8("formLayout"))
        self.lblTaxonomyFile = QtWidgets.QLabel(CreateProfileBiomDlg)
        self.lblTaxonomyFile.setObjectName(_fromUtf8("lblTaxonomyFile"))
        self.formLayout.setWidget(0, QtWidgets.QFormLayout.ItemRole.LabelRole, self.lblTaxonomyFile)
        self.horizontalLayout = QtWidgets.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.txtBiomFile = QtWidgets.QLineEdit(CreateProfileBiomDlg)
        self.txtBiomFile.setObjectName(_fromUtf8("txtBiomFile"))
        self.horizontalLayout.addWidget(self.txtBiomFile)
        self.btnBiomFile = QtWidgets.QPushButton(CreateProfileBiomDlg)
        self.btnBiomFile.setObjectName(_fromUtf8("btnBiomFile"))
        self.horizontalLayout.addWidget(self.btnBiomFile)
        self.formLayout.setLayout(0, QtWidgets.QFormLayout.ItemRole.FieldRole, self.horizontalLayout)
        self.label_2 = QtWidgets.QLabel(CreateProfileBiomDlg)
        self.label_2.setObjectName(_fromUtf8("label_2"))
        self.formLayout.setWidget(1, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label_2)
        self.horizontalLayout_2 = QtWidgets.QHBoxLayout()
        self.horizontalLayout_2.setObjectName(_fromUtf8("horizontalLayout_2"))
        self.cboMetadataField = QtWidgets.QComboBox(CreateProfileBiomDlg)
        self.cboMetadataField.setEditable(True)
        self.cboMetadataField.setObjectName(_fromUtf8("cboMetadataField"))
        self.cboMetadataField.addItem(_fromUtf8(""))
        self.cboMetadataField.addItem(_fromUtf8(""))
        self.cboMetadataField.addItem(_fromUtf8(""))
        self.cboMetadataField.addItem(_fromUtf8(""))
        self.horizontalLayout_2.addWidget(self.cboMetadataField)
        self.formLayout.setLayout(1, QtWidgets.QFormLayout.ItemRole.FieldRole, self.horizontalLayout_2)
        spacerItem = QtWidgets.QSpacerItem(40, 20, QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Minimum)
        self.formLayout.setItem(2, QtWidgets.QFormLayout.ItemRole.LabelRole, spacerItem)
        self.horizontalLayout_4 = QtWidgets.QHBoxLayout()
        self.horizontalLayout_4.setObjectName(_fromUtf8("horizontalLayout_4"))
        self.btnCreateProfile = QtWidgets.QPushButton(CreateProfileBiomDlg)
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.horizontalLayout_4.addWidget(self.btnCreateProfile)
        self.btnCancel = QtWidgets.QPushButton(CreateProfileBiomDlg)
        self.btnCancel.setDefault(True)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.horizontalLayout_4.addWidget(self.btnCancel)
        self.formLayout.setLayout(2, QtWidgets.QFormLayout.ItemRole.FieldRole, self.horizontalLayout_4)
        self.verticalLayout_2.addLayout(self.formLayout)

        self.retranslateUi(CreateProfileBiomDlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileBiomDlg)

    def retranslateUi(self, CreateProfileBiomDlg):
        CreateProfileBiomDlg.setWindowTitle(QtWidgets.QApplication.translate("CreateProfileBiomDlg", "Create profile", None))
        self.lblTaxonomyFile.setText(QtWidgets.QApplication.translate("CreateProfileBiomDlg", "BIOM file:", None))
        self.btnBiomFile.setText(QtWidgets.QApplication.translate("CreateProfileBiomDlg", "Load", None))
        self.label_2.setText(QtWidgets.QApplication.translate("CreateProfileBiomDlg", "Metadata field:", None))
        self.cboMetadataField.setItemText(0, QtWidgets.QApplication.translate("CreateProfileBiomDlg", "<observation ids>", None))
        self.cboMetadataField.setItemText(1, QtWidgets.QApplication.translate("CreateProfileBiomDlg", "COG_Category (PICRUSt COG analysis)", None))
        self.cboMetadataField.setItemText(2, QtWidgets.QApplication.translate("CreateProfileBiomDlg", "KEGG_Pathways (PICRUSt KEGG analysis)", None))
        self.cboMetadataField.setItemText(3, QtWidgets.QApplication.translate("CreateProfileBiomDlg", "taxonomy (QIIME OTU table)", None))
        self.btnCreateProfile.setText(QtWidgets.QApplication.translate("CreateProfileBiomDlg", "Create STAMP profile", None))
        self.btnCancel.setText(QtWidgets.QApplication.translate("CreateProfileBiomDlg", "Cancel", None))

