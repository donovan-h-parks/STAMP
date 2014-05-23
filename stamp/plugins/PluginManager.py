#=======================================================================
# Author: Donovan Parks
#
# Load plugins.
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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.	See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP. If not, see <http://www.gnu.org/licenses/>.
#=======================================================================

import os, platform

from stamp.metagenomics.DirectoryHelper import runningExecutable, getMainDir

class PluginManager:
	def __init__(self, preferences):
		self.preferences = preferences
		pass
		
	def loadPlugins(self, pluginFolder):
		os.chdir(getMainDir())
		
		pluginModulePath = pluginFolder.replace('/', '.')
		
		if runningExecutable():
			if platform.system() == 'Windows':
				# windows plugin folder
				pluginFolder = 'library/' + pluginFolder
			else:
				# os x plugin folder
				pluginFolder = './lib/python2.6/site-packages/' + pluginFolder
		else:
			pluginFolder = os.path.join(os.path.split(os.path.realpath(__file__))[0], '..', '..', pluginFolder)
		
		d = {}
		for filename in os.listdir(pluginFolder):
			if os.path.isdir(os.path.join (pluginFolder, filename)):
				continue

			extension = filename[filename.rfind('.')+1:len(filename)]
			if extension == 'py' and filename != '__init__.py':
				pluginModule = filename[0:filename.rfind('.')]
				theModule = __import__(pluginModulePath + pluginModule, fromlist='*')
				theClass = getattr(theModule, pluginModule)
				theObject = theClass(self.preferences)
				d[theObject.name] = theObject

		return d
	
	def populateComboBox(self, d, comboBox, defaultPlugin):
		keys = d.keys()
		keys.sort(lambda x,y: cmp(x.lower(), y.lower()))
		for key in keys:
			comboBox.addItem(key)
		comboBox.setCurrentIndex(comboBox.findText(defaultPlugin))
