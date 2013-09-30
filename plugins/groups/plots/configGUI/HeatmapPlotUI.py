# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'HeatmapPlot.ui'
#
# Created: Fri May 24 14:46:38 2013
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

class Ui_BoxPlotDialog(object):
    def setupUi(self, BoxPlotDialog):
        BoxPlotDialog.setObjectName(_fromUtf8("BoxPlotDialog"))
        BoxPlotDialog.resize(282, 158)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Fixed, QtGui.QSizePolicy.Fixed)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(BoxPlotDialog.sizePolicy().hasHeightForWidth())
        BoxPlotDialog.setSizePolicy(sizePolicy)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8("../../../../../icons/programIcon.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        BoxPlotDialog.setWindowIcon(icon)
        self.verticalLayout = QtGui.QVBoxLayout(BoxPlotDialog)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.horizontalLayout = QtGui.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.lblFieldToPlot = QtGui.QLabel(BoxPlotDialog)
        self.lblFieldToPlot.setObjectName(_fromUtf8("lblFieldToPlot"))
        self.horizontalLayout.addWidget(self.lblFieldToPlot)
        self.cboFieldToPlot = QtGui.QComboBox(BoxPlotDialog)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Preferred, QtGui.QSizePolicy.Fixed)
        sizePolicy.setHorizontalStretch(1)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.cboFieldToPlot.sizePolicy().hasHeightForWidth())
        self.cboFieldToPlot.setSizePolicy(sizePolicy)
        self.cboFieldToPlot.setObjectName(_fromUtf8("cboFieldToPlot"))
        self.cboFieldToPlot.addItem(_fromUtf8(""))
        self.cboFieldToPlot.addItem(_fromUtf8(""))
        self.horizontalLayout.addWidget(self.cboFieldToPlot)
        self.verticalLayout.addLayout(self.horizontalLayout)
        self.groupBox_3 = QtGui.QGroupBox(BoxPlotDialog)
        self.groupBox_3.setObjectName(_fromUtf8("groupBox_3"))
        self.verticalLayout_6 = QtGui.QVBoxLayout(self.groupBox_3)
        self.verticalLayout_6.setObjectName(_fromUtf8("verticalLayout_6"))
        self.formLayout_2 = QtGui.QFormLayout()
        self.formLayout_2.setFieldGrowthPolicy(QtGui.QFormLayout.AllNonFixedFieldsGrow)
        self.formLayout_2.setObjectName(_fromUtf8("formLayout_2"))
        self.spinFigWidth = QtGui.QDoubleSpinBox(self.groupBox_3)
        self.spinFigWidth.setDecimals(2)
        self.spinFigWidth.setMinimum(0.1)
        self.spinFigWidth.setMaximum(20.0)
        self.spinFigWidth.setSingleStep(0.05)
        self.spinFigWidth.setProperty("value", 6.0)
        self.spinFigWidth.setObjectName(_fromUtf8("spinFigWidth"))
        self.formLayout_2.setWidget(0, QtGui.QFormLayout.FieldRole, self.spinFigWidth)
        self.lblFigureHeight = QtGui.QLabel(self.groupBox_3)
        self.lblFigureHeight.setObjectName(_fromUtf8("lblFigureHeight"))
        self.formLayout_2.setWidget(1, QtGui.QFormLayout.LabelRole, self.lblFigureHeight)
        self.spinFigHeight = QtGui.QDoubleSpinBox(self.groupBox_3)
        self.spinFigHeight.setMinimum(0.1)
        self.spinFigHeight.setMaximum(20.0)
        self.spinFigHeight.setSingleStep(0.05)
        self.spinFigHeight.setProperty("value", 6.0)
        self.spinFigHeight.setObjectName(_fromUtf8("spinFigHeight"))
        self.formLayout_2.setWidget(1, QtGui.QFormLayout.FieldRole, self.spinFigHeight)
        self.lblFigureWidth = QtGui.QLabel(self.groupBox_3)
        self.lblFigureWidth.setObjectName(_fromUtf8("lblFigureWidth"))
        self.formLayout_2.setWidget(0, QtGui.QFormLayout.LabelRole, self.lblFigureWidth)
        self.verticalLayout_6.addLayout(self.formLayout_2)
        self.verticalLayout.addWidget(self.groupBox_3)
        self.horizontalLayout_5 = QtGui.QHBoxLayout()
        self.horizontalLayout_5.setObjectName(_fromUtf8("horizontalLayout_5"))
        spacerItem = QtGui.QSpacerItem(100, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.horizontalLayout_5.addItem(spacerItem)
        self.buttonBox = QtGui.QDialogButtonBox(BoxPlotDialog)
        self.buttonBox.setOrientation(QtCore.Qt.Horizontal)
        self.buttonBox.setStandardButtons(QtGui.QDialogButtonBox.Cancel|QtGui.QDialogButtonBox.Ok)
        self.buttonBox.setCenterButtons(False)
        self.buttonBox.setObjectName(_fromUtf8("buttonBox"))
        self.horizontalLayout_5.addWidget(self.buttonBox)
        self.verticalLayout.addLayout(self.horizontalLayout_5)

        self.retranslateUi(BoxPlotDialog)
        QtCore.QObject.connect(self.buttonBox, QtCore.SIGNAL(_fromUtf8("accepted()")), BoxPlotDialog.accept)
        QtCore.QObject.connect(self.buttonBox, QtCore.SIGNAL(_fromUtf8("rejected()")), BoxPlotDialog.reject)
        QtCore.QMetaObject.connectSlotsByName(BoxPlotDialog)

    def retranslateUi(self, BoxPlotDialog):
        BoxPlotDialog.setWindowTitle(_translate("BoxPlotDialog", "Box plot", None))
        self.lblFieldToPlot.setText(_translate("BoxPlotDialog", "Field to plot:", None))
        self.cboFieldToPlot.setItemText(0, _translate("BoxPlotDialog", "Number of sequences", None))
        self.cboFieldToPlot.setItemText(1, _translate("BoxPlotDialog", "Proportion of sequences (%)", None))
        self.groupBox_3.setTitle(_translate("BoxPlotDialog", "Figure size", None))
        self.lblFigureHeight.setText(_translate("BoxPlotDialog", "Height:", None))
        self.lblFigureWidth.setText(_translate("BoxPlotDialog", "Width", None))

