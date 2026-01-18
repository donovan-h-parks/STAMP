#!/srv/sw/python/2.7.4/bin/python

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

import sys

from PyQt5 import QtGui, QtCore
from PyQt5.QtWidgets import QApplication
# Import the file that contains your logic (STAMP.py)
from stamp import STAMP


def main():
    # 1. Create the application instance
    app = QApplication(sys.argv)

    # 1. Initialize the real Preferences dictionary
    # STAMP.py expects certain keys to exist right away (like colors)
    # 2. Initialize the preferences dictionary with default values
    # This prevents the KeyErrors you were seeing in STAMP.py
    preferences = {
        'Pseudocount': 1.0,
        'Replicates': 1000,
        'Truncate feature names': True,
        'Length of truncated feature names': 30,
        'Minimum reported p-value exponent': 15,
        'Axes colour': QtGui.QColor(0, 0, 0),
        'All other samples colour': QtGui.QColor(200, 200, 200),
        'Sample 1 colour': QtGui.QColor(0, 0, 255),  # Default Blue
        'Sample 2 colour': QtGui.QColor(255, 0, 0),  # Default Red
        'Group colours': {},
        'Highlighted sample features' : [],
        'Highlighted group features':[],
        'Highlighted multiple group features': [],
        'Selected group feature': '',
        'Last directory': '',
        'Selected multiple group feature': '' # Prevents your recent KeyError
    }


    # preferences = {}
    #
    # preferences['Pseudocount'] = settings.value('Preferences/Pseudocount', 0.5).toDouble()[0]
    # preferences['Replicates'] = settings.value('Preferences/Replicates', 1000).toInt()[0]
    # preferences['Truncate feature names'] = settings.value('Preferences/Truncate feature names', True).toBool()
    # preferences['Length of truncated feature names'] = \
    # settings.value('Preferences/Length of truncated feature names', 50).toInt()[0]
    # preferences['Axes colour'] = QtGui.QColor(settings.value('Preferences/Axes colour', '#7f7f7f'))
    # preferences['All other samples colour'] = QtGui.QColor(
    #     settings.value('Preferences/All other samples colour', '#7f7f7f'))
    # preferences['Minimum reported p-value exponent'] = \
    # settings.value('Preferences/Minimum reported p-value exponent', -15).toDouble()[0]
    #
    # preferences['Sample 1 colour'] = QtGui.QColor(128, 177, 211)
    # preferences['Sample 2 colour'] = QtGui.QColor(253, 180, 98)
    # preferences['Group colours'] = {}
    # preferences['Highlighted sample features'] = []
    # preferences['Highlighted group features'] = []
    # preferences['Highlighted multiple group features'] = []
    # preferences['Selected group feature'] = ''
    # preferences['Selected multiple group feature'] = ''

    # # set the current working directory
    # workingDir = getMainDir()
    # preferences['Last directory'] = ''

    # 2. Pass the real dictionary into the app
    gui = STAMP.StampApp(preferences=preferences)

    # 3. Start the event loop
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
