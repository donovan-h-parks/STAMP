# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'customizeHeadings.ui'
#
# Created: Tue Apr 26 14:23:50 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_CreateProfileDlg(object):
    def setupUi(self, CreateProfileDlg):
        CreateProfileDlg.setObjectName(_fromUtf8("CreateProfileDlg"))
        CreateProfileDlg.resize(252, 397)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/createProfile.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        CreateProfileDlg.setWindowIcon(icon)
        self.verticalLayout = QtGui.QVBoxLayout(CreateProfileDlg)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.txtInfo = QtGui.QTextEdit(CreateProfileDlg)
        self.txtInfo.setReadOnly(True)
        self.txtInfo.setAcceptRichText(False)
        self.txtInfo.setObjectName(_fromUtf8("txtInfo"))
        self.verticalLayout.addWidget(self.txtInfo)
        self.formLayout = QtGui.QFormLayout()
        self.formLayout.setObjectName(_fromUtf8("formLayout"))
        self.label = QtGui.QLabel(CreateProfileDlg)
        self.label.setObjectName(_fromUtf8("label"))
        self.formLayout.setWidget(0, QtGui.QFormLayout.LabelRole, self.label)
        self.txtLevel1 = QtGui.QLineEdit(CreateProfileDlg)
        self.txtLevel1.setObjectName(_fromUtf8("txtLevel1"))
        self.formLayout.setWidget(0, QtGui.QFormLayout.FieldRole, self.txtLevel1)
        self.label_2 = QtGui.QLabel(CreateProfileDlg)
        self.label_2.setObjectName(_fromUtf8("label_2"))
        self.formLayout.setWidget(1, QtGui.QFormLayout.LabelRole, self.label_2)
        self.txtLevel2 = QtGui.QLineEdit(CreateProfileDlg)
        self.txtLevel2.setObjectName(_fromUtf8("txtLevel2"))
        self.formLayout.setWidget(1, QtGui.QFormLayout.FieldRole, self.txtLevel2)
        self.label_3 = QtGui.QLabel(CreateProfileDlg)
        self.label_3.setObjectName(_fromUtf8("label_3"))
        self.formLayout.setWidget(2, QtGui.QFormLayout.LabelRole, self.label_3)
        self.txtLevel3 = QtGui.QLineEdit(CreateProfileDlg)
        self.txtLevel3.setObjectName(_fromUtf8("txtLevel3"))
        self.formLayout.setWidget(2, QtGui.QFormLayout.FieldRole, self.txtLevel3)
        self.label_4 = QtGui.QLabel(CreateProfileDlg)
        self.label_4.setObjectName(_fromUtf8("label_4"))
        self.formLayout.setWidget(3, QtGui.QFormLayout.LabelRole, self.label_4)
        self.txtLevel4 = QtGui.QLineEdit(CreateProfileDlg)
        self.txtLevel4.setObjectName(_fromUtf8("txtLevel4"))
        self.formLayout.setWidget(3, QtGui.QFormLayout.FieldRole, self.txtLevel4)
        self.label_5 = QtGui.QLabel(CreateProfileDlg)
        self.label_5.setObjectName(_fromUtf8("label_5"))
        self.formLayout.setWidget(4, QtGui.QFormLayout.LabelRole, self.label_5)
        self.txtLevel5 = QtGui.QLineEdit(CreateProfileDlg)
        self.txtLevel5.setObjectName(_fromUtf8("txtLevel5"))
        self.formLayout.setWidget(4, QtGui.QFormLayout.FieldRole, self.txtLevel5)
        self.label_6 = QtGui.QLabel(CreateProfileDlg)
        self.label_6.setObjectName(_fromUtf8("label_6"))
        self.formLayout.setWidget(5, QtGui.QFormLayout.LabelRole, self.label_6)
        self.txtLevel6 = QtGui.QLineEdit(CreateProfileDlg)
        self.txtLevel6.setObjectName(_fromUtf8("txtLevel6"))
        self.formLayout.setWidget(5, QtGui.QFormLayout.FieldRole, self.txtLevel6)
        self.label_7 = QtGui.QLabel(CreateProfileDlg)
        self.label_7.setObjectName(_fromUtf8("label_7"))
        self.formLayout.setWidget(6, QtGui.QFormLayout.LabelRole, self.label_7)
        self.txtLevel7 = QtGui.QLineEdit(CreateProfileDlg)
        self.txtLevel7.setObjectName(_fromUtf8("txtLevel7"))
        self.formLayout.setWidget(6, QtGui.QFormLayout.FieldRole, self.txtLevel7)
        self.label_8 = QtGui.QLabel(CreateProfileDlg)
        self.label_8.setObjectName(_fromUtf8("label_8"))
        self.formLayout.setWidget(7, QtGui.QFormLayout.LabelRole, self.label_8)
        self.txtLevel8 = QtGui.QLineEdit(CreateProfileDlg)
        self.txtLevel8.setObjectName(_fromUtf8("txtLevel8"))
        self.formLayout.setWidget(7, QtGui.QFormLayout.FieldRole, self.txtLevel8)
        self.verticalLayout.addLayout(self.formLayout)
        self.horizontalLayout = QtGui.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        spacerItem = QtGui.QSpacerItem(40, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.horizontalLayout.addItem(spacerItem)
        self.btnOK = QtGui.QPushButton(CreateProfileDlg)
        self.btnOK.setObjectName(_fromUtf8("btnOK"))
        self.horizontalLayout.addWidget(self.btnOK)
        self.btnCancel = QtGui.QPushButton(CreateProfileDlg)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.horizontalLayout.addWidget(self.btnCancel)
        self.verticalLayout.addLayout(self.horizontalLayout)

        self.retranslateUi(CreateProfileDlg)
        QtCore.QMetaObject.connectSlotsByName(CreateProfileDlg)

    def retranslateUi(self, CreateProfileDlg):
        CreateProfileDlg.setWindowTitle(QtGui.QApplication.translate("CreateProfileDlg", "Customize headings", None, QtGui.QApplication.UnicodeUTF8))
        self.txtInfo.setStyleSheet(QtGui.QApplication.translate("CreateProfileDlg", "background-color: rgb(255, 255, 235);", None, QtGui.QApplication.UnicodeUTF8))
        self.label.setText(QtGui.QApplication.translate("CreateProfileDlg", "Level 1:", None, QtGui.QApplication.UnicodeUTF8))
        self.label_2.setText(QtGui.QApplication.translate("CreateProfileDlg", "Level 2:", None, QtGui.QApplication.UnicodeUTF8))
        self.label_3.setText(QtGui.QApplication.translate("CreateProfileDlg", "Level 3:", None, QtGui.QApplication.UnicodeUTF8))
        self.label_4.setText(QtGui.QApplication.translate("CreateProfileDlg", "Level 4:", None, QtGui.QApplication.UnicodeUTF8))
        self.label_5.setText(QtGui.QApplication.translate("CreateProfileDlg", "Level 5:", None, QtGui.QApplication.UnicodeUTF8))
        self.label_6.setText(QtGui.QApplication.translate("CreateProfileDlg", "Level 6:", None, QtGui.QApplication.UnicodeUTF8))
        self.label_7.setText(QtGui.QApplication.translate("CreateProfileDlg", "Level 7:", None, QtGui.QApplication.UnicodeUTF8))
        self.label_8.setText(QtGui.QApplication.translate("CreateProfileDlg", "Level 8:", None, QtGui.QApplication.UnicodeUTF8))
        self.btnOK.setText(QtGui.QApplication.translate("CreateProfileDlg", "OK", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCancel.setText(QtGui.QApplication.translate("CreateProfileDlg", "Cancel", None, QtGui.QApplication.UnicodeUTF8))

import stamp.STAMP_rc
