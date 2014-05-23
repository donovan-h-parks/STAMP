# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'createProfileBiom.ui'
#
# Created: Thu Mar 06 08:05:50 2014
#      by: PyQt4 UI code generator 4.9.5
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileBiomDlg(object):
    def setupUi(self, CreateProfileBiomDlg):
        CreateProfileBiomDlg.setObjectName(_fromUtf8("CreateProfileBiomDlg"))
        CreateProfileBiomDlg.resize(396, 122)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/programIcon.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        CreateProfileBiomDlg.setWindowIcon(icon)
        self.verticalLayout_2 = QtGui.QVBoxLayout(CreateProfileBiomDlg)
        self.verticalLayout_2.setObjectName(_fromUtf8("verticalLayout_2"))
        self.formLayout = QtGui.QFormLayout()
        self.formLayout.setFieldGrowthPolicy(QtGui.QFormLayout.AllNonFixedFieldsGrow)
        self.formLayout.setObjectName(_fromUtf8("formLayout"))
        self.lblTaxonomyFile = QtGui.QLabel(CreateProfileBiomDlg)
        self.lblTaxonomyFile.setObjectName(_fromUtf8("lblTaxonomyFile"))
        self.formLayout.setWidget(0, QtGui.QFormLayout.LabelRole, self.lblTaxonomyFile)
        self.horizontalLayout = QtGui.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.txtBiomFile = QtGui.QLineEdit(CreateProfileBiomDlg)
        self.txtBiomFile.setObjectName(_fromUtf8("txtBiomFile"))
        self.horizontalLayout.addWidget(self.txtBiomFile)
        self.btnBiomFile = QtGui.QPushButton(CreateProfileBiomDlg)
        self.btnBiomFile.setObjectName(_fromUtf8("btnBiomFile"))
        self.horizontalLayout.addWidget(self.btnBiomFile)
        self.formLayout.setLayout(0, QtGui.QFormLayout.FieldRole, self.horizontalLayout)
        self.label_2 = QtGui.QLabel(CreateProfileBiomDlg)
        self.label_2.setObjectName(_fromUtf8("label_2"))
        self.formLayout.setWidget(1, QtGui.QFormLayout.LabelRole, self.label_2)
        self.horizontalLayout_2 = QtGui.QHBoxLayout()
        self.horizontalLayout_2.setObjectName(_fromUtf8("horizontalLayout_2"))
        self.cboMetadataField = QtGui.QComboBox(CreateProfileBiomDlg)
        self.cboMetadataField.setEditable(True)
        self.cboMetadataField.setObjectName(_fromUtf8("cboMetadataField"))
        self.cboMetadataField.addItem(_fromUtf8(""))
        self.cboMetadataField.addItem(_fromUtf8(""))
        self.cboMetadataField.addItem(_fromUtf8(""))
        self.cboMetadataField.addItem(_fromUtf8(""))
        self.horizontalLayout_2.addWidget(self.cboMetadataField)
        self.formLayout.setLayout(1, QtGui.QFormLayout.FieldRole, self.horizontalLayout_2)
        spacerItem = QtGui.QSpacerItem(40, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.formLayout.setItem(2, QtGui.QFormLayout.LabelRole, spacerItem)
        self.horizontalLayout_4 = QtGui.QHBoxLayout()
        self.horizontalLayout_4.setObjectName(_fromUtf8("horizontalLayout_4"))
        self.btnCreateProfile = QtGui.QPushButton(CreateProfileBiomDlg)
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.horizontalLayout_4.addWidget(self.btnCreateProfile)
        self.btnCancel = QtGui.QPushButton(CreateProfileBiomDlg)
        self.btnCancel.setDefault(True)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.horizontalLayout_4.addWidget(self.btnCancel)
        self.formLayout.setLayout(2, QtGui.QFormLayout.FieldRole, self.horizontalLayout_4)
        self.verticalLayout_2.addLayout(self.formLayout)

        self.retranslateUi(CreateProfileBiomDlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileBiomDlg)

    def retranslateUi(self, CreateProfileBiomDlg):
        CreateProfileBiomDlg.setWindowTitle(QtGui.QApplication.translate("CreateProfileBiomDlg", "Create profile", None, QtGui.QApplication.UnicodeUTF8))
        self.lblTaxonomyFile.setText(QtGui.QApplication.translate("CreateProfileBiomDlg", "BIOM file:", None, QtGui.QApplication.UnicodeUTF8))
        self.btnBiomFile.setText(QtGui.QApplication.translate("CreateProfileBiomDlg", "Load", None, QtGui.QApplication.UnicodeUTF8))
        self.label_2.setText(QtGui.QApplication.translate("CreateProfileBiomDlg", "Metadata field:", None, QtGui.QApplication.UnicodeUTF8))
        self.cboMetadataField.setItemText(0, QtGui.QApplication.translate("CreateProfileBiomDlg", "<observation ids>", None, QtGui.QApplication.UnicodeUTF8))
        self.cboMetadataField.setItemText(1, QtGui.QApplication.translate("CreateProfileBiomDlg", "COG_Category (PICRUSt COG analysis)", None, QtGui.QApplication.UnicodeUTF8))
        self.cboMetadataField.setItemText(2, QtGui.QApplication.translate("CreateProfileBiomDlg", "KEGG_Pathways (PICRUSt KEGG analysis)", None, QtGui.QApplication.UnicodeUTF8))
        self.cboMetadataField.setItemText(3, QtGui.QApplication.translate("CreateProfileBiomDlg", "taxonomy (QIIME OTU table)", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCreateProfile.setText(QtGui.QApplication.translate("CreateProfileBiomDlg", "Create STAMP profile", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCancel.setText(QtGui.QApplication.translate("CreateProfileBiomDlg", "Cancel", None, QtGui.QApplication.UnicodeUTF8))

