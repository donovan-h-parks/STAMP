# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'multCompCorrection.ui'
#
# Created: Sat Apr 16 13:41:52 2011
#      by: PyQt4 UI code generator 4.6.2
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

class Ui_MultCompCorrectionDialog(object):
    def setupUi(self, MultCompCorrectionDialog):
        MultCompCorrectionDialog.setObjectName("MultCompCorrectionDialog")
        MultCompCorrectionDialog.resize(716, 162)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Fixed, QtGui.QSizePolicy.Fixed)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(MultCompCorrectionDialog.sizePolicy().hasHeightForWidth())
        MultCompCorrectionDialog.setSizePolicy(sizePolicy)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(":/icons/icons/programIcon.png"), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        MultCompCorrectionDialog.setWindowIcon(icon)
        self.verticalLayout_3 = QtGui.QVBoxLayout(MultCompCorrectionDialog)
        self.verticalLayout_3.setObjectName("verticalLayout_3")
        self.horizontalLayout_5 = QtGui.QHBoxLayout()
        self.horizontalLayout_5.setObjectName("horizontalLayout_5")
        self.groupBox_3 = QtGui.QGroupBox(MultCompCorrectionDialog)
        self.groupBox_3.setObjectName("groupBox_3")
        self.horizontalLayout_6 = QtGui.QHBoxLayout(self.groupBox_3)
        self.horizontalLayout_6.setObjectName("horizontalLayout_6")
        self.formLayout_2 = QtGui.QFormLayout()
        self.formLayout_2.setObjectName("formLayout_2")
        self.lblFigureWidth = QtGui.QLabel(self.groupBox_3)
        self.lblFigureWidth.setObjectName("lblFigureWidth")
        self.formLayout_2.setWidget(0, QtGui.QFormLayout.LabelRole, self.lblFigureWidth)
        self.spinFigWidth = QtGui.QDoubleSpinBox(self.groupBox_3)
        self.spinFigWidth.setDecimals(2)
        self.spinFigWidth.setMinimum(2.0)
        self.spinFigWidth.setMaximum(20.0)
        self.spinFigWidth.setSingleStep(0.5)
        self.spinFigWidth.setProperty("value", 6.5)
        self.spinFigWidth.setObjectName("spinFigWidth")
        self.formLayout_2.setWidget(0, QtGui.QFormLayout.FieldRole, self.spinFigWidth)
        self.lblFigureHeight = QtGui.QLabel(self.groupBox_3)
        self.lblFigureHeight.setObjectName("lblFigureHeight")
        self.formLayout_2.setWidget(1, QtGui.QFormLayout.LabelRole, self.lblFigureHeight)
        self.spinFigHeight = QtGui.QDoubleSpinBox(self.groupBox_3)
        self.spinFigHeight.setMinimum(2.0)
        self.spinFigHeight.setMaximum(12.0)
        self.spinFigHeight.setSingleStep(0.5)
        self.spinFigHeight.setProperty("value", 6.5)
        self.spinFigHeight.setObjectName("spinFigHeight")
        self.formLayout_2.setWidget(1, QtGui.QFormLayout.FieldRole, self.spinFigHeight)
        self.horizontalLayout_6.addLayout(self.formLayout_2)
        self.horizontalLayout_5.addWidget(self.groupBox_3)
        self.groupBox_8 = QtGui.QGroupBox(MultCompCorrectionDialog)
        self.groupBox_8.setObjectName("groupBox_8")
        self.verticalLayout_4 = QtGui.QVBoxLayout(self.groupBox_8)
        self.verticalLayout_4.setObjectName("verticalLayout_4")
        self.formLayout = QtGui.QFormLayout()
        self.formLayout.setObjectName("formLayout")
        self.lblBinWidth = QtGui.QLabel(self.groupBox_8)
        self.lblBinWidth.setObjectName("lblBinWidth")
        self.formLayout.setWidget(0, QtGui.QFormLayout.LabelRole, self.lblBinWidth)
        self.horizontalLayout_9 = QtGui.QHBoxLayout()
        self.horizontalLayout_9.setObjectName("horizontalLayout_9")
        self.spinBinWidth = QtGui.QDoubleSpinBox(self.groupBox_8)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Minimum, QtGui.QSizePolicy.Fixed)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.spinBinWidth.sizePolicy().hasHeightForWidth())
        self.spinBinWidth.setSizePolicy(sizePolicy)
        self.spinBinWidth.setDecimals(4)
        self.spinBinWidth.setMinimum(0.0001)
        self.spinBinWidth.setMaximum(10000.0)
        self.spinBinWidth.setSingleStep(0.0001)
        self.spinBinWidth.setProperty("value", 0.002)
        self.spinBinWidth.setObjectName("spinBinWidth")
        self.horizontalLayout_9.addWidget(self.spinBinWidth)
        spacerItem = QtGui.QSpacerItem(1, 20, QtGui.QSizePolicy.Minimum, QtGui.QSizePolicy.Minimum)
        self.horizontalLayout_9.addItem(spacerItem)
        self.formLayout.setLayout(0, QtGui.QFormLayout.FieldRole, self.horizontalLayout_9)
        self.label = QtGui.QLabel(self.groupBox_8)
        self.label.setObjectName("label")
        self.formLayout.setWidget(1, QtGui.QFormLayout.LabelRole, self.label)
        self.horizontalLayout_7 = QtGui.QHBoxLayout()
        self.horizontalLayout_7.setObjectName("horizontalLayout_7")
        self.spinXlimitFig1 = QtGui.QDoubleSpinBox(self.groupBox_8)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Minimum, QtGui.QSizePolicy.Fixed)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.spinXlimitFig1.sizePolicy().hasHeightForWidth())
        self.spinXlimitFig1.setSizePolicy(sizePolicy)
        self.spinXlimitFig1.setDecimals(4)
        self.spinXlimitFig1.setMinimum(0.0001)
        self.spinXlimitFig1.setMaximum(10000.0)
        self.spinXlimitFig1.setSingleStep(0.01)
        self.spinXlimitFig1.setProperty("value", 0.05)
        self.spinXlimitFig1.setObjectName("spinXlimitFig1")
        self.horizontalLayout_7.addWidget(self.spinXlimitFig1)
        self.btnXmaxFig1 = QtGui.QPushButton(self.groupBox_8)
        self.btnXmaxFig1.setObjectName("btnXmaxFig1")
        self.horizontalLayout_7.addWidget(self.btnXmaxFig1)
        self.formLayout.setLayout(1, QtGui.QFormLayout.FieldRole, self.horizontalLayout_7)
        self.verticalLayout_4.addLayout(self.formLayout)
        self.chkLogScale = QtGui.QCheckBox(self.groupBox_8)
        self.chkLogScale.setObjectName("chkLogScale")
        self.verticalLayout_4.addWidget(self.chkLogScale)
        self.horizontalLayout_5.addWidget(self.groupBox_8)
        self.groupBox_2 = QtGui.QGroupBox(MultCompCorrectionDialog)
        self.groupBox_2.setObjectName("groupBox_2")
        self.verticalLayout_2 = QtGui.QVBoxLayout(self.groupBox_2)
        self.verticalLayout_2.setObjectName("verticalLayout_2")
        self.horizontalLayout_4 = QtGui.QHBoxLayout()
        self.horizontalLayout_4.setObjectName("horizontalLayout_4")
        self.label_3 = QtGui.QLabel(self.groupBox_2)
        self.label_3.setObjectName("label_3")
        self.horizontalLayout_4.addWidget(self.label_3)
        self.spinXlimitFig2 = QtGui.QDoubleSpinBox(self.groupBox_2)
        self.spinXlimitFig2.setDecimals(4)
        self.spinXlimitFig2.setMinimum(0.0001)
        self.spinXlimitFig2.setMaximum(10000.0)
        self.spinXlimitFig2.setSingleStep(0.01)
        self.spinXlimitFig2.setProperty("value", 0.05)
        self.spinXlimitFig2.setObjectName("spinXlimitFig2")
        self.horizontalLayout_4.addWidget(self.spinXlimitFig2)
        self.verticalLayout_2.addLayout(self.horizontalLayout_4)
        self.horizontalLayout_2 = QtGui.QHBoxLayout()
        self.horizontalLayout_2.setObjectName("horizontalLayout_2")
        spacerItem1 = QtGui.QSpacerItem(40, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.horizontalLayout_2.addItem(spacerItem1)
        self.btnXmaxFig2 = QtGui.QPushButton(self.groupBox_2)
        self.btnXmaxFig2.setObjectName("btnXmaxFig2")
        self.horizontalLayout_2.addWidget(self.btnXmaxFig2)
        self.verticalLayout_2.addLayout(self.horizontalLayout_2)
        self.horizontalLayout_5.addWidget(self.groupBox_2)
        self.groupBox = QtGui.QGroupBox(MultCompCorrectionDialog)
        self.groupBox.setObjectName("groupBox")
        self.verticalLayout = QtGui.QVBoxLayout(self.groupBox)
        self.verticalLayout.setObjectName("verticalLayout")
        self.horizontalLayout = QtGui.QHBoxLayout()
        self.horizontalLayout.setObjectName("horizontalLayout")
        self.label_2 = QtGui.QLabel(self.groupBox)
        self.label_2.setObjectName("label_2")
        self.horizontalLayout.addWidget(self.label_2)
        self.spinXlimitFig3 = QtGui.QDoubleSpinBox(self.groupBox)
        self.spinXlimitFig3.setDecimals(4)
        self.spinXlimitFig3.setMinimum(0.0001)
        self.spinXlimitFig3.setMaximum(10000.0)
        self.spinXlimitFig3.setSingleStep(0.01)
        self.spinXlimitFig3.setProperty("value", 0.05)
        self.spinXlimitFig3.setObjectName("spinXlimitFig3")
        self.horizontalLayout.addWidget(self.spinXlimitFig3)
        self.verticalLayout.addLayout(self.horizontalLayout)
        self.horizontalLayout_8 = QtGui.QHBoxLayout()
        self.horizontalLayout_8.setObjectName("horizontalLayout_8")
        spacerItem2 = QtGui.QSpacerItem(40, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.horizontalLayout_8.addItem(spacerItem2)
        self.btnXmaxFig3 = QtGui.QPushButton(self.groupBox)
        self.btnXmaxFig3.setObjectName("btnXmaxFig3")
        self.horizontalLayout_8.addWidget(self.btnXmaxFig3)
        self.verticalLayout.addLayout(self.horizontalLayout_8)
        self.horizontalLayout_5.addWidget(self.groupBox)
        self.verticalLayout_3.addLayout(self.horizontalLayout_5)
        self.horizontalLayout_3 = QtGui.QHBoxLayout()
        self.horizontalLayout_3.setObjectName("horizontalLayout_3")
        self.buttonBox = QtGui.QDialogButtonBox(MultCompCorrectionDialog)
        self.buttonBox.setOrientation(QtCore.Qt.Horizontal)
        self.buttonBox.setStandardButtons(QtGui.QDialogButtonBox.Cancel|QtGui.QDialogButtonBox.Ok)
        self.buttonBox.setCenterButtons(False)
        self.buttonBox.setObjectName("buttonBox")
        self.horizontalLayout_3.addWidget(self.buttonBox)
        self.verticalLayout_3.addLayout(self.horizontalLayout_3)

        self.retranslateUi(MultCompCorrectionDialog)
        QtCore.QObject.connect(self.buttonBox, QtCore.SIGNAL("accepted()"), MultCompCorrectionDialog.accept)
        QtCore.QObject.connect(self.buttonBox, QtCore.SIGNAL("rejected()"), MultCompCorrectionDialog.reject)
        QtCore.QMetaObject.connectSlotsByName(MultCompCorrectionDialog)

    def retranslateUi(self, MultCompCorrectionDialog):
        MultCompCorrectionDialog.setWindowTitle(QtGui.QApplication.translate("MultCompCorrectionDialog", "Multiple comparison plots", None, QtGui.QApplication.UnicodeUTF8))
        self.groupBox_3.setTitle(QtGui.QApplication.translate("MultCompCorrectionDialog", "Main figure size", None, QtGui.QApplication.UnicodeUTF8))
        self.lblFigureWidth.setText(QtGui.QApplication.translate("MultCompCorrectionDialog", "Width:", None, QtGui.QApplication.UnicodeUTF8))
        self.lblFigureHeight.setText(QtGui.QApplication.translate("MultCompCorrectionDialog", "Height:", None, QtGui.QApplication.UnicodeUTF8))
        self.groupBox_8.setTitle(QtGui.QApplication.translate("MultCompCorrectionDialog", "Histogram plot", None, QtGui.QApplication.UnicodeUTF8))
        self.lblBinWidth.setText(QtGui.QApplication.translate("MultCompCorrectionDialog", "Bin width:", None, QtGui.QApplication.UnicodeUTF8))
        self.label.setText(QtGui.QApplication.translate("MultCompCorrectionDialog", "x-axis limit:", None, QtGui.QApplication.UnicodeUTF8))
        self.btnXmaxFig1.setText(QtGui.QApplication.translate("MultCompCorrectionDialog", "Max", None, QtGui.QApplication.UnicodeUTF8))
        self.chkLogScale.setText(QtGui.QApplication.translate("MultCompCorrectionDialog", "Show y-axis as log scale", None, QtGui.QApplication.UnicodeUTF8))
        self.groupBox_2.setTitle(QtGui.QApplication.translate("MultCompCorrectionDialog", "Correction plot", None, QtGui.QApplication.UnicodeUTF8))
        self.label_3.setText(QtGui.QApplication.translate("MultCompCorrectionDialog", "x-axis limit:", None, QtGui.QApplication.UnicodeUTF8))
        self.btnXmaxFig2.setText(QtGui.QApplication.translate("MultCompCorrectionDialog", "Max", None, QtGui.QApplication.UnicodeUTF8))
        self.groupBox.setTitle(QtGui.QApplication.translate("MultCompCorrectionDialog", "Significant features plot", None, QtGui.QApplication.UnicodeUTF8))
        self.label_2.setText(QtGui.QApplication.translate("MultCompCorrectionDialog", "x-axis limit:", None, QtGui.QApplication.UnicodeUTF8))
        self.btnXmaxFig3.setText(QtGui.QApplication.translate("MultCompCorrectionDialog", "Max", None, QtGui.QApplication.UnicodeUTF8))

