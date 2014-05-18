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

def isNumber(s):
	'''
	Check if a string represents a number.
	'''
	try:
		if isinstance(s, str):
			# strip any equality signs
			if s[0] == '<' or s[0] == '>':
				s = s[1:]
			if s[0] == '=':
				s = s[1:]
		
		float(s)
		return True
	except ValueError:
		return False
		
def isStrictNumber(s):
	'''
	Check if a string represents a number.
	'''
	try:
		float(s)
		return True
	except ValueError:
		return False

