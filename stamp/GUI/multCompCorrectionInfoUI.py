# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'multCompCorrectionInfoDlg.ui'
#
# Created: Tue Apr 26 14:24:33 2011
#      by: PyQt4 UI code generator 4.8.2
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_multCompCorrectionInfoDlg(object):
    def setupUi(self, multCompCorrectionInfoDlg):
        multCompCorrectionInfoDlg.setObjectName(_fromUtf8("multCompCorrectionInfoDlg"))
        multCompCorrectionInfoDlg.resize(400, 170)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/programIcon.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        multCompCorrectionInfoDlg.setWindowIcon(icon)
        self.verticalLayout = QtGui.QVBoxLayout(multCompCorrectionInfoDlg)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.layout = QtGui.QFormLayout()
        self.layout.setFieldGrowthPolicy(QtGui.QFormLayout.AllNonFixedFieldsGrow)
        self.layout.setObjectName(_fromUtf8("layout"))
        self.verticalLayout.addLayout(self.layout)

        self.retranslateUi(multCompCorrectionInfoDlg)
        QtCore.QMetaObject.connectSlotsByName(multCompCorrectionInfoDlg)

    def retranslateUi(self, multCompCorrectionInfoDlg):
        multCompCorrectionInfoDlg.setWindowTitle(QtGui.QApplication.translate("multCompCorrectionInfoDlg", "Additional info", None, QtGui.QApplication.UnicodeUTF8))

import stamp.STAMP_rc
