import random

numGroups = 3
numSamples = 100
numFeatures = 5000

# create metadata file
fout = open('metadata.tsv', 'w')
fout.write('Sample Id\tGroup\n')
for g in xrange(numGroups):
	for s in xrange(numSamples):
		fout.write('G%dS%d\t%d\n' % (g, s, g))
fout.close()

# create profiles
fout = open('profiles.tsv', 'w')
fout.write('Feature')
for g in xrange(numGroups):
	for s in xrange(numSamples):
		fout.write('\tG%dS%d' % (g, s))
fout.write('\n')

for f in xrange(numFeatures):
	fout.write('F%d' % f)
	for g in xrange(numGroups):
		rndGroup = random.randint(0, 100)
		for s in xrange(numSamples):
			fout.write('\t%d' % (rndGroup + random.randint(0, 10)))
	fout.write('\n')
		
fout.close()
