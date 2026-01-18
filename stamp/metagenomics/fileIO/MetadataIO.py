#=======================================================================
# Author: Donovan Parks
#
# Create a feature profile for a pair of metagenomic samples.
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

import string

from stamp.metagenomics.Metadata import Metadata

class MetadataIO(object):
	def __init__(self, preferences):
		self.preferences = preferences

	import os

	def read(self, filename, profileTree):
		print("Reading metadata from:", filename)
		warningMessage = None

		# 1. Python 3: Remove 'U' (deprecated), use 'r' with encoding
		# Using a context manager (with) ensures the file closes even if an error occurs
		try:
			with open(filename, 'r', encoding='utf-8', errors='ignore') as fin:
				# 2. In Python 3, map() is an iterator. Convert to list
				# or use list comprehension for indexing like data[0]
				data = [line.strip() for line in fin.readlines()]
		except IOError:
			return None, "Could not open metadata file."

		if not data:
			return None, "Metadata file is empty."

		metadata = Metadata()

		# Get features from header row
		features = data[0].split('\t')

		# Create a set for fast lookup
		profileSamples = set(profileTree.sampleNames)
		missingInProfile = []

		try:
			for r in range(1, len(data)):
				line = data[r].strip()
				if line == '':
					continue

				values = line.split('\t')
				sampleName = values[0].strip()

				if sampleName in profileSamples:
					# Discard from set so we can track missing samples later
					profileSamples.discard(sampleName)

					featureDict = {}
					# 3. Ensure we don't go out of bounds if a row is shorter than header
					for v in range(1, min(len(values), len(features))):
						featureDict[features[v]] = values[v].strip()

					metadata.metadataDict[sampleName] = featureDict
					metadata.activeSamples.append(sampleName)
				else:
					missingInProfile.append(sampleName)

		except Exception as e:
			# In Python 3, 'r' is still in scope here
			warningMessage = f'Failed to parse line {r + 1}: {str(e)}'

		# 4. Consolidate warning messages
		warnings = []
		if missingInProfile:
			warnings.append('Unknown sample(s) in metadata: ' + ', '.join(missingInProfile))

		if profileSamples:
			warnings.append('Missing metadata for samples: ' + ', '.join(list(profileSamples)))

		if warnings:
			warningMessage = '\n\n'.join(warnings)

		return metadata, warningMessage
 
	