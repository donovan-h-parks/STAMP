fin = open('Enterotype.RITA.Arumugam.genus.tsv')
data = fin.readlines()
fin.close()

fout = open('Enterotype.RITA.Arumugam.genus.filtered.tsv', 'w')
fout.write(data[0])

for r in xrange(1, len(data)):
	lineSplit = data[r].split('\t')
	
	freqs = []
	for i in xrange(1, len(lineSplit)):
		freqs.append(float(lineSplit[i].strip()))
		
	meanFreq = sum(freqs) / len(freqs)
	
	if meanFreq > 0.0001:
		fout.write(data[r])

fout.close()