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
		
	def read(self, filename, profileTree):
		warningMessage = None
		
		fin = open(filename, 'U')
		data = map(string.strip, fin.readlines())
		fin.close()
		
		metadata = Metadata()
		
		features = data[0].split('\t')

		profileSamples = set(profileTree.sampleNames)
		
		try:
			missingInProfile = []
			for r in xrange(1, len(data)):
				if data[r].strip() == '':
					continue

				values = data[r].split('\t')
				sampleName = values[0].strip()
				if sampleName in profileSamples:
					profileSamples.remove(sampleName)
					
					featureDict = {}
					for v in xrange(1, len(values)):
						featureDict[features[v]] = values[v].strip()
						
					metadata.metadataDict[sampleName] = featureDict
					metadata.activeSamples.append(sampleName)
				else:
					missingInProfile.append(sampleName)
		except:
			warningMessage = 'Failed to parse line: ' + str(r+1)

		if len(missingInProfile) != 0:
			warningMessage = 'Unknown sample(s) specified in metadata file: ' + ', '.join(missingInProfile) + '\n\n'
				
		if len(profileSamples) != 0:
			missingSamples = []
			for sample in profileSamples:
				missingSamples.append(sample)
				
			warningMessage = 'Missing metadata for the following samples: ' + ', '.join(missingSamples)

		return metadata, warningMessage
 
	