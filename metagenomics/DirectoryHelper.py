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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.	See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with STAMP.	If not, see <http://www.gnu.org/licenses/>.
#=======================================================================

import imp, sys
import os.path

def runningExecutable():
	return (hasattr(sys, "frozen") or # new py2exe
					 hasattr(sys, "importers") # old py2exe
					 or imp.is_frozen("__main__")) # tools/freeze
	
def getMainDir():
	if runningExecutable():	
		if 'MacOS' in sys.executable: # on a OS X box
			appResourceDir = sys.executable[0:sys.executable.rfind('/')]
			appResourceDir = appResourceDir[0:appResourceDir.rfind('/')] + '/Resources/'
			return appResourceDir
			
		# on a Windows box
		return os.path.dirname(sys.executable)

	return sys.path[0]
