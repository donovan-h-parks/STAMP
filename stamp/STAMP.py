#=======================================================================
# Author: Donovan Parks
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
#=======================================================================

__author__ = 'Donovan Parks'
__copyright__ = 'Copyright 2013'
__credits__ = ['Donovan Parks']
__license__ = 'GPL3'
__version__ = '2.1.2'
__date__ = 'June 15, 2015'
__maintainer__ = 'Donovan Parks'
__email__ = 'donovan.parks@gmail.com'
__status__ = 'Development'

import os
import sys
import platform
import string

import stamp.Dependencies
from stamp.GUI.plotDlg import PlotDlg  # forward reference so py2app recognizes this file is required

from PyQt4 import QtGui, QtCore

from mainUI import Ui_MainWindow
from stamp.GUI.selectFeaturesDlg import SelectFeaturesDlg
from stamp.GUI.createProfileMgRastDlg import CreateProfileMgRastDlg
from stamp.GUI.createProfileRITADlg import CreateProfileRITADlg
from stamp.GUI.createProfileCoMetDlg import CreateProfileCoMetDlg
from stamp.GUI.createProfileMothurDlg import CreateProfileMothurDlg
from stamp.GUI.createProfileBiomDlg import CreateProfileBiomDlg
from stamp.GUI.loadDataDlg import LoadDataDlg
from stamp.GUI.assignCOGsDlg import AssignCOGsDlg
from stamp.GUI.preferencesDlg import PreferencesDlg
from stamp.GUI.multCompCorrectionInfoDlg import MultCompCorrectionInfoDlg
from stamp.GUI.groupLegendDlg import GroupLegendDlg
from stamp.GUI.statsTableDlg import StatsTableDlg
from stamp.GUI.metadataTableDlg import MetadataTableDlg

from stamp.metagenomics.stats.SampleStatsTests import SampleStatsTests
from stamp.metagenomics.stats.GroupStatsTests import GroupStatsTests
from stamp.metagenomics.stats.MultiGroupStatsTests import MultiGroupStatsTests
from stamp.metagenomics.fileIO.StampIO import StampIO
from stamp.metagenomics.fileIO.MetadataIO import MetadataIO
from stamp.metagenomics.GenericTable import GenericTable
from stamp.metagenomics.ProfileTree import ProfileTree
from stamp.metagenomics.SampleProfile import SampleProfile
from stamp.metagenomics.GroupProfile import GroupProfile
from stamp.metagenomics.MultiGroupProfile import MultiGroupProfile

from stamp.metagenomics.DirectoryHelper import getMainDir

from stamp.plugins.PlotsManager import PlotsManager
from stamp.plugins.PluginManager import PluginManager

import matplotlib as mpl

from numpy import seterr

class MainWindow(QtGui.QMainWindow):
	def __init__(self, preferences, parent=None):
		QtGui.QWidget.__init__(self, parent)

		# setup default plot settings
		mpl.rcParams['font.size'] = 8
		mpl.rcParams['axes.titlesize'] = 8
		mpl.rcParams['axes.labelsize'] = 8
		mpl.rcParams['xtick.labelsize'] = 8
		mpl.rcParams['ytick.labelsize'] = 8
		mpl.rcParams['legend.fontsize'] = 8

		# setup preferences and settings
		self.preferences = preferences
		self.settings = QtCore.QSettings("BeikoLab", "STAMP")
		self.preferences['Settings'] = self.settings

		# icons
		self.refreshIcon = QtGui.QIcon()
		self.refreshIcon.addPixmap(QtGui.QPixmap(":/icons/icons/refresh.png"), QtGui.QIcon.Normal, QtGui.QIcon.Off)

		# initialize GUI
		self.ui = Ui_MainWindow()
		self.ui.setupUi(self)

		# setup status bar
		self.lblStatusBar = QtGui.QLabel()
		self.ui.statusBar.addPermanentWidget(self.lblStatusBar)

		self.btnAutoRecalculation = QtGui.QPushButton('Recalculate statistics and plots')
		self.btnAutoRecalculation.setCheckable(True)
		self.btnAutoRecalculation.setChecked(True)
		self.btnAutoRecalculation.setFixedHeight(20)
		self.ui.statusBar.addPermanentWidget(self.btnAutoRecalculation)
		self.connect(self.btnAutoRecalculation, QtCore.SIGNAL('toggled(bool)'), self.autoRecalculateChanged)
		self.bAutoRecalculate = True

		# initialize class variables
		self.profileTree = ProfileTree()
		self.sampleProfile = SampleProfile()
		self.groupProfile = GroupProfile()
		self.multiGroupProfile = MultiGroupProfile()

		# setup view STAMP properties menu item
		self.ui.dockProperties.toggleViewAction().setShortcut("Ctrl+P")
		self.ui.dockProperties.toggleViewAction().setToolTip("Show\hide properties window")
		self.ui.dockProperties.toggleViewAction().setStatusTip("Show\hide properties window")
		self.ui.menuView.addAction(self.ui.dockProperties.toggleViewAction())

		# setup group legend
		self.ui.menuView.addSeparator()
		self.groupLegendDlg = GroupLegendDlg(self.preferences, self)
		self.groupLegendDlg.setObjectName("groupLegendDlg");
		self.groupLegendDlg.setVisible(True)
		self.addDockWidget(QtCore.Qt.RightDockWidgetArea, self.groupLegendDlg)
		self.groupLegendDlg.setFloating(False)

		self.groupLegendDlg.toggleViewAction().setShortcut("Ctrl+L")
		self.groupLegendDlg.toggleViewAction().setToolTip("Show\hide group legend window")
		self.groupLegendDlg.toggleViewAction().setStatusTip("Show\hide group legend window")
		self.ui.menuView.addAction(self.groupLegendDlg.toggleViewAction())

		self.connect(self.groupLegendDlg, QtCore.SIGNAL('legendItemChanged()'), self.legendItemChanged)
		self.connect(self.groupLegendDlg, QtCore.SIGNAL('legendFieldChanged()'), self.legendFieldChanged)
		self.connect(self.groupLegendDlg, QtCore.SIGNAL('legendActiveGroupsChanged()'), self.legendActiveGroupsChanged)

		# setup metadata window
		self.ui.menuView.addSeparator()
		self.metadataDlg = MetadataTableDlg(self)
		self.metadataDlg.setObjectName("metadataDlg");
		self.metadataDlg.setVisible(True)
		self.addDockWidget(QtCore.Qt.BottomDockWidgetArea, self.metadataDlg)
		self.metadataDlg.setFloating(False)

		self.metadataDlg.toggleViewAction().setShortcut("Ctrl+M")
		self.metadataDlg.toggleViewAction().setToolTip("Show\hide metadata table")
		self.metadataDlg.toggleViewAction().setStatusTip("Show\hide metadata table")
		self.ui.menuView.addAction(self.metadataDlg.toggleViewAction())

		self.connect(self.metadataDlg, QtCore.SIGNAL('activeSamplesChanged()'), self.activeSamplesChanged)

		# connect menu items signals to slots
		self.connect(self.ui.mnuFileOpenProfile, QtCore.SIGNAL('triggered()'), self.loadProfile)
		self.connect(self.ui.mnuFileMgRast, QtCore.SIGNAL('triggered()'), self.createProfileMgRast)
		self.connect(self.ui.mnuFileRITA, QtCore.SIGNAL('triggered()'), self.createProfileRita)
		self.connect(self.ui.mnuFileCoMet, QtCore.SIGNAL('triggered()'), self.createProfileComet)
		self.connect(self.ui.mnuFileMothur, QtCore.SIGNAL('triggered()'), self.createProfileMothur)
		self.connect(self.ui.mnuFileBIOM, QtCore.SIGNAL('triggered()'), self.createProfileBIOM)
		self.connect(self.ui.mnuFileAppendCategoryCOG, QtCore.SIGNAL('triggered()'), self.appendCategoriesCOG)
		self.connect(self.ui.mnuFileSavePlot, QtCore.SIGNAL('triggered()'), self.saveImageDlg)
		self.connect(self.ui.mnuFileExit, QtCore.SIGNAL('triggered()'), QtCore.SLOT('close()'))

		self.connect(self.ui.mnuViewSendPlotToWindow, QtCore.SIGNAL('triggered()'), self.sendPlotToWindow)
		self.connect(self.ui.mnuSettingsPreferences, QtCore.SIGNAL('triggered()'), self.prefrencesDlg)
		self.connect(self.ui.mnuHelpAbout, QtCore.SIGNAL('triggered()'), self.openAboutDlg)

		# connect profile level combo box signals to slots
		self.connect(self.ui.cboProfileLevel, QtCore.SIGNAL('activated(QString)'), self.profileLevelChanged)
		self.connect(self.ui.cboParentalLevel, QtCore.SIGNAL('activated(QString)'), self.parentLevelChanged)
		self.connect(self.ui.cboUnclassified, QtCore.SIGNAL('activated(QString)'), self.unclassifiedTreatmentChanged)

		self.setupSampleWidgets()
		self.setupGroupWidgets()
		self.setupMultiGroupWidgets()

		# load multiple test correction methods
		pluginManager = PluginManager(self.preferences)
		self.multCompDict = pluginManager.loadPlugins('stamp/plugins/common/multipleComparisonCorrections/')
		pluginManager.populateComboBox(self.multCompDict, self.ui.cboSampleMultCompMethod, 'No correction')
		pluginManager.populateComboBox(self.multCompDict, self.ui.cboGroupMultCompMethod, 'No correction')
		pluginManager.populateComboBox(self.multCompDict, self.ui.cboMultiGroupMultCompMethod, 'No correction')

		# connect tab widget signals to slots
		self.connect(self.ui.tabWidgetProperties, QtCore.SIGNAL('currentChanged(int)'), self.propertiesTabChanged)

		# restore previous window states (size and location of main window and all dock widgets)
		windowSettings = QtCore.QSettings("BeikoLab", "STAMP");
		self.restoreState(windowSettings.value("MainWindow/State").toByteArray())
		bRestoredState = self.restoreGeometry(windowSettings.value("MainWindow/Geometry").toByteArray())

		if not bRestoredState:
			self.resize(800, 600)
			self.showMaximized()

		self.metadata = None

		# self.loadProfile() # *** For debugging purposes

	def propertiesTabChanged(self, currentIndex):
		self.ui.stackedWidgetViews.setCurrentIndex(currentIndex)
		self.updateStatusBar()

	def setupSampleWidgets(self):
		self.sampleStatsTest = SampleStatsTests(self.preferences)

		# initialize statistical summary tables
		self.ui.menuView.addSeparator()
		self.sampleTable = StatsTableDlg(self.preferences, self)
		self.sampleTable.setWindowTitle('Two sample statistics table')
		self.sampleTable.setVisible(False)
		self.addDockWidget(QtCore.Qt.BottomDockWidgetArea, self.sampleTable)
		self.sampleTable.setFloating(True)

		self.sampleTable.toggleViewAction().setShortcut("Ctrl+T")
		self.sampleTable.toggleViewAction().setToolTip("Show\hide two sample statistical table")
		self.sampleTable.toggleViewAction().setStatusTip("Show\hide two sample statistical table")
		self.ui.menuView.addAction(self.sampleTable.toggleViewAction())

		# load plot plugins
		self.samplePlot = PlotsManager(self.ui.cboSamplePlots, self.ui.plotSampleScrollArea, 'Scatter plot')
		self.samplePlot.loadPlots(self.preferences, 'stamp/plugins/samples/plots/')

		# load statistical technique plugins
		pluginManager = PluginManager(self.preferences)
		self.sampleStatTestDict = pluginManager.loadPlugins('stamp/plugins/samples/statisticalTests/')
		pluginManager.populateComboBox(self.sampleStatTestDict, self.ui.cboSampleStatTests, 'G-test (w/ Yates\') + Fisher\'s')

		self.sampleConfIntervMethodDict = pluginManager.loadPlugins('stamp/plugins/samples/confidenceIntervalMethods/')
		pluginManager.populateComboBox(self.sampleConfIntervMethodDict, self.ui.cboSampleConfIntervMethods, 'DP: Asymptotic-CC')

		# load effect size filters
		self.sampleEffectSizeDict = pluginManager.loadPlugins('stamp/plugins/samples/effectSizeFilters/')
		pluginManager.populateComboBox(self.sampleEffectSizeDict, self.ui.cboSampleEffectSizeMeasure1, 'Difference between proportions')
		pluginManager.populateComboBox(self.sampleEffectSizeDict, self.ui.cboSampleEffectSizeMeasure2, 'Ratio of proportions')

		# widget controls in sidebar
		self.connect(self.ui.btnSampleProfileTab, QtCore.SIGNAL('clicked()'), self.sampleProfileTabClicked)
		self.connect(self.ui.btnSampleProfileArrow, QtCore.SIGNAL('clicked()'), self.sampleProfileTabClicked)
		self.connect(self.ui.btnSampleStatisticsTab, QtCore.SIGNAL('clicked()'), self.samplePropTabClicked)
		self.connect(self.ui.btnSampleStatisticsArrow, QtCore.SIGNAL('clicked()'), self.samplePropTabClicked)
		self.connect(self.ui.btnSampleFilteringTab, QtCore.SIGNAL('clicked()'), self.sampleFilteringTabClicked)
		self.connect(self.ui.btnSampleFilteringArrow, QtCore.SIGNAL('clicked()'), self.sampleFilteringTabClicked)

		# connect profile widget signals to slots
		self.connect(self.ui.cboSample1, QtCore.SIGNAL('activated(QString)'), self.sampleHierarchicalLevelsChanged)
		self.connect(self.ui.cboSample2, QtCore.SIGNAL('activated(QString)'), self.sampleHierarchicalLevelsChanged)
		self.connect(self.ui.btnSample1Colour, QtCore.SIGNAL('clicked()'), self.sample1ColourDlg)
		self.connect(self.ui.btnSample2Colour, QtCore.SIGNAL('clicked()'), self.sample2ColourDlg)

		# connect statistical test widget signals to slots
		self.connect(self.ui.cboSampleStatTests, QtCore.SIGNAL('activated(QString)'), self.sampleRunTest)
		self.connect(self.ui.cboSampleSignTestType, QtCore.SIGNAL('activated(QString)'), self.sampleRunTest)
		self.connect(self.ui.cboSampleConfIntervMethods, QtCore.SIGNAL('activated(QString)'), self.sampleRunTest)
		self.connect(self.ui.cboSampleNominalCoverage, QtCore.SIGNAL('activated(QString)'), self.sampleRunTest)
		self.connect(self.ui.cboSampleMultCompMethod, QtCore.SIGNAL('activated(QString)'), self.sampleMultCompCorrectionChanged)
		self.connect(self.ui.btnSampleMultCompCorrectionInfo, QtCore.SIGNAL('clicked()'), self.sampleMultCompCorrectionInfo)

		# connect filtering test widget signals to slots
		self.connect(self.ui.chkSampleSelectFeatures, QtCore.SIGNAL('toggled(bool)'), self.sampleSelectFeaturesCheckbox)
		self.connect(self.ui.btnSampleSelectFeatures, QtCore.SIGNAL('clicked()'), self.sampleSelectFeaturesDlg)

		self.connect(self.ui.chkSampleEnableSignLevelFilter, QtCore.SIGNAL('toggled(bool)'), self.sampleFilteringPropChanged)
		self.connect(self.ui.spinSampleSignLevelFilter, QtCore.SIGNAL('editingFinished()'), self.sampleFilteringPropChanged)

		self.connect(self.ui.cboSampleSeqFilter, QtCore.SIGNAL('activated(QString)'), self.sampleSeqFilterChanged)
		self.connect(self.ui.chkSampleEnableSeqFilter, QtCore.SIGNAL('toggled(bool)'), self.sampleFilteringPropChanged)
		self.connect(self.ui.spinSampleFilterSample1, QtCore.SIGNAL('editingFinished()'), self.sampleFilteringPropChanged)
		self.connect(self.ui.spinSampleFilterSample2, QtCore.SIGNAL('editingFinished()'), self.sampleFilteringPropChanged)

		self.connect(self.ui.cboSampleParentSeqFilter, QtCore.SIGNAL('activated(QString)'), self.sampleParentSeqFilterChanged)
		self.connect(self.ui.chkSampleEnableParentSeqFilter, QtCore.SIGNAL('toggled(bool)'), self.sampleFilteringPropChanged)
		self.connect(self.ui.spinSampleParentFilterSample1, QtCore.SIGNAL('editingFinished()'), self.sampleFilteringPropChanged)
		self.connect(self.ui.spinSampleParentFilterSample2, QtCore.SIGNAL('editingFinished()'), self.sampleFilteringPropChanged)

		self.connect(self.ui.radioSampleOR, QtCore.SIGNAL('clicked()'), self.sampleFilteringPropChanged)
		self.connect(self.ui.radioSampleAND, QtCore.SIGNAL('clicked()'), self.sampleFilteringPropChanged)

		self.connect(self.ui.cboSampleEffectSizeMeasure1, QtCore.SIGNAL('activated(QString)'), self.sampleChangeEffectSizeMeasure)
		self.connect(self.ui.cboSampleEffectSizeMeasure2, QtCore.SIGNAL('activated(QString)'), self.sampleChangeEffectSizeMeasure)
		self.connect(self.ui.spinSampleMinEffectSize1, QtCore.SIGNAL('editingFinished()'), self.sampleFilteringPropChanged)
		self.connect(self.ui.spinSampleMinEffectSize2, QtCore.SIGNAL('editingFinished()'), self.sampleFilteringPropChanged)
		self.connect(self.ui.chkSampleEnableEffectSizeFilter1, QtCore.SIGNAL('toggled(bool)'), self.sampleFilteringPropChanged)
		self.connect(self.ui.chkSampleEnableEffectSizeFilter2, QtCore.SIGNAL('toggled(bool)'), self.sampleFilteringPropChanged)

		# connect statistical plot page widget signals to slots
		self.connect(self.ui.cboSamplePlots, QtCore.SIGNAL('activated(QString)'), self.samplePlotUpdate)
		self.connect(self.ui.btnSampleConfigurePlot, QtCore.SIGNAL('clicked()'), self.samplePlotConfigure)
		self.connect(self.ui.cboSampleHighlightHierarchy, QtCore.SIGNAL('activated(QString)'), self.sampleHighlightHierarchyChanged)
		self.connect(self.ui.cboSampleHighlightFeature, QtCore.SIGNAL('activated(QString)'), self.sampleHighlightFeatureChanged)

		# initialize dynamic GUI elements
		self.setSample1Colour(self.preferences['Sample 1 colour'])
		self.setSample2Colour(self.preferences['Sample 2 colour'])

	def setupGroupWidgets(self):
		self.groupStatsTest = GroupStatsTests(self.preferences)

		# initialize statistical summary tables
		self.groupTable = StatsTableDlg(self.preferences, self)
		self.groupTable.setWindowTitle('Two group statistics table')
		self.groupTable.setVisible(False)
		self.addDockWidget(QtCore.Qt.BottomDockWidgetArea, self.groupTable)
		self.groupTable.setFloating(True)

		self.groupTable.toggleViewAction().setShortcut("Ctrl+G")
		self.groupTable.toggleViewAction().setToolTip("Show\hide two group statistical table")
		self.groupTable.toggleViewAction().setStatusTip("Show\hide two group statistical table")
		self.ui.menuView.addAction(self.groupTable.toggleViewAction())

		# load plot plugins
		self.groupPlot = PlotsManager(self.ui.cboGroupPlots, self.ui.plotGroupScrollArea, 'PCA plot')
		self.groupPlot.loadPlots(self.preferences, 'stamp/plugins/groups/plots/')

		# load statistical technique plugins
		pluginManager = PluginManager(self.preferences)
		self.groupStatTestDict = pluginManager.loadPlugins('stamp/plugins/groups/statisticalTests/')
		pluginManager.populateComboBox(self.groupStatTestDict, self.ui.cboGroupStatTests, "Welch's t-test")

		# load effect size filters
		self.groupEffectSizeDict = pluginManager.loadPlugins('stamp/plugins/groups/effectSizeFilters/')
		pluginManager.populateComboBox(self.groupEffectSizeDict, self.ui.cboGroupEffectSizeMeasure1, 'Difference between proportions')
		pluginManager.populateComboBox(self.groupEffectSizeDict, self.ui.cboGroupEffectSizeMeasure2, 'Ratio of proportions')

		# widget controls in sidebar
		self.connect(self.ui.btnGroupProfileTab, QtCore.SIGNAL('clicked()'), self.groupProfileTabClicked)
		self.connect(self.ui.btnGroupProfileArrow, QtCore.SIGNAL('clicked()'), self.groupProfileTabClicked)
		self.connect(self.ui.btnGroupStatisticsTab, QtCore.SIGNAL('clicked()'), self.groupPropTabClicked)
		self.connect(self.ui.btnGroupStatisticsArrow, QtCore.SIGNAL('clicked()'), self.groupPropTabClicked)
		self.connect(self.ui.btnGroupFilteringTab, QtCore.SIGNAL('clicked()'), self.groupFilteringTabClicked)
		self.connect(self.ui.btnGroupFilteringArrow, QtCore.SIGNAL('clicked()'), self.groupFilteringTabClicked)

		# connect profile widget signals to slots
		self.connect(self.ui.cboGroup1, QtCore.SIGNAL('activated(QString)'), self.groupHierarchicalLevelsChanged)
		self.connect(self.ui.cboGroup2, QtCore.SIGNAL('activated(QString)'), self.groupHierarchicalLevelsChanged)
		self.connect(self.ui.btnGroup1Colour, QtCore.SIGNAL('clicked()'), self.group1ColourDlg)
		self.connect(self.ui.btnGroup2Colour, QtCore.SIGNAL('clicked()'), self.group2ColourDlg)

		# connect statistical test widget signals to slots
		self.connect(self.ui.cboGroupStatTests, QtCore.SIGNAL('activated(QString)'), self.groupRunTest)
		self.connect(self.ui.cboGroupSignTestType, QtCore.SIGNAL('activated(QString)'), self.groupRunTest)
		self.connect(self.ui.cboGroupConfIntervMethods, QtCore.SIGNAL('activated(QString)'), self.groupRunTest)
		self.connect(self.ui.cboGroupNominalCoverage, QtCore.SIGNAL('activated(QString)'), self.groupRunTest)
		self.connect(self.ui.cboGroupMultCompMethod, QtCore.SIGNAL('activated(QString)'), self.groupMultCompCorrectionChanged)
		self.connect(self.ui.btnGroupMultCompCorrectionInfo, QtCore.SIGNAL('clicked()'), self.groupMultCompCorrectionInfo)

		# connect filtering test widget signals to slots
		self.connect(self.ui.chkGroupSelectFeatures, QtCore.SIGNAL('toggled(bool)'), self.groupSelectFeaturesCheckbox)
		self.connect(self.ui.btnGroupSelectFeatures, QtCore.SIGNAL('clicked()'), self.groupSelectFeaturesDlg)

		self.connect(self.ui.chkGroupEnableSignLevelFilter, QtCore.SIGNAL('toggled(bool)'), self.groupFilteringPropChanged)
		self.connect(self.ui.spinGroupSignLevelFilter, QtCore.SIGNAL('editingFinished()'), self.groupFilteringPropChanged)

		self.connect(self.ui.cboGroupSeqFilter, QtCore.SIGNAL('activated(QString)'), self.groupSeqFilterChanged)
		self.connect(self.ui.chkGroupEnableSeqFilter, QtCore.SIGNAL('toggled(bool)'), self.groupFilteringPropChanged)
		self.connect(self.ui.spinGroupFilter1, QtCore.SIGNAL('editingFinished()'), self.groupFilteringPropChanged)
		self.connect(self.ui.spinGroupFilter2, QtCore.SIGNAL('editingFinished()'), self.groupFilteringPropChanged)

		self.connect(self.ui.cboGroupParentSeqFilter, QtCore.SIGNAL('activated(QString)'), self.groupParentSeqFilterChanged)
		self.connect(self.ui.chkGroupEnableParentSeqFilter, QtCore.SIGNAL('toggled(bool)'), self.groupFilteringPropChanged)
		self.connect(self.ui.spinGroupParentFilter1, QtCore.SIGNAL('editingFinished()'), self.groupFilteringPropChanged)
		self.connect(self.ui.spinGroupParentFilter2, QtCore.SIGNAL('editingFinished()'), self.groupFilteringPropChanged)

		self.connect(self.ui.radioGroupOR, QtCore.SIGNAL('clicked()'), self.groupFilteringPropChanged)
		self.connect(self.ui.radioGroupAND, QtCore.SIGNAL('clicked()'), self.groupFilteringPropChanged)

		self.connect(self.ui.cboGroupEffectSizeMeasure1, QtCore.SIGNAL('activated(QString)'), self.groupChangeEffectSizeMeasure)
		self.connect(self.ui.cboGroupEffectSizeMeasure2, QtCore.SIGNAL('activated(QString)'), self.groupChangeEffectSizeMeasure)
		self.connect(self.ui.spinGroupMinEffectSize1, QtCore.SIGNAL('editingFinished()'), self.groupFilteringPropChanged)
		self.connect(self.ui.spinGroupMinEffectSize2, QtCore.SIGNAL('editingFinished()'), self.groupFilteringPropChanged)
		self.connect(self.ui.chkGroupEnableEffectSizeFilter1, QtCore.SIGNAL('toggled(bool)'), self.groupFilteringPropChanged)
		self.connect(self.ui.chkGroupEnableEffectSizeFilter2, QtCore.SIGNAL('toggled(bool)'), self.groupFilteringPropChanged)

		self.connect(self.ui.chkShowActiveFeaturesGroupTable, QtCore.SIGNAL('clicked()'), self.groupFeaturesTableUpdate)

		# connect statistical plot page widget signals to slots
		self.connect(self.ui.cboGroupPlots, QtCore.SIGNAL('activated(QString)'), self.groupPlotUpdate)
		self.connect(self.ui.btnGroupConfigurePlot, QtCore.SIGNAL('clicked()'), self.groupPlotConfigure)
		self.connect(self.ui.cboGroupHighlightHierarchy, QtCore.SIGNAL('activated(QString)'), self.groupHighlightHierarchyChanged)
		self.connect(self.ui.cboGroupHighlightFeature, QtCore.SIGNAL('activated(QString)'), self.groupHighlightFeatureChanged)

		# initialize dynamic GUI elements
		self.setGroup1Colour(self.groupLegendDlg.groupColours[0], False)
		self.setGroup2Colour(self.groupLegendDlg.groupColours[1], False)

		self.groupTestConfIntervMethods()

	def setupMultiGroupWidgets(self):
		self.multiGroupStatsTest = MultiGroupStatsTests(self.preferences)

		# initialize statistical summary tables
		self.multiGroupTable = StatsTableDlg(self.preferences, self)
		self.multiGroupTable.setWindowTitle('Multiple group statistics table')
		self.multiGroupTable.setVisible(False)
		self.addDockWidget(QtCore.Qt.BottomDockWidgetArea, self.multiGroupTable)
		self.multiGroupTable.setFloating(True)

		self.multiGroupTable.toggleViewAction().setShortcut("Ctrl+M")
		self.multiGroupTable.toggleViewAction().setToolTip("Show\hide multiple group statistical table")
		self.multiGroupTable.toggleViewAction().setStatusTip("Show\hide multiple group statistical table")
		self.ui.menuView.addAction(self.multiGroupTable.toggleViewAction())

		# load plot plugins
		self.multiGroupPlot = PlotsManager(self.ui.cboMultiGroupPlots, self.ui.plotMultiGroupScrollArea, 'PCA plot')
		self.multiGroupPlot.loadPlots(self.preferences, 'stamp/plugins/multiGroups/plots/')

		# load statistical technique plugins
		pluginManager = PluginManager(self.preferences)
		self.multiGroupStatTestDict = pluginManager.loadPlugins('stamp/plugins/multiGroups/statisticalTests/')
		pluginManager.populateComboBox(self.multiGroupStatTestDict, self.ui.cboMultiGroupStatTests, 'ANOVA')

		self.postHocTestDict = pluginManager.loadPlugins('stamp/plugins/multiGroups/postHoc/')
		pluginManager.populateComboBox(self.postHocTestDict, self.ui.cboPostHocTest, 'Tukey-Kramer')

		# load effect size filters
		self.multiGroupEffectSizeDict = pluginManager.loadPlugins('stamp/plugins/multiGroups/effectSizeFilters/')
		pluginManager.populateComboBox(self.multiGroupEffectSizeDict, self.ui.cboMultiGroupEffectSizeMeasure, 'Eta-squared')

		# widget controls in sidebar
		self.connect(self.ui.btnMultiGroupStatisticsTab, QtCore.SIGNAL('clicked()'), self.multiGroupPropTabClicked)
		self.connect(self.ui.btnMultiGroupStatisticsArrow, QtCore.SIGNAL('clicked()'), self.multiGroupPropTabClicked)
		self.connect(self.ui.btnMultiGroupFilteringTab, QtCore.SIGNAL('clicked()'), self.multiGroupFilteringTabClicked)
		self.connect(self.ui.btnMultiGroupFilteringArrow, QtCore.SIGNAL('clicked()'), self.multiGroupFilteringTabClicked)

		# connect statistical test widget signals to slots
		self.connect(self.ui.cboMultiGroupStatTests, QtCore.SIGNAL('activated(QString)'), self.multiGroupRunTest)
		self.connect(self.ui.cboMultiGroupMultCompMethod, QtCore.SIGNAL('activated(QString)'), self.multiGroupMultCompCorrectionChanged)
		self.connect(self.ui.btnMultiGroupMultCompCorrectionInfo, QtCore.SIGNAL('clicked()'), self.multiGroupMultCompCorrectionInfo)
		self.connect(self.ui.cboPostHocTest, QtCore.SIGNAL('activated(QString)'), self.multiGroupPlotUpdate)
		self.connect(self.ui.cboMultiGroupNominalCoverage, QtCore.SIGNAL('activated(QString)'), self.multiGroupPlotUpdate)

		# connect filtering test widget signals to slots
		self.connect(self.ui.chkMultiGroupSelectFeatures, QtCore.SIGNAL('toggled(bool)'), self.multiGroupSelectFeaturesCheckbox)
		self.connect(self.ui.btnMultiGroupSelectFeatures, QtCore.SIGNAL('clicked()'), self.multiGroupSelectFeaturesDlg)

		self.connect(self.ui.chkMultiGroupEnableSignLevelFilter, QtCore.SIGNAL('toggled(bool)'), self.multiGroupFilteringPropChanged)
		self.connect(self.ui.spinMultiGroupSignLevelFilter, QtCore.SIGNAL('editingFinished()'), self.multiGroupFilteringPropChanged)

		self.connect(self.ui.spinMultiGroupMinEffectSize, QtCore.SIGNAL('editingFinished()'), self.multiGroupFilteringPropChanged)
		self.connect(self.ui.chkMultiGroupEnableEffectSizeFilter, QtCore.SIGNAL('toggled(bool)'), self.multiGroupFilteringPropChanged)

		self.connect(self.ui.chkShowActiveFeaturesMultiGroupTable, QtCore.SIGNAL('clicked()'), self.multiGroupFeaturesTableUpdate)

		# connect statistical plot page widget signals to slots
		self.connect(self.ui.cboMultiGroupPlots, QtCore.SIGNAL('activated(QString)'), self.multiGroupPlotUpdate)
		self.connect(self.ui.btnMultiGroupConfigurePlot, QtCore.SIGNAL('clicked()'), self.multiGroupPlotConfigure)
		self.connect(self.ui.cboMultiGroupHighlightHierarchy, QtCore.SIGNAL('activated(QString)'), self.multiGroupHighlightHierarchyChanged)
		self.connect(self.ui.cboMultiGroupHighlightFeature, QtCore.SIGNAL('activated(QString)'), self.multiGroupHighlightFeatureChanged)

	def autoRecalculateChanged(self, checked):
		self.bAutoRecalculate = checked

		if self.bAutoRecalculate == True:
			self.sampleRunTest()
			self.groupRunTest()
			self.multiGroupRunTest()

	def activeSamplesChanged(self):
		self.populateSampleComboBoxes()
		self.sampleRunTest()

		self.groupLegendDlg.initLegend(self.profileTree, self.metadata, self.metadata.activeField)
		self.groupRunTest()
		self.multiGroupRunTest()

	def populateSampleComboBoxes(self):
		# cache currently selected samples
		sampleName1 = str(self.ui.cboSample1.currentText())
		sampleName2 = str(self.ui.cboSample2.currentText())

		self.ui.cboSample1.clear()
		self.ui.cboSample2.clear()
		for name in sorted(self.profileTree.sampleNames):
			if self.metadata == None or name in self.metadata.activeSamples:
				self.ui.cboSample1.addItem(name)
				self.ui.cboSample2.addItem(name)

		if self.ui.cboSample1.findText(sampleName1) != -1:
			self.ui.cboSample1.setCurrentIndex(self.ui.cboSample1.findText(sampleName1))
		else:
			self.ui.cboSample1.setCurrentIndex(0)

		if self.ui.cboSample2.findText(sampleName2) != -1:
			self.ui.cboSample2.setCurrentIndex(self.ui.cboSample2.findText(sampleName2))
		else:
			self.ui.cboSample2.setCurrentIndex(1)

	def legendActiveGroupsChanged(self):
		group1 = self.ui.cboGroup1.currentText()
		group2 = self.ui.cboGroup2.currentText()

		self.ui.cboGroup1.clear()
		self.ui.cboGroup2.clear()
		for name, bActive in sorted(self.profileTree.groupActive.items()):
			if bActive:
				self.ui.cboGroup1.addItem(name)
				self.ui.cboGroup2.addItem(name)
		self.ui.cboGroup2.addItem('<All other samples>')

		index = self.ui.cboGroup1.findText(group1)
		if index != -1:
			self.ui.cboGroup1.setCurrentIndex(index)
		else:
			self.ui.cboGroup1.setCurrentIndex(0)

		index = self.ui.cboGroup2.findText(group2)
		if index != -1:
			self.ui.cboGroup2.setCurrentIndex(index)
		else:
			if self.ui.cboGroup2.count() >= 2:
				self.ui.cboGroup2.setCurrentIndex(1)
			else:
				self.ui.cboGroup2.setCurrentIndex(0)

		self.multiGroupRunTest()
		self.groupRunTest()

	def legendFieldChanged(self):
		self.multiGroupRunTest()

		self.populateGroupComboBoxes()
		self.groupRunTest()

	def legendItemChanged(self):
		self.setGroup1Colour(self.groupLegendDlg.groupColourDict[str(self.ui.cboGroup1.currentText())], False)

		if self.ui.cboGroup2.currentText() != '<All other samples>':
			self.setGroup2Colour(self.groupLegendDlg.groupColourDict[str(self.ui.cboGroup2.currentText())], False)
		else:
			self.setGroup2Colour(self.preferences['All other samples colour'])

		self.groupPlotUpdate()
		self.multiGroupPlotUpdate()

	def appendCategoriesCOG(self):
		assignCOGsDlg = AssignCOGsDlg(self.preferences, self)
		assignCOGsDlg.exec_()

	def sampleProfileTabClicked(self):
		self.ui.widgetSampleProfile.setVisible(not self.ui.widgetSampleProfile.isVisible())
		self.updateSideBarTabIcon(self.ui.widgetSampleProfile, self.ui.btnSampleProfileArrow)

	def samplePropTabClicked(self):
		self.ui.widgetSampleStatisticalProp.setVisible(not self.ui.widgetSampleStatisticalProp.isVisible())
		self.updateSideBarTabIcon(self.ui.widgetSampleStatisticalProp, self.ui.btnSampleStatisticsArrow)

	def sampleFilteringTabClicked(self):
		self.ui.widgetSampleFilter.setVisible(not self.ui.widgetSampleFilter.isVisible())
		self.updateSideBarTabIcon(self.ui.widgetSampleFilter, self.ui.btnSampleFilteringArrow)

	def groupProfileTabClicked(self):
		self.ui.widgetGroupProfile.setVisible(not self.ui.widgetGroupProfile.isVisible())
		self.updateSideBarTabIcon(self.ui.widgetGroupProfile, self.ui.btnGroupProfileArrow)

	def groupPropTabClicked(self):
		self.ui.widgetGroupStatisticalProp.setVisible(not self.ui.widgetGroupStatisticalProp.isVisible())
		self.updateSideBarTabIcon(self.ui.widgetGroupStatisticalProp, self.ui.btnGroupStatisticsArrow)

	def groupFilteringTabClicked(self):
		self.ui.widgetGroupFilter.setVisible(not self.ui.widgetGroupFilter.isVisible())
		self.updateSideBarTabIcon(self.ui.widgetGroupFilter, self.ui.btnGroupFilteringArrow)

	def multiGroupPropTabClicked(self):
		self.ui.widgetMultiGroupStatisticalProp.setVisible(not self.ui.widgetMultiGroupStatisticalProp.isVisible())
		self.updateSideBarTabIcon(self.ui.widgetMultiGroupStatisticalProp, self.ui.btnMultiGroupStatisticsArrow)

	def multiGroupFilteringTabClicked(self):
		self.ui.widgetMultiGroupFiltering.setVisible(not self.ui.widgetMultiGroupFiltering.isVisible())
		self.updateSideBarTabIcon(self.ui.widgetMultiGroupFiltering, self.ui.btnMultiGroupFilteringArrow)

	def updateSideBarTabIcon(self, tab, arrowButton):
		icon = QtGui.QIcon()
		if tab.isVisible():
			icon.addPixmap(QtGui.QPixmap(":/icons/icons/downArrow.png"), QtGui.QIcon.Normal, QtGui.QIcon.Off)
		else:
			icon.addPixmap(QtGui.QPixmap(":/icons/icons/rightArrow.png"), QtGui.QIcon.Normal, QtGui.QIcon.Off)
		arrowButton.setIcon(icon)

	def prefrencesDlg(self):
		preferencesDlg = PreferencesDlg(self)

		preferencesDlg.ui.spinPseudoCount.setValue(self.preferences['Pseudocount'])
		preferencesDlg.ui.spinReplicates.setValue(self.preferences['Replicates'])
		preferencesDlg.ui.chkTruncateFeatureNames.setChecked(self.preferences['Truncate feature names'])
		preferencesDlg.ui.spinFeatureNameLength.setValue(self.preferences['Length of truncated feature names'])
		preferencesDlg.setMinimumReportedPValue(self.preferences['Minimum reported p-value exponent'])
		preferencesDlg.setAxesButtonColour(self.preferences['Axes colour'])
		preferencesDlg.setAllOtherSamplesButtonColour(self.preferences['All other samples colour'])

		if preferencesDlg.exec_() == QtGui.QDialog.Accepted:
			self.preferences['Pseudocount'] = preferencesDlg.ui.spinPseudoCount.value()
			self.preferences['Replicates'] = preferencesDlg.ui.spinReplicates.value()
			self.preferences['Truncate feature names'] = preferencesDlg.ui.chkTruncateFeatureNames.isChecked()
			self.preferences['Length of truncated feature names'] = preferencesDlg.ui.spinFeatureNameLength.value()
			self.preferences['Minimum reported p-value exponent'] = preferencesDlg.getMinimumReportedPValue()

			self.preferences['Axes colour'] = preferencesDlg.getAxesColour()

			if self.preferences['All other samples colour'] != preferencesDlg.getAllOtherSamplesColour():
				self.preferences['All other samples colour'] = preferencesDlg.getAllOtherSamplesColour()
				self.setGroup2Colour(self.preferences['All other samples colour'], False)

		self.samplePlotUpdate()
		self.groupPlotUpdate()
		self.multiGroupPlotUpdate()

	def samplePlotUpdate(self):
		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))
		if self.sampleStatsTest.results.data != []:
			self.samplePlot.update(self.sampleProfile, self.sampleStatsTest.results)
		else:
			self.samplePlot.update(None, None)
		QtGui.QApplication.instance().restoreOverrideCursor()

	def groupPlotUpdate(self):
		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		# update plot
		if self.groupStatsTest.results.data != []:
			self.groupPlot.update(self.groupProfile, self.groupStatsTest.results)
		else:
			self.groupPlot.update(None, None)

		self.ui.cboGroupHighlightHierarchy.setEnabled(self.groupPlot.currentPlot.bSupportsHighlight)
		self.ui.cboGroupHighlightFeature.setEnabled(self.groupPlot.currentPlot.bSupportsHighlight)
		self.ui.frameGroupTable.setVisible(self.groupPlot.currentPlot.bPlotFeaturesIndividually)

		QtGui.QApplication.instance().restoreOverrideCursor()

	def multiGroupPlotUpdate(self):
		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		# update plot
		if self.multiGroupStatsTest.results.data != []:
			if self.multiGroupPlot.checkFlags().bRunPostHocTest:
				coverage = float(self.ui.cboMultiGroupNominalCoverage.currentText())
				postHocTest = self.postHocTestDict[unicode(self.ui.cboPostHocTest.currentText(), 'latin-1')]
				self.multiGroupStatsTest.runPostHocTest(postHocTest, self.multiGroupProfile, self.preferences['Selected multiple group feature'], coverage)

			self.multiGroupPlot.update(self.multiGroupProfile, self.multiGroupStatsTest.results)
		else:
			self.multiGroupPlot.update(None, None)

		self.ui.cboMultiGroupHighlightHierarchy.setEnabled(self.multiGroupPlot.currentPlot.bSupportsHighlight)
		self.ui.cboMultiGroupHighlightFeature.setEnabled(self.multiGroupPlot.currentPlot.bSupportsHighlight)
		self.ui.frameMultiGroupTable.setVisible(self.multiGroupPlot.currentPlot.bPlotFeaturesIndividually)

		QtGui.QApplication.instance().restoreOverrideCursor()

	def samplePlotConfigure(self):
		self.samplePlot.configure(self.sampleProfile, self.sampleStatsTest.results)

	def groupPlotConfigure(self):
		self.groupPlot.configure(self.groupProfile, self.groupStatsTest.results)

	def multiGroupPlotConfigure(self):
		self.multiGroupPlot.configure(self.multiGroupProfile, self.multiGroupStatsTest.results)

	def sample1ColourDlg(self):
		colour = QtGui.QColorDialog.getColor(self.preferences['Sample 1 colour'], self, 'Colour for sample 1')

		if colour.isValid():
			self.preferences['Sample 1 colour'] = colour
			self.setSample1Colour(colour)

	def setSample1Colour(self, colour):
		colourStr = str(colour.red()) + ',' + str(colour.green()) + ',' + str(colour.blue())
		self.ui.btnSample1Colour.setStyleSheet('* { background-color: rgb(' + colourStr + ') }')
		self.samplePlotUpdate()

	def sample2ColourDlg(self):
		colour = QtGui.QColorDialog.getColor(self.preferences['Sample 2 colour'], self, 'Colour for sample 2')

		if colour.isValid():
			self.preferences['Sample 2 colour'] = colour
			self.setSample2Colour(colour)

	def setSample2Colour(self, colour):
		colourStr = str(colour.red()) + ',' + str(colour.green()) + ',' + str(colour.blue())
		self.ui.btnSample2Colour.setStyleSheet('* { background-color: rgb(' + colourStr + ') }')
		self.samplePlotUpdate()

	def group1ColourDlg(self):
		colour = QtGui.QColorDialog.getColor(self.preferences['Group colours'][self.groupProfile.groupName1], self, 'Colour for group 1')

		if colour.isValid():
			self.setGroup1Colour(colour)

	def setGroup1Colour(self, colour, bUpdatePlot=True):
		colourStr = str(colour.red()) + ',' + str(colour.green()) + ',' + str(colour.blue())
		self.ui.btnGroup1Colour.setStyleSheet('* { background-color: rgb(' + colourStr + ') }')
		self.preferences['Group colours'][self.groupProfile.groupName1] = colour

		if bUpdatePlot:
			self.groupLegendDlg.updateLegend(self.groupProfile.groupName1, colour)
			self.groupPlotUpdate()

	def group2ColourDlg(self):
		colour = QtGui.QColorDialog.getColor(self.preferences['Group colours'][self.groupProfile.groupName2], self, 'Colour for group 2')

		if colour.isValid():
			self.setGroup2Colour(colour)

	def setGroup2Colour(self, colour, bUpdatePlot=True):
		colourStr = str(colour.red()) + ',' + str(colour.green()) + ',' + str(colour.blue())
		self.ui.btnGroup2Colour.setStyleSheet('* { background-color: rgb(' + colourStr + ') }')
		self.preferences['Group colours'][self.groupProfile.groupName2] = colour

		if bUpdatePlot:
			if self.groupProfile.groupName2 != '<All other samples>':
				self.groupLegendDlg.updateLegend(self.groupProfile.groupName2, colour)
			else:
				self.preferences['All other samples colour'] = colour
			self.groupPlotUpdate()

	def createProfileMgRast(self):
		createProfileMgRastDlg = CreateProfileMgRastDlg(self.preferences, self)
		createProfileMgRastDlg.exec_()

	def createProfileRita(self):
		createProfileRITADlg = CreateProfileRITADlg(self.preferences, self)
		createProfileRITADlg.exec_()

	def createProfileComet(self):
		createProfileCoMetDlg = CreateProfileCoMetDlg(self.preferences, self)
		createProfileCoMetDlg.exec_()

	def createProfileMothur(self):
		createProfileMothurDlg = CreateProfileMothurDlg(self.preferences, self)
		createProfileMothurDlg.exec_()

	def createProfileBIOM(self):
		createProfileBiomDlg = CreateProfileBiomDlg(self.preferences, self)
		createProfileBiomDlg.exec_()

	def loadProfile(self):
		loadDataDlg = LoadDataDlg(self.preferences, self)
		if loadDataDlg.exec_() == QtGui.QDialog.Accepted:
			profileFile = loadDataDlg.getProfileFile()
			if profileFile == '':
				return
			self.preferences['Last directory'] = profileFile[0:profileFile.lastIndexOf('/')]

			metadataFile = loadDataDlg.getMetadataFile()

			# read profiles from file
			try:
				stampIO = StampIO(self.preferences)
				self.profileTree, errMsg = stampIO.read(profileFile)

				if errMsg != None:
					QtGui.QMessageBox.information(self, 'Error reading profile file', errMsg, QtGui.QMessageBox.Warning)
					return
			except:
				QtGui.QMessageBox.information(self, 'Error reading profile file', 'Unknown parsing error.', QtGui.QMessageBox.Warning)
				return

			self.metadata = None
			if metadataFile != '':
				try:
					metadataIO = MetadataIO(self.preferences)
					self.metadata, warningMsg = metadataIO.read(metadataFile, self.profileTree)

					if warningMsg != None:
						QtGui.QMessageBox.information(self, 'Metadata warnings', warningMsg)
				except:
					QtGui.QMessageBox.information(self, 'Error reading metadata file', 'Unknown parsing error.', QtGui.QMessageBox.Warning)
					return

			QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

			# populate sample combo boxes
			self.populateSampleComboBoxes()

			# populate hierarchy combo boxes
			self.ui.cboParentalLevel.clear()
			self.ui.cboParentalLevel.addItem('Entire sample')
			for header in self.profileTree.hierarchyHeadings[0:-1]:
				self.ui.cboParentalLevel.addItem(header)
			self.ui.cboParentalLevel.setCurrentIndex(0)

			self.ui.cboProfileLevel.clear()
			for header in self.profileTree.hierarchyHeadings:
				self.ui.cboProfileLevel.addItem(header)
			self.ui.cboProfileLevel.setCurrentIndex(0)

			# setup group legend
			if self.metadata != None and len(self.metadata.getFeatures()) != 0:
				self.groupLegendDlg.initLegend(self.profileTree, self.metadata, self.metadata.getFeatures()[0])
				self.preferences['Group colours'] = self.groupLegendDlg.groupColourDict

			# populate group combo box
			self.populateGroupComboBoxes()

			# indicate the hierarchical level of interest has changed
			bGroupLegendVisibility = self.groupLegendDlg.isVisible()
			if platform.system() != 'Windows' and bGroupLegendVisibility:
				self.groupLegendDlg.setVisible(False)  # HACK: OS X crashes if this dialog is open when loading data for the first time!

			self.multiGroupHierarchicalLevelsChanged()
			self.groupHierarchicalLevelsChanged()
			self.sampleHierarchicalLevelsChanged()

			if platform.system() != 'Windows' and bGroupLegendVisibility:
				self.groupLegendDlg.setVisible(True)

			# update tables
			self.groupFeaturesTableUpdate()
			self.multiGroupFeaturesTableUpdate()
			self.metadataDlg.setTable(self.metadata)

			QtGui.QApplication.instance().restoreOverrideCursor()

	def populateGroupComboBoxes(self):
		self.ui.cboGroup1.clear()
		self.ui.cboGroup2.clear()
		for name, bActive in sorted(self.profileTree.groupActive.items()):
			if bActive:
				self.ui.cboGroup1.addItem(name)
				self.ui.cboGroup2.addItem(name)
		self.ui.cboGroup2.addItem('<All other samples>')
		self.ui.cboGroup1.setCurrentIndex(0)
		self.ui.cboGroup2.setCurrentIndex(1)

	def parentLevelChanged(self):
		parentDepth = self.profileTree.getHierarchicalLevelDepth(str(self.ui.cboParentalLevel.currentText()))
		profileDepth = self.profileTree.getHierarchicalLevelDepth(str(self.ui.cboProfileLevel.currentText()))
		if parentDepth >= profileDepth:
			QtGui.QMessageBox.information(self, 'Invalid profile', 'The parent level must be higher in the hierarchy than the profile level.', QtGui.QMessageBox.Warning)
			self.ui.cboParentalLevel.setCurrentIndex(0)

		self.sampleHierarchicalLevelsChanged()
		self.groupHierarchicalLevelsChanged()
		self.multiGroupHierarchicalLevelsChanged()

	def profileLevelChanged(self):
		parentDepth = self.profileTree.getHierarchicalLevelDepth(str(self.ui.cboParentalLevel.currentText()))
		profileDepth = self.profileTree.getHierarchicalLevelDepth(str(self.ui.cboProfileLevel.currentText()))

		if profileDepth <= parentDepth:
			QtGui.QMessageBox.information(self, 'Invalid profile', 'The profile level must be deeper in the hierarchy than the parent level.', QtGui.QMessageBox.Warning)
			self.ui.cboProfileLevel.setCurrentIndex(len(self.profileTree.hierarchyHeadings) - 1)
			return

		self.sampleHierarchicalLevelsChanged()

		self.preferences['Selected group feature'] = ''
		self.groupHierarchicalLevelsChanged()

		self.preferences['Selected multiple group feature'] = ''
		self.multiGroupHierarchicalLevelsChanged()

	def unclassifiedTreatmentChanged(self):
		self.sampleRunTest()

		if self.preferences['Selected group feature'].lower() == 'unclassified':
			self.preferences['Selected group feature'] = ''
		self.groupRunTest()

		if self.preferences['Selected multiple group feature'].lower() == 'unclassified':
			self.preferences['Selected multiple group feature'] = ''
		self.multiGroupRunTest()

	def sampleHierarchicalLevelsChanged(self):
		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		# populate highlight hierarchy combo box
		profileHeading = str(self.ui.cboProfileLevel.currentText())
		profileIndex = self.profileTree.hierarchyHeadings.index(profileHeading)
		self.ui.cboSampleHighlightHierarchy.clear()
		self.ui.cboSampleHighlightHierarchy.addItem('None')
		for header in self.profileTree.hierarchyHeadings[0:profileIndex + 1]:
			self.ui.cboSampleHighlightHierarchy.addItem(header)
		self.ui.cboSampleHighlightHierarchy.setCurrentIndex(0)

		self.ui.cboSampleHighlightFeature.clear()

		# keep selected features
		selectedFeatures = self.sampleStatsTest.results.getSelectedFeatures()
		self.sampleStatsTest = SampleStatsTests(self.preferences)
		self.sampleStatsTest.results.setSelectedFeatures(selectedFeatures)

		QtGui.QApplication.instance().restoreOverrideCursor()

		# run statistics
		self.sampleRunTest()

		self.updateStatusBar()

	def groupHierarchicalLevelsChanged(self):
		if self.metadata == None:
			return

		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		# set group colours
		groupName1 = str(self.ui.cboGroup1.currentText())
		groupName2 = str(self.ui.cboGroup2.currentText())

		if groupName1 == '' or groupName2 == '':
			QtGui.QApplication.instance().restoreOverrideCursor()
			return

		# populate highlight hierarchy combo box
		profileHeading = str(self.ui.cboProfileLevel.currentText())
		profileIndex = self.profileTree.hierarchyHeadings.index(profileHeading)
		self.ui.cboGroupHighlightHierarchy.clear()
		self.ui.cboGroupHighlightHierarchy.addItem('None')
		for header in self.profileTree.hierarchyHeadings[0:profileIndex + 1]:
			self.ui.cboGroupHighlightHierarchy.addItem(header)
		self.ui.cboGroupHighlightHierarchy.setCurrentIndex(0)

		self.ui.cboGroupHighlightFeature.clear()

		# keep selected features
		selectedFeatures = self.groupStatsTest.results.getSelectedFeatures()
		self.groupStatsTest = GroupStatsTests(self.preferences)
		self.groupStatsTest.results.setSelectedFeatures(selectedFeatures)

		QtGui.QApplication.instance().restoreOverrideCursor()

		# run statistics
		self.groupRunTest()

		self.updateStatusBar()

	def multiGroupHierarchicalLevelsChanged(self):
		if self.metadata == None:
			return

		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		# populate highlight hierarchy combo box
		profileHeading = str(self.ui.cboProfileLevel.currentText())
		profileIndex = self.profileTree.hierarchyHeadings.index(profileHeading)
		self.ui.cboMultiGroupHighlightHierarchy.clear()
		self.ui.cboMultiGroupHighlightHierarchy.addItem('None')
		for header in self.profileTree.hierarchyHeadings[0:profileIndex + 1]:
			self.ui.cboMultiGroupHighlightHierarchy.addItem(header)
		self.ui.cboMultiGroupHighlightHierarchy.setCurrentIndex(0)

		self.ui.cboMultiGroupHighlightFeature.clear()

		# keep selected features
		selectedFeatures = self.multiGroupStatsTest.results.getSelectedFeatures()
		self.multiGroupStatsTest = MultiGroupStatsTests(self.preferences)
		self.multiGroupStatsTest.results.setSelectedFeatures(selectedFeatures)

		QtGui.QApplication.instance().restoreOverrideCursor()

		# run test
		self.multiGroupRunTest()

		self.updateStatusBar()

	def groupFeaturesTableUpdate(self):
		tableData = []

		bActiveFeatures = self.ui.chkShowActiveFeaturesGroupTable.isChecked()

		features = self.groupStatsTest.results.getColumn('Features', bActiveFeatures)
		if len(features) != 0:
			effectSizes = self.groupStatsTest.results.getColumnAsFloatStr('EffectSize', bActiveFeatures)
			pValues = self.groupStatsTest.results.getColumnAsStr('pValues', bActiveFeatures)
			pValuesCorrected = self.groupStatsTest.results.getColumnAsStr('pValuesCorrected', bActiveFeatures)
			notes = self.groupStatsTest.results.getColumn('Note', bActiveFeatures)

			for i in xrange(0, len(features)):
				tableData.append([features[i], effectSizes[i], pValues[i], pValuesCorrected[i], notes[i]])

		if self.preferences['Selected group feature'] not in features:
			self.preferences['Selected group feature'] = ''

		self.groupFeatureTable = GenericTable(tableData, ['Feature', 'Diff. between means', 'p-value', 'Corrected p-value', 'Note'], self)
		self.groupFeatureTable.sort(0, QtCore.Qt.AscendingOrder)  # start with features in alphabetical order

		self.ui.tableGroupFeatures.horizontalHeader().setStretchLastSection(True)
		self.ui.tableGroupFeatures.setModel(self.groupFeatureTable)
		self.ui.tableGroupFeatures.verticalHeader().setVisible(True)
		self.ui.tableGroupFeatures.resizeColumnsToContents()

		self.tableGroupSelectionModel = QtGui.QItemSelectionModel(self.groupFeatureTable, self.ui.tableGroupFeatures)
		self.ui.tableGroupFeatures.setSelectionModel(self.tableGroupSelectionModel)
		self.connect(self.tableGroupSelectionModel, QtCore.SIGNAL('selectionChanged(const QItemSelection, const QItemSelection)'), self.groupTableFeatureChanged)

	def multiGroupFeaturesTableUpdate(self):
		tableData = []

		bActiveFeatures = self.ui.chkShowActiveFeaturesMultiGroupTable.isChecked()

		features = self.multiGroupStatsTest.results.getColumn('Features', bActiveFeatures)
		if len(features) != 0:
			effectSizes = self.multiGroupStatsTest.results.getColumnAsFloatStr('EffectSize', bActiveFeatures)
			pValues = self.multiGroupStatsTest.results.getColumnAsStr('pValues', bActiveFeatures)
			pValuesCorrected = self.multiGroupStatsTest.results.getColumnAsStr('pValuesCorrected', bActiveFeatures)
			notes = self.multiGroupStatsTest.results.getColumn('Note', bActiveFeatures)

			for i in xrange(0, len(features)):
				tableData.append([features[i], effectSizes[i], pValues[i], pValuesCorrected[i], notes[i]])

		if self.preferences['Selected multiple group feature'] not in features:
			self.preferences['Selected multiple group feature'] = ''

		self.multiGroupFeatureTable = GenericTable(tableData, ['Feature', 'Eta-squared', 'p-value', 'Corrected p-value', 'Note'], self)
		self.multiGroupFeatureTable.sort(0, QtCore.Qt.AscendingOrder)  # start with features in alphabetical order

		self.ui.tableMultiGroupFeatures.horizontalHeader().setStretchLastSection(True)
		self.ui.tableMultiGroupFeatures.setModel(self.multiGroupFeatureTable)
		self.ui.tableMultiGroupFeatures.verticalHeader().setVisible(True)
		self.ui.tableMultiGroupFeatures.resizeColumnsToContents()

		self.tableMultiGroupSelectionModel = QtGui.QItemSelectionModel(self.multiGroupFeatureTable, self.ui.tableMultiGroupFeatures)
		self.ui.tableMultiGroupFeatures.setSelectionModel(self.tableMultiGroupSelectionModel)
		self.connect(self.tableMultiGroupSelectionModel, QtCore.SIGNAL('selectionChanged(const QItemSelection, const QItemSelection)'), self.multiGroupSideTableFeatureChanged)

	def groupTableFeatureChanged(self, selected, deselected):
		selectedRow = selected.indexes()[0]
		self.preferences['Selected group feature'] = str(selectedRow.data(QtCore.Qt.DisplayRole).toString())
		self.groupPlotUpdate()

	def multiGroupSideTableFeatureChanged(self, selected, deselected):
		selectedRow = selected.indexes()[0]
		self.preferences['Selected multiple group feature'] = str(selectedRow.data(QtCore.Qt.DisplayRole).toString())
		self.multiGroupPlotUpdate()

	def sampleRunTest(self):
		if self.bAutoRecalculate == False:
			return

		sampleName1 = str(self.ui.cboSample1.currentText())
		sampleName2 = str(self.ui.cboSample2.currentText())

		if sampleName1 != '' and sampleName2 != '':
			# create new profile
			parentHeading = str(self.ui.cboParentalLevel.currentText())
			profileHeading = str(self.ui.cboProfileLevel.currentText())

			self.sampleProfile = self.profileTree.createSampleProfile(sampleName1, sampleName2,
																		parentHeading, profileHeading,
																		self.ui.cboUnclassified.currentText())

			# show progress of test
			progress = QtGui.QProgressDialog('Running two-sample statistical test...', 'Cancel', 0, len(self.sampleProfile.getFeatures()) + 1, self)
			progress.setWindowTitle('Progress')
			progress.setWindowModality(QtCore.Qt.WindowModal)
			progress.setVisible(True)

			# run significance test
			test = self.sampleStatTestDict[str(self.ui.cboSampleStatTests.currentText())]
			testType = str(self.ui.cboSampleSignTestType.currentText())
			confIntervMethod = self.sampleConfIntervMethodDict[str(self.ui.cboSampleConfIntervMethods.currentText())]
			coverage = float(self.ui.cboSampleNominalCoverage.currentText())
			self.sampleStatsTest.run(test, testType, confIntervMethod, coverage, self.sampleProfile, progress)

			if not progress.wasCanceled():
				# apply multiple test correction
				multCompClass = self.multCompDict[str(self.ui.cboSampleMultCompMethod.currentText())]
				self.sampleStatsTest.results.performMultCompCorrection(multCompClass)

				# apply filters
				self.sampleApplyFilters()
		else:
			self.sampleStatsTest.results.data = []
			self.sampleApplyFilters()

	def groupRunTest(self):
		self.groupTestConfIntervMethods()

		if self.metadata == None or self.bAutoRecalculate == False:
			return

		groupName1 = str(self.ui.cboGroup1.currentText())
		groupName2 = str(self.ui.cboGroup2.currentText())

		if groupName1 != '' and groupName2 != '':
			# create new profile
			parentHeading = str(self.ui.cboParentalLevel.currentText())
			profileHeading = str(self.ui.cboProfileLevel.currentText())

			self.groupProfile = self.profileTree.createGroupProfile(groupName1, groupName2,
																																	parentHeading, profileHeading, self.metadata,
																																	self.ui.cboUnclassified.currentText())

			self.setGroup1Colour(self.preferences['Group colours'][groupName1], False)
			if groupName2 != '<All other samples>':
				self.setGroup2Colour(self.preferences['Group colours'][groupName2], False)
			else:
				self.setGroup2Colour(self.preferences['All other samples colour'])

			# show progress of test
			progress = QtGui.QProgressDialog('Running two-group statistical test...', 'Cancel', 0, len(self.groupProfile.getFeatures()) + 1, self)
			progress.setWindowTitle('Progress')
			progress.setWindowModality(QtCore.Qt.WindowModal)
			progress.setVisible(True)

			# run significance test
			test = self.groupStatTestDict[str(self.ui.cboGroupStatTests.currentText())]
			testType = str(self.ui.cboGroupSignTestType.currentText())
			confIntervMethod = self.ui.cboGroupConfIntervMethods.currentText()
			coverage = float(self.ui.cboGroupNominalCoverage.currentText())
			self.groupStatsTest.run(test, testType, confIntervMethod, coverage, self.groupProfile, progress)

			if not progress.wasCanceled():
				# apply multiple test correction
				multCompClass = self.multCompDict[str(self.ui.cboGroupMultCompMethod.currentText())]
				self.groupStatsTest.results.performMultCompCorrection(multCompClass)

				# apply filters
				self.groupApplyFilters()
		else:
			self.groupStatsTest.results.data = []
			self.groupApplyFilters()

	def multiGroupRunTest(self):
		if self.metadata == None or self.bAutoRecalculate == False:
			return

		# create new profile
		parentHeading = str(self.ui.cboParentalLevel.currentText())
		profileHeading = str(self.ui.cboProfileLevel.currentText())

		self.multiGroupProfile = self.profileTree.createMultiGroupProfile(self.profileTree.groupDict.keys(),
																			parentHeading, profileHeading, self.metadata,
																			self.ui.cboUnclassified.currentText())

		# set active groups
		self.multiGroupProfile.setActiveGroups(self.profileTree.groupActive)

		if len(self.multiGroupProfile.activeGroupNames) >= 2:
			# show progress of test
			progress = QtGui.QProgressDialog('Running multiple groups statistical test...', 'Cancel', 0, len(self.multiGroupProfile.getFeatures()) + 1, self)
			progress.setWindowTitle('Progress')
			progress.setWindowModality(QtCore.Qt.WindowModal)
			progress.setVisible(True)

			# run significance test
			hypothesisTest = self.multiGroupStatTestDict[str(self.ui.cboMultiGroupStatTests.currentText())]
			effectSizeMeasure = self.multiGroupEffectSizeDict[str(self.ui.cboMultiGroupEffectSizeMeasure.currentText())]
			self.multiGroupStatsTest.run(hypothesisTest, effectSizeMeasure, self.multiGroupProfile, progress)

			if self.multiGroupStatsTest.results.data != []:
				# apply multiple test correction
				multCompClass = self.multCompDict[str(self.ui.cboMultiGroupMultCompMethod.currentText())]
				self.multiGroupStatsTest.results.performMultCompCorrection(multCompClass)

				# apply filters
				self.multiGroupApplyFilters()
		else:
			self.multiGroupStatsTest.results.data = []
			self.multiGroupApplyFilters()

	def groupTestConfIntervMethods(self):
		# populate combo box with CI methods compatible with current hypothesis test
		test = self.groupStatTestDict[str(self.ui.cboGroupStatTests.currentText())]
		self.ui.cboGroupConfIntervMethods.clear()
		self.ui.cboGroupConfIntervMethods.insertItems(len(test.confIntervMethods), test.confIntervMethods)
		self.ui.cboGroupConfIntervMethods.setCurrentIndex(0)
		self.ui.cboGroupConfIntervMethods.adjustSize()

		index = self.ui.cboGroupNominalCoverage.currentIndex()
		self.ui.cboGroupNominalCoverage.clear()
		if test.confIntervMethods[0] == '<none>':
			self.ui.cboGroupNominalCoverage.insertItems(1, ['0.0'])
		else:
			self.ui.cboGroupNominalCoverage.insertItems(5, ['0.90', '0.95', '0.98', '0.99', '0.999'])
			self.ui.cboGroupNominalCoverage.setCurrentIndex(index)

	def sampleMultCompCorrectionChanged(self):
		multCompClass = self.multCompDict[str(self.ui.cboSampleMultCompMethod.currentText())]
		if multCompClass.method == 'False discovery rate':
			self.ui.lblSampleSignLevelFilter.setText('q-value filter (>):')
		else:
			self.ui.lblSampleSignLevelFilter.setText('p-value filter (>):')
		self.sampleRunTest()

	def groupMultCompCorrectionChanged(self):
		multCompClass = self.multCompDict[str(self.ui.cboGroupMultCompMethod.currentText())]
		if multCompClass.method == 'False discovery rate':
			self.ui.lblGroupSignLevelFilter.setText('q-value filter (>):')
		else:
			self.ui.lblGroupSignLevelFilter.setText('p-value filter (>):')

		self.groupRunTest()

	def multiGroupMultCompCorrectionChanged(self):
		multCompClass = self.multCompDict[str(self.ui.cboMultiGroupMultCompMethod.currentText())]
		if multCompClass.method == 'False discovery rate':
			self.ui.lblMultiGroupSignLevelFilter.setText('q-value filter (>):')
		else:
			self.ui.lblMultiGroupSignLevelFilter.setText('p-value filter (>):')
		self.multiGroupRunTest()

	def sampleMultCompCorrectionInfo(self):
		if self.sampleStatsTest.results.multCompCorrection != None:
			multCompDlg = MultCompCorrectionInfoDlg(self, self.sampleStatsTest.results.multCompCorrectionInfo)
			multCompDlg.exec_()
		else:
			QtGui.QMessageBox.information(self, 'Run test', 'Run hypothesis test first.', QtGui.QMessageBox.Ok)

	def groupMultCompCorrectionInfo(self):
		if self.groupStatsTest.results.multCompCorrection != None:
			multCompDlg = MultCompCorrectionInfoDlg(self, self.groupStatsTest.results.multCompCorrectionInfo)
			multCompDlg.exec_()
		else:
			QtGui.QMessageBox.information(self, 'Run test', 'Run hypothesis test first.', QtGui.QMessageBox.Ok)

	def multiGroupMultCompCorrectionInfo(self):
		if self.multiGroupStatsTest.results.multCompCorrection != None:
			multCompDlg = MultCompCorrectionInfoDlg(self, self.multiGroupStatsTest.results.multCompCorrectionInfo)
			multCompDlg.exec_()
		else:
			QtGui.QMessageBox.information(self, 'Run test', 'Run hypothesis test first.', QtGui.QMessageBox.Ok)

	def sampleSeqFilterChanged(self):
		if self.ui.cboSampleSeqFilter.currentText() == 'maximum':
			self.ui.lblSampleSeqFilterSample1.setText('Maximum (<):')

		elif self.ui.cboSampleSeqFilter.currentText() == 'minimum':
			self.ui.lblSampleSeqFilterSample1.setText('Minimum (<):')

		elif self.ui.cboSampleSeqFilter.currentText() == 'independent':
			self.ui.lblSampleSeqFilterSample1.setText('Sample 1 (<):')

		self.sampleFilteringPropChanged()

	def sampleParentSeqFilterChanged(self):
		if self.ui.cboSampleParentSeqFilter.currentText() == 'maximum':
			self.ui.lblSampleParentSeqFilterSample1.setText('Maximum (<):')

		elif self.ui.cboSampleParentSeqFilter.currentText() == 'minimum':
			self.ui.lblSampleParentSeqFilterSample1.setText('Minimum (<):')

		elif self.ui.cboSampleParentSeqFilter.currentText() == 'independent':
			self.ui.lblSampleParentSeqFilterSample1.setText('Sample 1 (<):')

		self.sampleFilteringPropChanged()

	def groupSeqFilterChanged(self):
		if self.ui.cboGroupSeqFilter.currentText() == 'maximum':
			self.ui.lblGroupSeqFilter1.setText('Maximum (<):')
		elif self.ui.cboGroupSeqFilter.currentText() == 'minimum':
			self.ui.lblGroupSeqFilter1.setText('Minimum (<):')
		elif self.ui.cboGroupSeqFilter.currentText() == 'independent, maximum':
			self.ui.lblGroupSeqFilter1.setText('Max. group 1 (<):')
			self.ui.lblGroupSeqFilter2.setText('Max. group 2 (<):')
		elif self.ui.cboGroupSeqFilter.currentText() == 'independent, minimum':
			self.ui.lblGroupSeqFilter1.setText('Min. group 1 (<):')
			self.ui.lblGroupSeqFilter2.setText('Min. group 2 (<):')

		self.groupFilteringPropChanged()

	def groupParentSeqFilterChanged(self):
		if self.ui.cboGroupParentSeqFilter.currentText() == 'maximum':
			self.ui.lblGroupParentSeqFilter1.setText('Maximum (<):')
		elif self.ui.cboGroupParentSeqFilter.currentText() == 'minimum':
			self.ui.lblGroupParentSeqFilter1.setText('Minimum (<):')
		elif self.ui.cboGroupParentSeqFilter.currentText() == 'independent, maximum':
			self.ui.lblGroupParentSeqFilter1.setText('Max. group 1 (<):')
			self.ui.lblGroupParentSeqFilter2.setText('Max. group 2 (<):')
		elif self.ui.cboGroupParentSeqFilter.currentText() == 'independent, minimum':
			self.ui.lblGroupParentSeqFilter1.setText('Min. group 1 (<):')
			self.ui.lblGroupParentSeqFilter2.setText('Min. group 2 (<):')

		self.groupFilteringPropChanged()

	def sampleChangeEffectSizeMeasure(self):
		self.sampleFilteringPropChanged()

	def groupChangeEffectSizeMeasure(self):
		self.groupFilteringPropChanged()

	def sampleFilteringPropChanged(self):
		# indicate that profile information has changed
		self.ui.btnSampleSelectFeatures.setEnabled(self.ui.chkSampleSelectFeatures.isChecked())

		self.ui.spinSampleSignLevelFilter.setEnabled(self.ui.chkSampleEnableSignLevelFilter.isChecked())

		self.ui.cboSampleSeqFilter.setEnabled(self.ui.chkSampleEnableSeqFilter.isChecked())
		self.ui.spinSampleFilterSample1.setEnabled(self.ui.chkSampleEnableSeqFilter.isChecked())
		self.ui.spinSampleFilterSample2.setEnabled(self.ui.chkSampleEnableSeqFilter.isChecked() and self.ui.cboSampleSeqFilter.currentText() == 'independent')
		self.ui.lblSampleSeqFilterSample2.setEnabled(self.ui.cboSampleSeqFilter.currentText() == 'independent')

		self.ui.cboSampleParentSeqFilter.setEnabled(self.ui.chkSampleEnableParentSeqFilter.isChecked())
		self.ui.spinSampleParentFilterSample1.setEnabled(self.ui.chkSampleEnableParentSeqFilter.isChecked())
		self.ui.spinSampleParentFilterSample2.setEnabled(self.ui.chkSampleEnableParentSeqFilter.isChecked() and self.ui.cboSampleParentSeqFilter.currentText() == 'independent')
		self.ui.lblSampleParentSeqFilterSample2.setEnabled(self.ui.cboSampleParentSeqFilter.currentText() == 'independent')

		self.ui.cboSampleEffectSizeMeasure1.setEnabled(self.ui.chkSampleEnableEffectSizeFilter1.isChecked())
		self.ui.spinSampleMinEffectSize1.setEnabled(self.ui.chkSampleEnableEffectSizeFilter1.isChecked())

		self.ui.cboSampleEffectSizeMeasure2.setEnabled(self.ui.chkSampleEnableEffectSizeFilter2.isChecked())
		self.ui.spinSampleMinEffectSize2.setEnabled(self.ui.chkSampleEnableEffectSizeFilter2.isChecked())

		self.sampleApplyFilters()

	def groupFilteringPropChanged(self):
		# indicate that profile information has changed
		self.ui.btnGroupSelectFeatures.setEnabled(self.ui.chkGroupSelectFeatures.isChecked())

		self.ui.spinGroupSignLevelFilter.setEnabled(self.ui.chkGroupEnableSignLevelFilter.isChecked())

		bIndependent = (self.ui.cboGroupSeqFilter.currentText() == 'independent, maximum' or self.ui.cboGroupSeqFilter.currentText() == 'independent, minimum')
		self.ui.cboGroupSeqFilter.setEnabled(self.ui.chkGroupEnableSeqFilter.isChecked())
		self.ui.spinGroupFilter1.setEnabled(self.ui.chkGroupEnableSeqFilter.isChecked())
		self.ui.spinGroupFilter2.setEnabled(self.ui.chkGroupEnableSeqFilter.isChecked() and bIndependent)
		self.ui.lblGroupSeqFilter2.setEnabled(bIndependent)

		bParentIndependent = (self.ui.cboGroupParentSeqFilter.currentText() == 'independent, maximum' or self.ui.cboGroupParentSeqFilter.currentText() == 'independent, minimum')
		self.ui.cboGroupParentSeqFilter.setEnabled(self.ui.chkGroupEnableParentSeqFilter.isChecked())
		self.ui.spinGroupParentFilter1.setEnabled(self.ui.chkGroupEnableParentSeqFilter.isChecked())
		self.ui.spinGroupParentFilter2.setEnabled(self.ui.chkGroupEnableParentSeqFilter.isChecked() and bParentIndependent)
		self.ui.lblGroupParentSeqFilter2.setEnabled(bParentIndependent)

		self.ui.cboGroupEffectSizeMeasure1.setEnabled(self.ui.chkGroupEnableEffectSizeFilter1.isChecked())
		self.ui.spinGroupMinEffectSize1.setEnabled(self.ui.chkGroupEnableEffectSizeFilter1.isChecked())

		self.ui.cboGroupEffectSizeMeasure2.setEnabled(self.ui.chkGroupEnableEffectSizeFilter2.isChecked())
		self.ui.spinGroupMinEffectSize2.setEnabled(self.ui.chkGroupEnableEffectSizeFilter2.isChecked())

		self.groupApplyFilters()

	def multiGroupFilteringPropChanged(self):
		# indicate that profile information has changed
		self.ui.btnMultiGroupSelectFeatures.setEnabled(self.ui.chkMultiGroupSelectFeatures.isChecked())
		self.ui.spinMultiGroupSignLevelFilter.setEnabled(self.ui.chkMultiGroupEnableSignLevelFilter.isChecked())
		self.ui.spinMultiGroupMinEffectSize.setEnabled(self.ui.chkMultiGroupEnableEffectSizeFilter.isChecked())
		self.multiGroupApplyFilters()

	def sampleSelectFeaturesCheckbox(self):
		self.sampleFilteringPropChanged()

	def sampleSelectFeaturesDlg(self):
		selectFeatureDialog = SelectFeaturesDlg(self.sampleStatsTest.results, self)

		if selectFeatureDialog.exec_() == QtGui.QDialog.Accepted:
			selectedFeatures = selectFeatureDialog.getSelectedFeatures()
			self.sampleStatsTest.results.setSelectedFeatures(selectedFeatures)
			self.sampleFilteringPropChanged()

	def groupSelectFeaturesCheckbox(self):
		self.groupFilteringPropChanged()

	def groupSelectFeaturesDlg(self):
		selectFeatureDialog = SelectFeaturesDlg(self.groupStatsTest.results, self)

		if selectFeatureDialog.exec_() == QtGui.QDialog.Accepted:
			selectedFeatures = selectFeatureDialog.getSelectedFeatures()
			self.groupStatsTest.results.setSelectedFeatures(selectedFeatures)
			self.groupFilteringPropChanged()

	def multiGroupSelectFeaturesCheckbox(self):
		self.multiGroupFilteringPropChanged()

	def multiGroupSelectFeaturesDlg(self):
		selectFeatureDialog = SelectFeaturesDlg(self.multiGroupStatsTest.results, self)

		if selectFeatureDialog.exec_() == QtGui.QDialog.Accepted:
			selectedFeatures = selectFeatureDialog.getSelectedFeatures()
			self.multiGroupStatsTest.results.setSelectedFeatures(selectedFeatures)
			self.multiGroupFilteringPropChanged()

	def sampleApplyFilters(self):
		if self.bAutoRecalculate == False:
			return

		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		if not self.ui.chkSampleSelectFeatures.isChecked():
			self.sampleStatsTest.results.selectAllFeautres()

		# perform filtering
		signLevelFilter = self.ui.spinSampleSignLevelFilter.value()
		if not self.ui.chkSampleEnableSignLevelFilter.isChecked():
			signLevelFilter = None

		# sequence filtering
		seqFilter = str(self.ui.cboSampleSeqFilter.currentText())
		sample1Filter = int(self.ui.spinSampleFilterSample1.value())
		sample2Filter = int(self.ui.spinSampleFilterSample2.value())
		if not self.ui.chkSampleEnableSeqFilter.isChecked():
			seqFilter = None
			sample1Filter = None
			sample2Filter = None

		parentSeqFilter = str(self.ui.cboSampleParentSeqFilter.currentText())
		parentSample1Filter = int(self.ui.spinSampleParentFilterSample1.value())
		parentSample2Filter = int(self.ui.spinSampleParentFilterSample2.value())
		if not self.ui.chkSampleEnableParentSeqFilter.isChecked():
			parentSeqFilter = None
			parentSample1Filter = None
			parentSample2Filter = None

		# effect size filters
		if self.ui.chkSampleEnableEffectSizeFilter1.isChecked():
			effectSizeMeasure1 = self.sampleEffectSizeDict[str(self.ui.cboSampleEffectSizeMeasure1.currentText())]
			minEffectSize1 = float(self.ui.spinSampleMinEffectSize1.value())
		else:
			effectSizeMeasure1 = None
			minEffectSize1 = None

		if self.ui.chkSampleEnableEffectSizeFilter2.isChecked():
			effectSizeMeasure2 = self.sampleEffectSizeDict[str(self.ui.cboSampleEffectSizeMeasure2.currentText())]
			minEffectSize2 = float(self.ui.spinSampleMinEffectSize2.value())
		else:
			effectSizeMeasure2 = None
			minEffectSize2 = None

		if self.ui.radioSampleOR.isChecked():
			effectSizeOperator = 'OR'
		else:
			effectSizeOperator = 'AND'

		self.sampleStatsTest.results.filterFeatures(signLevelFilter, seqFilter, sample1Filter, sample2Filter,
																							parentSeqFilter, parentSample1Filter, parentSample2Filter,
																							effectSizeMeasure1, minEffectSize1, effectSizeOperator,
																							effectSizeMeasure2, minEffectSize2)

		# update table summarizing statistical results
		self.sampleUpdateFilterInfo()
		self.sampleTable.updateTable(self.sampleStatsTest)

		# update plots
		self.samplePlotUpdate()

		QtGui.QApplication.instance().restoreOverrideCursor()

	def groupApplyFilters(self):
		if self.metadata == None or self.bAutoRecalculate == False:
			return

		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		if not self.ui.chkGroupSelectFeatures.isChecked():
			self.groupStatsTest.results.selectAllFeautres()

		# perform filtering
		signLevelFilter = self.ui.spinGroupSignLevelFilter.value()
		if not self.ui.chkGroupEnableSignLevelFilter.isChecked():
			signLevelFilter = None

		# sequence filtering
		seqFilter = str(self.ui.cboGroupSeqFilter.currentText())
		group1Filter = int(self.ui.spinGroupFilter1.value())
		group2Filter = int(self.ui.spinGroupFilter2.value())
		if not self.ui.chkGroupEnableSeqFilter.isChecked():
			seqFilter = None
			group1Filter = None
			group2Filter = None

		parentSeqFilter = str(self.ui.cboGroupParentSeqFilter.currentText())
		parentGroup1Filter = int(self.ui.spinGroupParentFilter1.value())
		parentGroup2Filter = int(self.ui.spinGroupParentFilter2.value())
		if not self.ui.chkGroupEnableParentSeqFilter.isChecked():
			parentSeqFilter = None
			parentGroup1Filter = None
			parentGroup2Filter = None

		# effect size filters
		if self.ui.chkGroupEnableEffectSizeFilter1.isChecked():
			effectSizeMeasure1 = self.groupEffectSizeDict[str(self.ui.cboGroupEffectSizeMeasure1.currentText())]
			minEffectSize1 = float(self.ui.spinGroupMinEffectSize1.value())
		else:
			effectSizeMeasure1 = None
			minEffectSize1 = None

		if self.ui.chkGroupEnableEffectSizeFilter2.isChecked():
			effectSizeMeasure2 = self.groupEffectSizeDict[str(self.ui.cboGroupEffectSizeMeasure2.currentText())]
			minEffectSize2 = float(self.ui.spinGroupMinEffectSize2.value())
		else:
			effectSizeMeasure2 = None
			minEffectSize2 = None

		if self.ui.radioGroupOR.isChecked():
			effectSizeOperator = 'OR'
		else:
			effectSizeOperator = 'AND'

		self.groupStatsTest.results.filterFeatures(signLevelFilter, seqFilter, group1Filter, group2Filter,
																							parentSeqFilter, parentGroup1Filter, parentGroup2Filter,
																							effectSizeMeasure1, minEffectSize1, effectSizeOperator,
																							effectSizeMeasure2, minEffectSize2)

		# update table summarizing statistical results
		self.groupUpdateFilterInfo()
		self.groupFeaturesTableUpdate()
		self.groupTable.updateTable(self.groupStatsTest)

		# update plots
		self.groupPlotUpdate()

		QtGui.QApplication.instance().restoreOverrideCursor()

	def multiGroupApplyFilters(self):
		if self.metadata == None or self.bAutoRecalculate == False:
			return

		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		if not self.ui.chkMultiGroupSelectFeatures.isChecked():
			self.multiGroupStatsTest.results.selectAllFeautres()

		# perform filtering
		signLevelFilter = self.ui.spinMultiGroupSignLevelFilter.value()
		if not self.ui.chkMultiGroupEnableSignLevelFilter.isChecked():
			signLevelFilter = None

		if self.ui.chkMultiGroupEnableEffectSizeFilter.isChecked():
			effectSizeMeasure = self.multiGroupEffectSizeDict[str(self.ui.cboMultiGroupEffectSizeMeasure.currentText())]
			minEffectSize = float(self.ui.spinMultiGroupMinEffectSize.value())
		else:
			effectSizeMeasure = None
			minEffectSize = None

		self.multiGroupStatsTest.results.filterFeatures(signLevelFilter, effectSizeMeasure, minEffectSize)

		# update table summarizing statistical results
		self.multiGroupUpdateFilterInfo()
		self.multiGroupFeaturesTableUpdate()
		self.multiGroupTable.updateTable(self.multiGroupStatsTest)

		# update plots
		self.multiGroupPlotUpdate()

		QtGui.QApplication.instance().restoreOverrideCursor()

	def sampleUpdateFilterInfo(self):
		self.ui.txtSampleNumActiveFeatures.setText(str(len(self.sampleStatsTest.results.getActiveFeatures())))

	def groupUpdateFilterInfo(self):
		self.ui.txtGroupNumActiveFeatures.setText(str(len(self.groupStatsTest.results.getActiveFeatures())))

	def multiGroupUpdateFilterInfo(self):
		self.ui.txtMultiGroupNumActiveFeatures.setText(str(len(self.multiGroupStatsTest.results.getActiveFeatures())))

	def sampleHighlightHierarchyChanged(self):
		index = self.ui.cboSampleHighlightHierarchy.currentIndex() - 1
		if index == -1:
			self.preferences['Highlighted sample features'] = []
			self.ui.cboSampleHighlightFeature.clear()
			self.samplePlotUpdate()
			return

		features = set([])
		for feature in self.sampleProfile.profileDict.keys():
			hierarchy = self.sampleProfile.getHierarchy(feature)
			features.add(hierarchy[index])

		features = list(features)
		features.sort(key=string.lower)

		featureStrList = QtCore.QStringList()
		for feature in features:
			featureStrList.append(feature)

		self.ui.cboSampleHighlightFeature.clear()
		self.ui.cboSampleHighlightFeature.insertItems(len(featureStrList), featureStrList)
		self.ui.cboSampleHighlightFeature.setCurrentIndex(0)

		self.ui.cboSampleHighlightFeature.adjustSize()

		self.sampleHighlightFeatureChanged()

	def groupHighlightHierarchyChanged(self):
		index = self.ui.cboGroupHighlightHierarchy.currentIndex() - 1
		if index == -1:
			self.preferences['Highlighted group features'] = []
			self.ui.cboGroupHighlightFeature.clear()
			self.groupPlotUpdate()
			return

		features = set([])
		for feature in self.groupProfile.profileDict.keys():
			hierarchy = self.groupProfile.getHierarchy(feature)
			features.add(hierarchy[index])

		features = list(features)
		features.sort(key=string.lower)

		featureStrList = QtCore.QStringList()
		for feature in features:
			featureStrList.append(feature)

		self.ui.cboGroupHighlightFeature.clear()
		self.ui.cboGroupHighlightFeature.insertItems(len(featureStrList), featureStrList)
		self.ui.cboGroupHighlightFeature.setCurrentIndex(0)

		self.ui.cboGroupHighlightFeature.adjustSize()

		self.groupHighlightFeatureChanged()

	def multiGroupHighlightHierarchyChanged(self):
		index = self.ui.cboMultiGroupHighlightHierarchy.currentIndex() - 1
		if index == -1:
			self.preferences['Highlighted multiple group features'] = []
			self.ui.cboMultiGroupHighlightFeature.clear()
			self.multiGroupPlotUpdate()
			return

		features = set([])
		for feature in self.multiGroupProfile.profileDict.keys():
			hierarchy = self.multiGroupProfile.getHierarchy(feature)
			features.add(hierarchy[index])

		features = list(features)
		features.sort(key=string.lower)

		featureStrList = QtCore.QStringList()
		for feature in features:
			featureStrList.append(feature)

		self.ui.cboMultiGroupHighlightFeature.clear()
		self.ui.cboMultiGroupHighlightFeature.insertItems(len(featureStrList), featureStrList)
		self.ui.cboMultiGroupHighlightFeature.setCurrentIndex(0)

		self.ui.cboMultiGroupHighlightFeature.adjustSize()

		self.multiGroupHighlightFeatureChanged()

	def sampleHighlightFeatureChanged(self):
		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		index = self.ui.cboSampleHighlightHierarchy.currentIndex() - 1
		selectedFeature = self.ui.cboSampleHighlightFeature.currentText()

		self.preferences['Highlighted sample features'] = []
		for feature in self.sampleProfile.profileDict.keys():
			hierarchy = self.sampleProfile.getHierarchy(feature)
			if hierarchy[index] == selectedFeature:
				self.preferences['Highlighted sample features'].append(feature)

		self.samplePlotUpdate()

		QtGui.QApplication.instance().restoreOverrideCursor()

	def groupHighlightFeatureChanged(self):
		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		index = self.ui.cboGroupHighlightHierarchy.currentIndex() - 1
		selectedFeature = self.ui.cboGroupHighlightFeature.currentText()

		self.preferences['Highlighted group features'] = []
		for feature in self.groupProfile.profileDict.keys():
			hierarchy = self.groupProfile.getHierarchy(feature)
			if hierarchy[index] == selectedFeature:
				self.preferences['Highlighted group features'].append(feature)

		self.groupPlotUpdate()

		QtGui.QApplication.instance().restoreOverrideCursor()

	def multiGroupHighlightFeatureChanged(self):
		QtGui.QApplication.instance().setOverrideCursor(QtGui.QCursor(QtCore.Qt.WaitCursor))

		index = self.ui.cboMultiGroupHighlightHierarchy.currentIndex() - 1
		selectedFeature = self.ui.cboMultiGroupHighlightFeature.currentText()

		self.preferences['Highlighted multiple group features'] = []
		for feature in self.multiGroupProfile.profileDict.keys():
			hierarchy = self.multiGroupProfile.getHierarchy(feature)
			if hierarchy[index] == selectedFeature:
				self.preferences['Highlighted multiple group features'].append(feature)

		self.multiGroupPlotUpdate()

		QtGui.QApplication.instance().restoreOverrideCursor()

	def saveImageDlg(self):
		stackedWidget = self.ui.stackedWidgetViews
		if stackedWidget.currentIndex() == 2:
			plotToSave = self.samplePlot
		elif stackedWidget.currentIndex() == 1:
			plotToSave = self.groupPlot
		elif stackedWidget.currentIndex() == 0:
			plotToSave = self.multiGroupPlot
		else:
			QtGui.QMessageBox.information(self, 'Select plot', 'A plot tab must be active to save a plot.', QtGui.QMessageBox.Ok)
			return

		f = QtGui.QFileDialog.getSaveFileName(self, 'Save plot...', self.preferences['Last directory'],
								'Portable Network Graphics (*.png);;' +
								'Portable Document Format (*.pdf);;' +
								'PostScript (*.ps);;' +
								'Encapsulated PostScript (*.eps);;' +
								'Scalable Vector Graphics (*.svg)')

		if f != '':
			self.preferences['Last directory'] = f[0:f.lastIndexOf('/')]
			try:
				if f[len(f) - 3:len(f)] == 'png' or f[len(f) - 3:len(f)] == 'PNG':
					dpi, ok = QtGui.QInputDialog.getInteger(self, 'Desired resolution', 'Enter desired resolution (DPI) of image:', 300)
					if ok:
						plotToSave.save(str(f), dpi)
				else:
					plotToSave.save(str(f))
			except IOError:
					QtGui.QMessageBox.information(self, 'Failed to save image', 'Write permission for file denied.', QtGui.QMessageBox.Ok)

	def sendPlotToWindow(self):
		if self.ui.stackedWidgetViews.currentIndex() == 2:
			self.samplePlot.sendToNewWindow(self, self.sampleProfile, self.sampleStatsTest.results)
		elif self.ui.stackedWidgetViews.currentIndex() == 1:
			self.groupPlot.sendToNewWindow(self, self.groupProfile, self.groupStatsTest.results)
		elif self.ui.stackedWidgetViews.currentIndex() == 0:
			self.multiGroupPlot.sendToNewWindow(self, self.multiGroupProfile, self.multiGroupStatsTest.results)

	def updateStatusBar(self):
		if self.ui.tabWidgetProperties.tabText(self.ui.tabWidgetProperties.currentIndex()) == 'Two samples':
			sampleName1 = str(self.ui.cboSample1.currentText())
			sampleName2 = str(self.ui.cboSample2.currentText())

			message = 'Parent categories: ' + str(self.sampleProfile.getNumParentCategories());
			message += ', Features: ' + str(self.sampleProfile.getNumFeatures())
			message += ', Sequences in sample 1: ' + str(self.profileTree.numSequencesInSample(sampleName1))
			message += ', Sequences in sample 2: ' + str(self.profileTree.numSequencesInSample(sampleName2))

		elif self.ui.tabWidgetProperties.tabText(self.ui.tabWidgetProperties.currentIndex()) == 'Two groups':
			groupName1 = str(self.ui.cboGroup1.currentText())
			groupName2 = str(self.ui.cboGroup2.currentText())

			message = 'Parent categories: ' + str(self.groupProfile.getNumParentCategories());
			message += ', Features: ' + str(self.groupProfile.getNumFeatures())
			message += ', Sequences in group 1: ' + str(self.profileTree.numSequencesInGroup(groupName1, self.metadata))

			if groupName2 != '<All other samples>':
				message += ', Sequences in group 2: ' + str(self.profileTree.numSequencesInGroup(groupName2, self.metadata))
			else:
				message += ', Sequences in group2: ' + str(self.profileTree.numSequences(self.metadata) - self.profileTree.numSequencesInGroup(groupName1, self.metadata))
		else:
			message = 'Parent categories: ' + str(self.multiGroupProfile.getNumParentCategories());
			message += ', Features: ' + str(self.multiGroupProfile.getNumFeatures())

		self.lblStatusBar.setText(message)

	def openAboutDlg(self):
		QtGui.QMessageBox.about(self, 'About...',
				'STAMP: statistical analysis of taxonomic and functional profiles\n\n'
				'%s\n'
				'%s\n'
				'%s\n\n'
				'%s' % ('Donovan Parks and Robert Beiko', __version__, __date__, 'Program icon by Caihua (http://commons.wikimedia.org/wiki/File:Fairytale_colors.png)'))

	def closeEvent(self, event):
		# save size and location of main window and all dock widgets
		settings = QtCore.QSettings("BeikoLab", "STAMP")
		settings.setValue("MainWindow/State", self.saveState())
		settings.setValue("MainWindow/Geometry", self.saveGeometry())

		# save preferences
		settings.setValue('Preferences/Pseudocount', self.preferences['Pseudocount'])
		settings.setValue('Preferences/Replicates', self.preferences['Replicates'])
		settings.setValue('Preferences/Truncate feature names', self.preferences['Truncate feature names'])
		settings.setValue('Preferences/Length of truncated feature names', self.preferences['Length of truncated feature names'])
		settings.setValue('Preferences/Axes colour', self.preferences['Axes colour'].name())
		settings.setValue('Preferences/All other samples colour', self.preferences['All other samples colour'].name())
		settings.setValue('Preferences/Minimum reported p-value exponent', self.preferences['Minimum reported p-value exponent'])

def exceptHook(exc_type, exc_value, exc_traceback):
	# # Copyright (c) 2002-2007 Pascal Varet <p.varet@gmail.com>
	# #
	# # Originally part of Spyrit.

	import traceback

	# # KeyboardInterrupt is a special case.
	# # We don't raise the error dialog when it occurs.
	if issubclass(exc_type, KeyboardInterrupt):
		if qApp():
			qApp().quit()
		return

	filename, line, dummy, dummy = traceback.extract_tb(exc_traceback).pop()
	filename = os.path.basename(filename)
	error = "%s: %s" % (exc_type.__name__, exc_value)

	QtGui.QMessageBox.critical(None, "Unknown error...",
		"<center>An error has occured:<br/><br/>"
	+ "<b><i>%s</i></b><br/>" % error
	+ "It occured at <b>line %d</b> of file <b>%s</b>.<br/>" % (line, filename)
	+ "</center>")

def main():
	# ignore numpy warnings as invalid results are handled within STAMP
	seterr(all='ignore')

	# add main directory to the path
	sys.path.insert(0, getMainDir())
	os.chdir(getMainDir())

	# initialize preferences
	settings = QtCore.QSettings("BeikoLab", "STAMP")
	preferences = {}

	preferences['Pseudocount'] = settings.value('Preferences/Pseudocount', 0.5).toDouble()[0]
	preferences['Replicates'] = settings.value('Preferences/Replicates', 1000).toInt()[0]
	preferences['Truncate feature names'] = settings.value('Preferences/Truncate feature names', True).toBool()
	preferences['Length of truncated feature names'] = settings.value('Preferences/Length of truncated feature names', 50).toInt()[0]
	preferences['Axes colour'] = QtGui.QColor(settings.value('Preferences/Axes colour', '#7f7f7f'))
	preferences['All other samples colour'] = QtGui.QColor(settings.value('Preferences/All other samples colour', '#7f7f7f'))
	preferences['Minimum reported p-value exponent'] = settings.value('Preferences/Minimum reported p-value exponent', -15).toDouble()[0]

	preferences['Sample 1 colour'] = QtGui.QColor(128, 177, 211)
	preferences['Sample 2 colour'] = QtGui.QColor(253, 180, 98)
	preferences['Group colours'] = {}
	preferences['Highlighted sample features'] = []
	preferences['Highlighted group features'] = []
	preferences['Highlighted multiple group features'] = []
	preferences['Selected group feature'] = ''
	preferences['Selected multiple group feature'] = ''

	# set the current working directory
	workingDir = getMainDir()
	preferences['Last directory'] = ''
	if 'STAMP.app/Contents/Resources' in workingDir:
		workingDir = workingDir[0:len(workingDir) - len('/STAMP.app/Contents/Resources')]
		preferences['Last directory'] = workingDir
		os.chdir(workingDir)

	if platform.system() == 'Windows' and platform.release() == 'post2008Server':
		# We are running under windows, so indicate that this is a new app
		# and should not be considered as running under Python. This ensures
		# the program icon is displayed in the taskbar instead of the generic
		# Python icon.
		import ctypes
		myappid = 'beikolab.software.stamp.2'  # arbitrary string
		ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(myappid)

	if (platform.system() == 'Windows' and len(sys.argv) == 1) or (platform.system() != 'Windows' and len(sys.argv) <= 2):
		sys.excepthook = exceptHook
		app = QtGui.QApplication(sys.argv)

		if(False):  # profile code
			import cProfile
			cProfile.run('mainWindow = MainWindow(preferences)', 'profile.txt')
			##########################################
			##########################################
			# Use this in python console!
			# import pstats
			# p = pstats.Stats('profile.txt')
			# p.sort_stats('cumulative').print_stats(10)
			# p.sort_stats('time').print_stats(10)
			##########################################
			##########################################
		else:
			mainWindow = MainWindow(preferences)

		mainWindow.show()
		sys.exit(app.exec_())
	else:
		print 'Failed to start STAMP.'
		sys.exit()

if __name__ == "__main__":
	main()
