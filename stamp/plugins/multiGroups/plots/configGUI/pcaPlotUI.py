# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'pcaPlot.ui'
#
# Created: Fri Nov 30 12:51:51 2012
#      by: PyQt4 UI code generator 4.9.5
#
# WARNING! All changes made in this file will be lost!

from PyQt4 import QtCore, QtGui

try:
    _fromUtf8 = QtCore.QString.fromUtf8
except AttributeError:
    _fromUtf8 = lambda s: s

class Ui_PcaPlotDialog(object):
    def setupUi(self, PcaPlotDialog):
        PcaPlotDialog.setObjectName(_fromUtf8("PcaPlotDialog"))
        PcaPlotDialog.resize(334, 306)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Fixed, QtGui.QSizePolicy.Fixed)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(PcaPlotDialog.sizePolicy().hasHeightForWidth())
        PcaPlotDialog.setSizePolicy(sizePolicy)
        icon = QtGui.QIcon()
        icon.addPixmap(QtGui.QPixmap(_fromUtf8(":/icons/icons/programIcon.png")), QtGui.QIcon.Normal, QtGui.QIcon.Off)
        PcaPlotDialog.setWindowIcon(icon)
        self.verticalLayout_3 = QtGui.QVBoxLayout(PcaPlotDialog)
        self.verticalLayout_3.setObjectName(_fromUtf8("verticalLayout_3"))
        self.groupBox_3 = QtGui.QGroupBox(PcaPlotDialog)
        self.groupBox_3.setObjectName(_fromUtf8("groupBox_3"))
        self.verticalLayout = QtGui.QVBoxLayout(self.groupBox_3)
        self.verticalLayout.setObjectName(_fromUtf8("verticalLayout"))
        self.chkFixedPixelsPerUnitDistance = QtGui.QCheckBox(self.groupBox_3)
        self.chkFixedPixelsPerUnitDistance.setChecked(True)
        self.chkFixedPixelsPerUnitDistance.setObjectName(_fromUtf8("chkFixedPixelsPerUnitDistance"))
        self.verticalLayout.addWidget(self.chkFixedPixelsPerUnitDistance)
        self.horizontalLayout = QtGui.QHBoxLayout()
        self.horizontalLayout.setObjectName(_fromUtf8("horizontalLayout"))
        self.lblFigureWidth = QtGui.QLabel(self.groupBox_3)
        self.lblFigureWidth.setObjectName(_fromUtf8("lblFigureWidth"))
        self.horizontalLayout.addWidget(self.lblFigureWidth)
        self.spinFigWidth = QtGui.QDoubleSpinBox(self.groupBox_3)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Minimum, QtGui.QSizePolicy.Fixed)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.spinFigWidth.sizePolicy().hasHeightForWidth())
        self.spinFigWidth.setSizePolicy(sizePolicy)
        self.spinFigWidth.setDecimals(2)
        self.spinFigWidth.setMinimum(0.5)
        self.spinFigWidth.setMaximum(20.0)
        self.spinFigWidth.setSingleStep(0.5)
        self.spinFigWidth.setProperty("value", 7.0)
        self.spinFigWidth.setObjectName(_fromUtf8("spinFigWidth"))
        self.horizontalLayout.addWidget(self.spinFigWidth)
        self.lblFigureHeight = QtGui.QLabel(self.groupBox_3)
        self.lblFigureHeight.setObjectName(_fromUtf8("lblFigureHeight"))
        self.horizontalLayout.addWidget(self.lblFigureHeight)
        self.spinFigHeight = QtGui.QDoubleSpinBox(self.groupBox_3)
        self.spinFigHeight.setEnabled(False)
        sizePolicy = QtGui.QSizePolicy(QtGui.QSizePolicy.Minimum, QtGui.QSizePolicy.Fixed)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.spinFigHeight.sizePolicy().hasHeightForWidth())
        self.spinFigHeight.setSizePolicy(sizePolicy)
        self.spinFigHeight.setMinimum(0.5)
        self.spinFigHeight.setMaximum(20.0)
        self.spinFigHeight.setSingleStep(0.05)
        self.spinFigHeight.setProperty("value", 6.0)
        self.spinFigHeight.setObjectName(_fromUtf8("spinFigHeight"))
        self.horizontalLayout.addWidget(self.spinFigHeight)
        spacerItem = QtGui.QSpacerItem(10, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.horizontalLayout.addItem(spacerItem)
        self.verticalLayout.addLayout(self.horizontalLayout)
        self.verticalLayout_3.addWidget(self.groupBox_3)
        self.groupBox = QtGui.QGroupBox(PcaPlotDialog)
        self.groupBox.setObjectName(_fromUtf8("groupBox"))
        self.verticalLayout_2 = QtGui.QVBoxLayout(self.groupBox)
        self.verticalLayout_2.setObjectName(_fromUtf8("verticalLayout_2"))
        self.gridLayout = QtGui.QGridLayout()
        self.gridLayout.setObjectName(_fromUtf8("gridLayout"))
        self.checkBox = QtGui.QCheckBox(self.groupBox)
        self.checkBox.setEnabled(False)
        self.checkBox.setLayoutDirection(QtCore.Qt.RightToLeft)
        self.checkBox.setChecked(True)
        self.checkBox.setObjectName(_fromUtf8("checkBox"))
        self.gridLayout.addWidget(self.checkBox, 0, 0, 1, 1)
        self.chkPC3vsPC2 = QtGui.QCheckBox(self.groupBox)
        self.chkPC3vsPC2.setChecked(True)
        self.chkPC3vsPC2.setObjectName(_fromUtf8("chkPC3vsPC2"))
        self.gridLayout.addWidget(self.chkPC3vsPC2, 0, 1, 1, 1)
        self.chkPC1vsPC3 = QtGui.QCheckBox(self.groupBox)
        self.chkPC1vsPC3.setLayoutDirection(QtCore.Qt.RightToLeft)
        self.chkPC1vsPC3.setChecked(True)
        self.chkPC1vsPC3.setObjectName(_fromUtf8("chkPC1vsPC3"))
        self.gridLayout.addWidget(self.chkPC1vsPC3, 1, 0, 1, 1)
        self.verticalLayout_2.addLayout(self.gridLayout)
        self.verticalLayout_3.addWidget(self.groupBox)
        self.verticalLayout_4 = QtGui.QVBoxLayout()
        self.verticalLayout_4.setObjectName(_fromUtf8("verticalLayout_4"))
        self.horizontalLayout_5 = QtGui.QHBoxLayout()
        self.horizontalLayout_5.setObjectName(_fromUtf8("horizontalLayout_5"))
        self.label_5 = QtGui.QLabel(PcaPlotDialog)
        self.label_5.setObjectName(_fromUtf8("label_5"))
        self.horizontalLayout_5.addWidget(self.label_5)
        self.spinMarkerSize = QtGui.QSpinBox(PcaPlotDialog)
        self.spinMarkerSize.setMinimum(1)
        self.spinMarkerSize.setMaximum(100)
        self.spinMarkerSize.setProperty("value", 30)
        self.spinMarkerSize.setObjectName(_fromUtf8("spinMarkerSize"))
        self.horizontalLayout_5.addWidget(self.spinMarkerSize)
        spacerItem1 = QtGui.QSpacerItem(40, 20, QtGui.QSizePolicy.Expanding, QtGui.QSizePolicy.Minimum)
        self.horizontalLayout_5.addItem(spacerItem1)
        self.chkRotateLabels = QtGui.QCheckBox(PcaPlotDialog)
        self.chkRotateLabels.setChecked(True)
        self.chkRotateLabels.setObjectName(_fromUtf8("chkRotateLabels"))
        self.horizontalLayout_5.addWidget(self.chkRotateLabels)
        self.verticalLayout_4.addLayout(self.horizontalLayout_5)
        self.chkUniqueShapes = QtGui.QCheckBox(PcaPlotDialog)
        self.chkUniqueShapes.setChecked(True)
        self.chkUniqueShapes.setObjectName(_fromUtf8("chkUniqueShapes"))
        self.verticalLayout_4.addWidget(self.chkUniqueShapes)
        self.verticalLayout_3.addLayout(self.verticalLayout_4)
        self.horizontalLayout_3 = QtGui.QHBoxLayout()
        self.horizontalLayout_3.setObjectName(_fromUtf8("horizontalLayout_3"))
        self.buttonBox = QtGui.QDialogButtonBox(PcaPlotDialog)
        self.buttonBox.setOrientation(QtCore.Qt.Horizontal)
        self.buttonBox.setStandardButtons(QtGui.QDialogButtonBox.Cancel|QtGui.QDialogButtonBox.Ok)
        self.buttonBox.setCenterButtons(False)
        self.buttonBox.setObjectName(_fromUtf8("buttonBox"))
        self.horizontalLayout_3.addWidget(self.buttonBox)
        self.verticalLayout_3.addLayout(self.horizontalLayout_3)

        self.retranslateUi(PcaPlotDialog)
        QtCore.QObject.connect(self.buttonBox, QtCore.SIGNAL(_fromUtf8("accepted()")), PcaPlotDialog.accept)
        QtCore.QObject.connect(self.buttonBox, QtCore.SIGNAL(_fromUtf8("rejected()")), PcaPlotDialog.reject)
        QtCore.QObject.connect(self.chkFixedPixelsPerUnitDistance, QtCore.SIGNAL(_fromUtf8("clicked(bool)")), self.spinFigHeight.setDisabled)
        QtCore.QMetaObject.connectSlotsByName(PcaPlotDialog)

    def retranslateUi(self, PcaPlotDialog):
        PcaPlotDialog.setWindowTitle(QtGui.QApplication.translate("PcaPlotDialog", "PCA plot", None, QtGui.QApplication.UnicodeUTF8))
        self.groupBox_3.setTitle(QtGui.QApplication.translate("PcaPlotDialog", "Figure size", None, QtGui.QApplication.UnicodeUTF8))
        self.chkFixedPixelsPerUnitDistance.setText(QtGui.QApplication.translate("PcaPlotDialog", "Fixed pixels per unit distance (recommended)", None, QtGui.QApplication.UnicodeUTF8))
        self.lblFigureWidth.setText(QtGui.QApplication.translate("PcaPlotDialog", "Width:", None, QtGui.QApplication.UnicodeUTF8))
        self.lblFigureHeight.setText(QtGui.QApplication.translate("PcaPlotDialog", "Height:", None, QtGui.QApplication.UnicodeUTF8))
        self.groupBox.setTitle(QtGui.QApplication.translate("PcaPlotDialog", "Plots to show", None, QtGui.QApplication.UnicodeUTF8))
        self.checkBox.setText(QtGui.QApplication.translate("PcaPlotDialog", "PC1 vs. PC2", None, QtGui.QApplication.UnicodeUTF8))
        self.chkPC3vsPC2.setText(QtGui.QApplication.translate("PcaPlotDialog", "PC3 vs. PC2", None, QtGui.QApplication.UnicodeUTF8))
        self.chkPC1vsPC3.setText(QtGui.QApplication.translate("PcaPlotDialog", "PC1 vs. PC3", None, QtGui.QApplication.UnicodeUTF8))
        self.label_5.setText(QtGui.QApplication.translate("PcaPlotDialog", "Marker size:", None, QtGui.QApplication.UnicodeUTF8))
        self.chkRotateLabels.setText(QtGui.QApplication.translate("PcaPlotDialog", "Rotate PC3 labels", None, QtGui.QApplication.UnicodeUTF8))
        self.chkUniqueShapes.setText(QtGui.QApplication.translate("PcaPlotDialog", "Assign unique shapes", None, QtGui.QApplication.UnicodeUTF8))

