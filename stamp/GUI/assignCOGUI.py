# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'assignCOG.ui'
#
# Created: Tue Apr 26 14:23:30 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt6 import QtCore, QtGui, QtWidgets

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_AssignCOGsDlg(object):
    def setupUi(self, AssignCOGsDlg):
        AssignCOGsDlg.setObjectName(_fromUtf8("AssignCOGsDlg"))
        AssignCOGsDlg.resize(357, 102)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("icons:appendCOGs.png")), QtGui.QIcon.Mode.Normal, QtGui.QIcon.State.Off)
        AssignCOGsDlg.setWindowIcon(icon)
        self.verticalLayout = QtWidgets.QVBoxLayout(AssignCOGsDlg)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.horizontalLayout = QtWidgets.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.txtInputProfile = QtWidgets.QLineEdit(AssignCOGsDlg)
        self.txtInputProfile.setReadOnly(True)
        self.txtInputProfile.setObjectName(_fromUtf8("txtInputProfile"))
        self.horizontalLayout.addWidget(self.txtInputProfile)
        self.btnLoadProfiles = QtWidgets.QPushButton(AssignCOGsDlg)
        self.btnLoadProfiles.setObjectName(_fromUtf8("btnLoadProfiles"))
        self.horizontalLayout.addWidget(self.btnLoadProfiles)
        self.verticalLayout.addLayout(self.horizontalLayout)
        self.horizontalLayout_3 = QtWidgets.QHBoxLayout()
        self.horizontalLayout_3.setObjectName(_fromUtf8("horizontalLayout_3"))
        spacerItem = QtWidgets.QSpacerItem(40, 20, QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Minimum)
        self.horizontalLayout_3.addItem(spacerItem)
        self.label = QtWidgets.QLabel(AssignCOGsDlg)
        self.label.setObjectName(_fromUtf8("label"))
        self.horizontalLayout_3.addWidget(self.label)
        self.cboMultiCogTreatment = QtWidgets.QComboBox(AssignCOGsDlg)
        self.cboMultiCogTreatment.setObjectName(_fromUtf8("cboMultiCogTreatment"))
        self.cboMultiCogTreatment.addItem(_fromUtf8(""))
        self.cboMultiCogTreatment.addItem(_fromUtf8(""))
        self.horizontalLayout_3.addWidget(self.cboMultiCogTreatment)
        self.verticalLayout.addLayout(self.horizontalLayout_3)
        self.horizontalLayout_2 = QtWidgets.QHBoxLayout()
        self.horizontalLayout_2.setObjectName(_fromUtf8("horizontalLayout_2"))
        spacerItem1 = QtWidgets.QSpacerItem(40, 20, QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Minimum)
        self.horizontalLayout_2.addItem(spacerItem1)
        self.btnCreateProfile = QtWidgets.QPushButton(AssignCOGsDlg)
        self.btnCreateProfile.setEnabled(False)
        self.btnCreateProfile.setObjectName(_fromUtf8("btnCreateProfile"))
        self.horizontalLayout_2.addWidget(self.btnCreateProfile)
        self.btnCancel = QtWidgets.QPushButton(AssignCOGsDlg)
        self.btnCancel.setObjectName(_fromUtf8("btnCancel"))
        self.horizontalLayout_2.addWidget(self.btnCancel)
        self.verticalLayout.addLayout(self.horizontalLayout_2)

        self.retranslateUi(AssignCOGsDlg)
        QtCore.QMetaObject.connectSlotsByName(AssignCOGsDlg)

    def retranslateUi(self, AssignCOGsDlg):
        AssignCOGsDlg.setWindowTitle(QtWidgets.QApplication.translate("AssignCOGsDlg", "Assign COG categories", None))
        self.btnLoadProfiles.setText(QtWidgets.QApplication.translate("AssignCOGsDlg", "Load profiles", None))
        self.label.setText(QtWidgets.QApplication.translate("AssignCOGsDlg", "Multi-code COG treatment:", None))
        self.cboMultiCogTreatment.setItemText(0, QtWidgets.QApplication.translate("AssignCOGsDlg", "Assign sequence to each COG code", None))
        self.cboMultiCogTreatment.setItemText(1, QtWidgets.QApplication.translate("AssignCOGsDlg", "Treat multi-code COGs as features", None))
        self.btnCreateProfile.setText(QtWidgets.QApplication.translate("AssignCOGsDlg", "Create STAMP profile", None))
        self.btnCancel.setText(QtWidgets.QApplication.translate("AssignCOGsDlg", "Cancel", None))

import stamp.STAMP_rc
