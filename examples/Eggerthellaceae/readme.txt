Eggerthellaceae gut symbionts of mammals (STAMP v3 example data)
================================================================

Gut metagenomes from the Sequence Read Archive (SRA) in which the
Eggerthellaceae genera Equivita, Gorillivita or Tapirivita were detected.
These genera are gut symbionts of mammals (mostly horses and their
relatives, great apes, elephants and tapirs) described in:

  Parks DH et al. (2026). Stop codon reassignment to tryptophan in members of
  the bacterial phylum Actinomycetota. Microbial Genomics, 12(8).
  doi:10.1099/mgen.0.001767

Equivita and Gorillivita use genetic code 4 (UGA read as tryptophan);
Tapirivita uses the standard bacterial code 11.


Files
-----

eggerthellaceae.spf
  STAMP profile: 1,289 metagenomes (one column per NCBI BioSample) x 544
  features, six levels Domain, Phylum, Class, Order, Family, Genus (GTDB
  R232 taxonomy). Values are SingleM estimates of genome coverage, rounded
  to three significant digits; they are NOT counts of reads. To keep the
  file small, it holds the 475 genera present at >= 0.1% relative abundance
  in >= 2% of samples, plus every genus of the three focus genera; the rest
  of the coverage of each phylum is in a row named
  "unclassified other <phylum>" (at Class to Genus), so each sample's total
  is the total SingleM coverage of the metagenome. Coverage that SingleM
  could only place at a higher rank is in "unclassified <rank>__<taxon>"
  rows, as `stamp-cli convert sandpiper` writes them.

eggerthellaceae.metadata.tsv
  One row per BioSample (first column):
    BioProject          NCBI BioProject accession
    Host_group          13 curated groups (Domestic horse, Feral horse
                        (Sable Island), Przewalski's / E. ferus, Asiatic wild
                        asses, Donkey / African wild ass, Zebra, Great apes,
                        Elephants, Rhinoceros, Tapir, Other mammals,
                        Environmental, Unknown)
    Host_species        host scientific name
    Host_common_name    host common name
    Host_order          host order (Perissodactyla, Primates, Proboscidea,
                        Artiodactyla, Carnivora; Environmental or Unknown)
    Captivity           Domestic, Feral, Wild, Captive or Unknown
    Country, Continent  where the sample was collected
    Latitude, Longitude decimal degrees, where the BioSample gives them
                        (916 BioSamples, 927 runs); empty otherwise
    Collection_year     year of collection, where given
    Metagenome_Gbp      size of the metagenome in gigabases
    Equivita_relab, Gorillivita_relab, Tapirivita_relab
                        relative abundance (%) of each focus genus in the
                        whole community (from the full genus profile; for
                        the 11 BioSamples with several runs, the mean of
                        the runs' percentages, which can differ from the
                        percentage of the averaged coverage in the profile)
    MGR_genera_present  which focus genera are at >= 0.1% (Equivita,
                        Gorillivita, Tapirivita, Equivita+Tapirivita, or
                        "none >= 0.1%")
    Runs                SRA runs of the BioSample, comma-separated (1,307
                        runs in all)

eggerthellaceae_mgr_species.spf
  STAMP profile with two levels, Genus and Species: the GlobDB species of
  Equivita, Gorillivita and Tapirivita (SingleM run against GlobDB, which
  resolves more species in these genera than GTDB), with the coverage
  SingleM placed only at genus level in "unclassified g__<genus>" rows, and
  one row, "unclassified rest of community", holding the coverage of every
  other organism. Open it with Unclassified set to "Use only for
  calculating frequency profiles" (`--unclassified frequency-only` on the
  command line), so that proportions are of the whole community but the
  rest of the community is not itself a feature. Uses the same metadata.


How it was made
---------------

1. Sandpiper 2.1.1 (https://sandpiper.qut.edu.au; SingleM profiles of public
   metagenomes, GTDB R232) was searched on 3 March 2026 for every SRA run in
   which SingleM detected Equivita, Gorillivita or Tapirivita. The condensed
   SingleM profile (GTDB and GlobDB) and the SRA/BioSample metadata of each
   of these runs were downloaded.
2. The profiles were converted to STAMP profiles (as
   `stamp-cli convert sandpiper` does) and the runs of one BioSample were
   averaged into one sample, named by the BioSample accession.
3. Host species, captivity and location were curated from the NCBI BioSample
   records by ordered regular expressions over the host, organism and
   description fields, with explicit overrides for samples that are not from
   an animal gut (compost, wastewater, a co-culture, permafrost) and for
   projects whose BioSamples are silent (for example zoo studies) or whose
   coordinates have latitude and longitude swapped.
4. The genus profile was reduced to the genera listed above and the
   coverage values rounded to three significant digits.

The scripts (01_fetch_sandpiper.py, 02_build_profiles.py,
03_curate_metadata.py, 04_prepare_analysis.py) are kept with the analysis
by Donovan Parks.


Caveats
-------

* Values are estimated genome coverage, not counts. Rarefaction curves,
  Chao1, count-based tests of two samples (Fisher's exact test, G-test,
  chi-square) and rarefying before diversity are not meaningful, and STAMP
  refuses the ones that need whole numbers. Proportions (relative
  abundances) and the tests on them across groups of samples are fine.
* The dataset is dominated by one study: 682 of the 1,289 BioSamples (688
  runs) are feral horses of Sable Island, Canada (654 of them in BioProject
  PRJNA1102860). Results over all samples mostly describe these horses; use
  a sample filter, or groups, to look at the other hosts.
* Host and location curation is rule-based. It was checked by eye, but
  individual BioSamples may be wrong, and some groups are small (Tapir 2,
  Rhinoceros 4, Zebra 5 samples).
* Samples were chosen because they contain at least one focus genus, so the
  data say nothing about how often hosts lack these genera.
* Latitude and longitude are as submitted to NCBI (a few with latitude and
  longitude swapped were corrected) and are missing for 373 of the 1,289
  BioSamples.


Licence and reuse
-----------------

The sequence data and BioSample metadata are public records of the
NCBI Sequence Read Archive and BioSample databases. The profiles and curated
metadata in this directory may be reused freely; please cite:

* Parks DH et al. (2026), Microbial Genomics 12(8),
  doi:10.1099/mgen.0.001767, for the genera and this dataset;
* Sandpiper (https://sandpiper.qut.edu.au) and SingleM (Woodcroft BJ et al.
  2025, Comprehensive taxonomic identification of microbial species in
  metagenomic data using SingleM and Sandpiper, Nature Biotechnology) for
  the profiles;
* the original studies of the BioProjects you use.
