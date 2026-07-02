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

import sys
import os.path

def runningExecutable():
	# Detect a bundled/frozen build (py2exe, py2app, PyInstaller, cx_Freeze).
	# The old `imp.is_frozen("__main__")` branch (stdlib `freeze` tool) is dropped:
	# the `imp` module was removed in Python 3.12 and STAMP never used that tool.
	return (hasattr(sys, "frozen") or # new py2exe / py2app / PyInstaller
					 hasattr(sys, "importers")) # old py2exe
	
def getMainDir():
	if runningExecutable():	
		if 'MacOS' in sys.executable: # on a OS X box
			appResourceDir = sys.executable[0:sys.executable.rfind('/')]
			appResourceDir = appResourceDir[0:appResourceDir.rfind('/')] + '/Resources/'
			return appResourceDir
			
		# on a Windows box
		return os.path.dirname(sys.executable)

	# Non-frozen: return the directory that contains the `stamp` package,
	# independent of how the app was launched (e.g. `python -m stamp`).
	return os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
