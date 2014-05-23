# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'assignCOG.ui'
#
# Created: Tue Apr 26 14:23:30 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_AssignCOGsDlg(object):
    def setupUi(self, AssignCOGsDlg):
        AssignCOGsDlg.setObjectName(_fromUtf8("AssignCOGsDlg"))
        AssignCOGsDlg.resize(357, 102)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/appendCOGs.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        AssignCOGsDlg.setWindowIcon(icon)
        self.verticalLayout = QtGui.QVBoxLayout(AssignCOGsDlg)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.horizontalLayout = QtGui.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.txtInputProfile = QtGui.QLineEdit(AssignCOGsDlg)
        self.txtInputProfile.setReadOnly(True)
        self.txtInputProfile.setObjectName(_fromUtf8("txtInputProfile"))
        self.horizontalLayout.addWidget(self.txtInputProfile)
        self.btnLoadProfiles = QtGui.QPushButton(AssignCOGsDlg)
        self.btnLoadProfiles.setObjectName(_fromUtf8("btnLoadProfiles"))
        self.horizontalLayout.addWidget(self.btnLoadProfiles)
        self.verticalLayout.addLayout(self.horizontalLayout)
        self.horizontalLayout_3 = QtGui.QHBoxLayout()
        self.horizontalLayout_3.setObjectName(_fromUtf8("horizontalLayout_3"))
        spacerItem = QtGui.QSpacerItem(40, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.horizontalLayout_3.addItem(spacerItem)
        self.label = QtGui.QLabel(AssignCOGsDlg)
        self.label.setObjectName(_fromUtf8("label"))
        self.horizontalLayout_3.addWidget(self.label)
        self.cboMultiCogTreatment = QtGui.QComboBox(AssignCOGsDlg)
        self.cboMultiCogTreatment.setObjectName(_fromUtf8("cboMultiCogTreatment"))
        self.cboMultiCogTreatment.addItem(_fromUtf8(""))
        self.cboMultiCogTreatment.addItem(_fromUtf8(""))
        self.horizontalLayout_3.addWidget(self.cboMultiCogTreatment)
        self.verticalLayout.addLayout(self.horizontalLayout_3)
        self.horizontalLayout_2 = QtGui.QHBoxLayout()
        self.horizontalLayout_2.setObjectName(_fromUtf8("horizontalLayout_2"))
        spacerItem1 = QtGui.QSpacerItem(40, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.horizontalLayout_2.addItem(spacerItem1)
        self.btnCreateProfile = QtGui.QPushButton(AssignCOGsDlg)
        self.btnCreateProfile.setEnabled(False)
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.horizontalLayout_2.addWidget(self.btnCreateProfile)
        self.btnCancel = QtGui.QPushButton(AssignCOGsDlg)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.horizontalLayout_2.addWidget(self.btnCancel)
        self.verticalLayout.addLayout(self.horizontalLayout_2)

        self.retranslateUi(AssignCOGsDlg)
        QtCore.QMetaObject.connectSlotsByName(AssignCOGsDlg)

    def retranslateUi(self, AssignCOGsDlg):
        AssignCOGsDlg.setWindowTitle(QtGui.QApplication.translate("AssignCOGsDlg", "Assign COG categories", None, QtGui.QApplication.UnicodeUTF8))
        self.btnLoadProfiles.setText(QtGui.QApplication.translate("AssignCOGsDlg", "Load profiles", None, QtGui.QApplication.UnicodeUTF8))
        self.label.setText(QtGui.QApplication.translate("AssignCOGsDlg", "Multi-code COG treatment:", None, QtGui.QApplication.UnicodeUTF8))
        self.cboMultiCogTreatment.setItemText(0, QtGui.QApplication.translate("AssignCOGsDlg", "Assign sequence to each COG code", None, QtGui.QApplication.UnicodeUTF8))
        self.cboMultiCogTreatment.setItemText(1, QtGui.QApplication.translate("AssignCOGsDlg", "Treat multi-code COGs as features", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCreateProfile.setText(QtGui.QApplication.translate("AssignCOGsDlg", "Create STAMP profile", None, QtGui.QApplication.UnicodeUTF8))
        self.btnCancel.setText(QtGui.QApplication.translate("AssignCOGsDlg", "Cancel", None, QtGui.QApplication.UnicodeUTF8))

import stamp.STAMP_rc
