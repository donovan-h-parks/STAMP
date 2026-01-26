# =======================================================================
# Author: Donovan Parks
#
# Dialog box used to select features to ignore/process.
#
# Copyright 2011 Donovan Parks
#
# This file is part of STAMP.
#
# STAMP is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# STAMP is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.  If not, see <http://www.gnu.org/licenses/>.
# =======================================================================

from PyQt5 import QtGui, QtCore, QtWidgets
from stamp.GUI.selectFeaturesUI import Ui_SelectFeatureDlg


class SelectFeaturesDlg(QtWidgets.QDialog):
    def __init__(self, statsTestResults, parent=None):
        super(SelectFeaturesDlg, self).__init__(parent)

        # initialize GUI
        self.ui = Ui_SelectFeatureDlg()
        self.ui.setupUi(self)

        self.selectedFeatures = []
        self.inactiveFeatures = []

        self.init(statsTestResults)

        # setup signals
        self.ui.btnFilterFeature.clicked.connect(self.filterFeature)
        self.ui.btnAddFeature.clicked.connect(self.addFeature)
        self.ui.cboHierarchicalLevel.activated[str].connect(self.hierarchyChanged)

        self.centerWindow()

    def init(self, statsTestResults):
        if statsTestResults.profile is None:
            return

        self.statsTestResults = statsTestResults

        # initialize hierarchy
        self.ui.cboHierarchicalLevel.clear()
        for heading in self.statsTestResults.profile.hierarchyHeadings:
            self.ui.cboHierarchicalLevel.addItem(heading)
        self.ui.cboHierarchicalLevel.setCurrentIndex(len(self.statsTestResults.profile.hierarchyHeadings) - 1)

        # initialize active and filtered features
        allFeatures = set(statsTestResults.getColumn('Features', False))
        self.selectedFeatures = set(statsTestResults.getSelectedFeatures())
        self.inactiveFeatures = list(allFeatures - self.selectedFeatures)
        self.selectedFeatures = list(self.selectedFeatures)

        for feature in self.selectedFeatures:
            self.ui.lstSelectedFeatures.addItem(feature)

        for feature in self.inactiveFeatures:
            self.ui.lstFilteredFeatures.addItem(feature)

        self.ui.txtNumSelectedFeatures.setText(str(len(self.selectedFeatures)))

    def getSelectedFeatures(self):
        return self.selectedFeatures

    def hierarchyChanged(self):
        index = self.ui.cboHierarchicalLevel.currentIndex()

        # modify active list
        feature_counts = {}
        for feature in self.selectedFeatures:
            headings = self.statsTestResults.profile.getHierarchy(feature)
            selectedFeature = headings[index]
            feature_counts[selectedFeature] = feature_counts.get(selectedFeature, 0) + 1

        self.ui.lstSelectedFeatures.clear()
        for feature in feature_counts:
            if index == len(self.statsTestResults.profile.hierarchyHeadings) - 1:
                self.ui.lstSelectedFeatures.addItem(feature)
            else:
                self.ui.lstSelectedFeatures.addItem(feature + ' [' + str(feature_counts[feature]) + ']')

        # modify filtered list
        feature_counts = {}
        for feature in self.inactiveFeatures:
            headings = self.statsTestResults.profile.getHierarchy(feature)
            selectedFeature = headings[index]
            feature_counts[selectedFeature] = feature_counts.get(selectedFeature, 0) + 1

        self.ui.lstFilteredFeatures.clear()
        for feature in feature_counts:
            if index == len(self.statsTestResults.profile.hierarchyHeadings) - 1:
                self.ui.lstFilteredFeatures.addItem(feature)
            else:
                self.ui.lstFilteredFeatures.addItem(feature + ' [' + str(feature_counts[feature]) + ']')

        self.ui.txtNumSelectedFeatures.setText(str(len(self.selectedFeatures)))

    def filterFeature(self):
        items = self.ui.lstSelectedFeatures.selectedItems()
        for item in items:
            itemText = str(item.text())
            pos = itemText.find(' [')
            if pos != -1:
                itemText = itemText[0:pos]

            for feature in reversed(self.selectedFeatures):
                headings = self.statsTestResults.profile.getHierarchy(feature)
                if itemText in headings:
                    self.selectedFeatures.remove(feature)
                    self.inactiveFeatures.append(feature)

        self.hierarchyChanged()

    def addFeature(self):
        items = self.ui.lstFilteredFeatures.selectedItems()
        for item in items:
            itemText = str(item.text())
            pos = itemText.find(' [')
            if pos != -1:
                itemText = itemText[0:pos]

            for feature in reversed(self.inactiveFeatures):
                headings = self.statsTestResults.profile.getHierarchy(feature)
                if itemText in headings:
                    self.selectedFeatures.append(feature)
                    self.inactiveFeatures.remove(feature)

        self.hierarchyChanged()

    def centerWindow(self):
        screen = QtWidgets.QDesktopWidget().screenGeometry()
        size = self.geometry()
        self.move((screen.width() - size.width()) // 2, (screen.height() - size.height()) // 2)