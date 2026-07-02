# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'customizeHeadings.ui'
#
# Created: Tue Apr 26 14:23:50 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt6 import QtCore, QtGui, QtWidgets

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileDlg(object):
    def setupUi(self, CreateProfileDlg):
        CreateProfileDlg.setObjectName(_fromUtf8("CreateProfileDlg"))
        CreateProfileDlg.resize(252, 397)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("icons:createProfile.png")), QtGui.QIcon.Mode.Normal, QtGui.QIcon.State.Off)
        CreateProfileDlg.setWindowIcon(icon)
        self.verticalLayout = QtWidgets.QVBoxLayout(CreateProfileDlg)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.txtInfo = QtWidgets.QTextEdit(CreateProfileDlg)
        self.txtInfo.setReadOnly(True)
        self.txtInfo.setAcceptRichText(False)
        self.txtInfo.setObjectName(_fromUtf8("txtInfo"))
        self.verticalLayout.addWidget(self.txtInfo)
        self.formLayout = QtWidgets.QFormLayout()
        self.formLayout.setObjectName(_fromUtf8("formLayout"))
        self.label = QtWidgets.QLabel(CreateProfileDlg)
        self.label.setObjectName(_fromUtf8("label"))
        self.formLayout.setWidget(0, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label)
        self.txtLevel1 = QtWidgets.QLineEdit(CreateProfileDlg)
        self.txtLevel1.setObjectName(_fromUtf8("txtLevel1"))
        self.formLayout.setWidget(0, QtWidgets.QFormLayout.ItemRole.FieldRole, self.txtLevel1)
        self.label_2 = QtWidgets.QLabel(CreateProfileDlg)
        self.label_2.setObjectName(_fromUtf8("label_2"))
        self.formLayout.setWidget(1, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label_2)
        self.txtLevel2 = QtWidgets.QLineEdit(CreateProfileDlg)
        self.txtLevel2.setObjectName(_fromUtf8("txtLevel2"))
        self.formLayout.setWidget(1, QtWidgets.QFormLayout.ItemRole.FieldRole, self.txtLevel2)
        self.label_3 = QtWidgets.QLabel(CreateProfileDlg)
        self.label_3.setObjectName(_fromUtf8("label_3"))
        self.formLayout.setWidget(2, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label_3)
        self.txtLevel3 = QtWidgets.QLineEdit(CreateProfileDlg)
        self.txtLevel3.setObjectName(_fromUtf8("txtLevel3"))
        self.formLayout.setWidget(2, QtWidgets.QFormLayout.ItemRole.FieldRole, self.txtLevel3)
        self.label_4 = QtWidgets.QLabel(CreateProfileDlg)
        self.label_4.setObjectName(_fromUtf8("label_4"))
        self.formLayout.setWidget(3, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label_4)
        self.txtLevel4 = QtWidgets.QLineEdit(CreateProfileDlg)
        self.txtLevel4.setObjectName(_fromUtf8("txtLevel4"))
        self.formLayout.setWidget(3, QtWidgets.QFormLayout.ItemRole.FieldRole, self.txtLevel4)
        self.label_5 = QtWidgets.QLabel(CreateProfileDlg)
        self.label_5.setObjectName(_fromUtf8("label_5"))
        self.formLayout.setWidget(4, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label_5)
        self.txtLevel5 = QtWidgets.QLineEdit(CreateProfileDlg)
        self.txtLevel5.setObjectName(_fromUtf8("txtLevel5"))
        self.formLayout.setWidget(4, QtWidgets.QFormLayout.ItemRole.FieldRole, self.txtLevel5)
        self.label_6 = QtWidgets.QLabel(CreateProfileDlg)
        self.label_6.setObjectName(_fromUtf8("label_6"))
        self.formLayout.setWidget(5, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label_6)
        self.txtLevel6 = QtWidgets.QLineEdit(CreateProfileDlg)
        self.txtLevel6.setObjectName(_fromUtf8("txtLevel6"))
        self.formLayout.setWidget(5, QtWidgets.QFormLayout.ItemRole.FieldRole, self.txtLevel6)
        self.label_7 = QtWidgets.QLabel(CreateProfileDlg)
        self.label_7.setObjectName(_fromUtf8("label_7"))
        self.formLayout.setWidget(6, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label_7)
        self.txtLevel7 = QtWidgets.QLineEdit(CreateProfileDlg)
        self.txtLevel7.setObjectName(_fromUtf8("txtLevel7"))
        self.formLayout.setWidget(6, QtWidgets.QFormLayout.ItemRole.FieldRole, self.txtLevel7)
        self.label_8 = QtWidgets.QLabel(CreateProfileDlg)
        self.label_8.setObjectName(_fromUtf8("label_8"))
        self.formLayout.setWidget(7, QtWidgets.QFormLayout.ItemRole.LabelRole, self.label_8)
        self.txtLevel8 = QtWidgets.QLineEdit(CreateProfileDlg)
        self.txtLevel8.setObjectName(_fromUtf8("txtLevel8"))
        self.formLayout.setWidget(7, QtWidgets.QFormLayout.ItemRole.FieldRole, self.txtLevel8)
        self.verticalLayout.addLayout(self.formLayout)
        self.horizontalLayout = QtWidgets.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        spacerItem = QtWidgets.QSpacerItem(40, 20, QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Minimum)
        self.horizontalLayout.addItem(spacerItem)
        self.btnOK = QtWidgets.QPushButton(CreateProfileDlg)
        self.btnOK.setObjectName(_fromUtf8("btnOK"))
        self.horizontalLayout.addWidget(self.btnOK)
        self.btnCancel = QtWidgets.QPushButton(CreateProfileDlg)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.horizontalLayout.addWidget(self.btnCancel)
        self.verticalLayout.addLayout(self.horizontalLayout)

        self.retranslateUi(CreateProfileDlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileDlg)

    def retranslateUi(self, CreateProfileDlg):
        CreateProfileDlg.setWindowTitle(QtWidgets.QApplication.translate("CreateProfileDlg", "Customize headings", None))
        self.txtInfo.setStyleSheet(QtWidgets.QApplication.translate("CreateProfileDlg", "background-color: rgb(255, 255, 235);", None))
        self.label.setText(QtWidgets.QApplication.translate("CreateProfileDlg", "Level 1:", None))
        self.label_2.setText(QtWidgets.QApplication.translate("CreateProfileDlg", "Level 2:", None))
        self.label_3.setText(QtWidgets.QApplication.translate("CreateProfileDlg", "Level 3:", None))
        self.label_4.setText(QtWidgets.QApplication.translate("CreateProfileDlg", "Level 4:", None))
        self.label_5.setText(QtWidgets.QApplication.translate("CreateProfileDlg", "Level 5:", None))
        self.label_6.setText(QtWidgets.QApplication.translate("CreateProfileDlg", "Level 6:", None))
        self.label_7.setText(QtWidgets.QApplication.translate("CreateProfileDlg", "Level 7:", None))
        self.label_8.setText(QtWidgets.QApplication.translate("CreateProfileDlg", "Level 8:", None))
        self.btnOK.setText(QtWidgets.QApplication.translate("CreateProfileDlg", "OK", None))
        self.btnCancel.setText(QtWidgets.QApplication.translate("CreateProfileDlg", "Cancel", None))

import stamp.STAMP_rc
