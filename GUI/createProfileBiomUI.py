# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'createProfileBiom.ui'
#
# Created: Wed Mar 05 08:53:03 2014
#      by: PyQt4 UI code generator 4.9.6
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    def _fromUtf8(s):
        return s

try:
    _encoding = QtGui.QApplication.UnicodeUTF8
    def _translate(context, text, disambig):
        return QtGui.QApplication.translate(context, text, disambig, _encoding)
except AttributeError:
    def _translate(context, text, disambig):
        return QtGui.QApplication.translate(context, text, disambig)

class Ui_CreateProfileBiomDlg(object):
    def setupUi(self, CreateProfileBiomDlg):
        CreateProfileBiomDlg.setObjectName(_fromUtf8("CreateProfileBiomDlg"))
        CreateProfileBiomDlg.resize(396, 107)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("../icons/programIcon.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
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
        CreateProfileBiomDlg.setWindowTitle(_translate("CreateProfileBiomDlg", "Create profile", None))
        self.lblTaxonomyFile.setText(_translate("CreateProfileBiomDlg", "BIOM file:", None))
        self.btnBiomFile.setText(_translate("CreateProfileBiomDlg", "Load", None))
        self.label_2.setText(_translate("CreateProfileBiomDlg", "Metadata field:", None))
        self.cboMetadataField.setItemText(0, _translate("CreateProfileBiomDlg", "taxonomy", None))
        self.cboMetadataField.setItemText(1, _translate("CreateProfileBiomDlg", "KEGG_Pathways", None))
        self.cboMetadataField.setItemText(2, _translate("CreateProfileBiomDlg", "COG_Category", None))
        self.btnCreateProfile.setText(_translate("CreateProfileBiomDlg", "Create STAMP profile", None))
        self.btnCancel.setText(_translate("CreateProfileBiomDlg", "Cancel", None))

