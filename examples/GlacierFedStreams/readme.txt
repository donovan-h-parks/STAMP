Glacier-fed stream sediment metagenomes (STAMP v3 example data)
===============================================================

Shotgun metagenomes of benthic sediment from glacier-fed streams in nine
mountain ranges, collected by the Vanishing Glaciers project (EPFL) and
described in:

  Michoud G, Peter H, Busi SB, et al. (2025). Mapping the metagenomic
  diversity of the multi-kingdom glacier-fed stream microbiome. Nature
  Microbiology 10:217-230. doi:10.1038/s41564-024-01874-9

  Ezzat L, Peter H, Bourquin M, et al. (2025). Global diversity and
  biogeography of the glacier-fed stream microbiome. Nature.
  doi:10.1038/s41586-024-08313-z

Sequence data: NCBI BioProject PRJNA781406.

Each stream was sampled at two reaches: an upstream reach as close as
possible to the glacier snout (median 78 m from it) and a downstream reach
near the terminal moraine of the Little Ice Age (median 571 m). The two
samples of a stream are therefore a pair, and the data set suits the paired
tests of two groups as well as comparisons of many groups (the mountain
ranges) and correlations with the measurements made at each reach.


Files
-----

glacier_streams.spf
  STAMP profile: 158 metagenomes x 12,446 rows, seven levels Domain, Phylum,
  Class, Order, Family, Genus, Species (GTDB R232 taxonomy; 78 phyla, 1,392
  families and 5,015 genera with a name). Values are SingleM estimates of genome coverage;
  they are NOT counts of reads. Coverage that SingleM could only place at a
  higher rank is in "unclassified <rank>__<taxon>" rows, as
  `stamp-cli convert singlem` writes them. 156 rows are zero in every
  sample (taxa found only in samples left out, see below); they are kept so
  that analyses give the same results as those of the full data set.

glacier_streams.metadata.tsv
  One row per sample (first column):
    Glacier             glacier (GL<number>); the two reaches of a glacier's
                        stream have the same value, so this is the field
                        that pairs the samples
    Mountain_range      Caucasus (27 samples), Ecuadorian Andes (12),
                        European Alps (37), Greenland (7), Himalaya (19),
                        Rwenzori (2), Scandinavian Mountains (16), Southern
                        Alps (19), Tien Shan (19)
    Country             country of the BioSample record
    Reach               Upstream (77 samples) or Downstream (81)
    Glacier_name        name of the glacier, where recorded
    Latitude, Longitude decimal degrees
    Abs_latitude        absolute latitude
    Hemisphere          Northern, Southern or Tropics (within 23.5 degrees
                        of the equator)
    Collection_date     YYYY-MM-DD (2019 to 2021)
    Glacier_area_km2, Glacier_coverage, Glacier_surface_area_km2,
    Snout_elevation_m   of the glacier; Glacier_coverage is the glacierised
                        fraction of the catchment (0 to 1)
    Elevation_m, Distance_to_snout_m
                        of the sampling point
    Water_temp_C, pH, Conductivity_uS_cm, Turbidity_NTU,
    Dissolved_oxygen_mg_l
                        stream water
    Chlorophyll_a_ug_g, Bacterial_abundance_cells_g
                        sediment
    NO3_ug_l, NH4_ug_l, SRP_ug_l, DOC_ug_l, SO4_ug_l, DIN_ug_l
                        stream water chemistry (SRP: soluble reactive
                        phosphorus; DOC: dissolved organic carbon, sampled
                        at upstream reaches only; DIN: dissolved inorganic
                        nitrogen)
    In_Michoud_2025     Yes if the sample is in Supplementary Table 1 of
                        Michoud et al. (149 of the 158)
    Run, Run_label      SRA run that was profiled and its sample alias
    Original_run, Original_label
                        the earlier deposit of the same library (see below)
    Profile_source      Sandpiper (132 samples) or This study (26)
    Run_Gbp             size of the run, in Gbp
    Total_coverage      sum of the SingleM coverage of the sample
  Empty cells are measurements that were not made or not recorded.


How the data were made
----------------------

Each library was deposited twice in the BioProject: an original run and a
"_new" run holding the same reads after quality control (98.9 to 99.8% of
the reads of each "_new" run were found in its original). The "_new" runs,
which are the ones Michoud et al. analysed, were profiled with SingleM
against GTDB R232 (metapackage S6.5.0.GTDB_r232.metapackage_20260319). The
profiles of 132 samples were taken from Sandpiper
(https://sandpiper.qut.edu.au, version 2.1.1); Sandpiper has no profile for
the other 26, which were made with SingleM 0.21.4 (`singlem pipe` on the
.sra file). Three runs profiled both ways agree (Bray-Curtis dissimilarity
0.002).

Measurements are those of Supplementary Table 1 of Michoud et al. where it
lists the sample, otherwise those of the NCBI BioSample record of the
original run. The mountain range comes from the BioSample country.

The data set has one sample per reach. Left out: the second sediment sample
taken at four reaches (GL6 upstream, GL12 downstream, GL13 both), the
second and third replicates of the Rwenzori glacier GL140, and GL111_UP,
which was sequenced to only 0.12 Gbp. 73 glaciers have a sample of both
reaches; 12 have one (GL7, GL35, GL42, GL43, GL52, GL61, GL68, GL81, GL111,
GL126, GL127, GL137).

Notes on the public records, for anyone going back to them:
  - The "B" label of a second sediment sample is swapped between the two
    deposits (the run labelled GL13_Up_new holds the reads of GL13_UPB, and
    GL13_UpB_new those of GL13_UP; likewise at GL6 and GL12). Sample names
    here follow the original deposit, which is why six samples are named
    with a replicate number (GL6_UP_1, GL12_DN_1, GL13_UP_1, GL13_DN_1,
    GL140_UP_1, GL140_DN_1).
  - Supplementary Table 1 of Michoud et al. gives the wrong region for
    GL61_DN, GL88_DN, GL88_UP and GL126_DN (their coordinates are in
    Ecuador, Norway, Norway and Kyrgyzstan).


Things to keep in mind
----------------------

  - Coverage is not counts: proportions, and the tests and plots of groups
    of samples, are fine; rarefaction, Chao1 and the count-based tests of
    two samples are not meaningful.
  - Mountain range explains much more of the variation than reach does, so
    compare the reaches with a paired test (pair field: Glacier).
  - Correlations with the measurements pool all mountain ranges.
  - Rwenzori has one stream and Greenland four.


What to try
-----------

  # Families that differ between the reaches of a stream (37 at q <= 0.05)
  stamp-cli two-groups --profile glacier_streams.spf \
      --metadata glacier_streams.metadata.tsv --profile-level Family \
      --field Reach --group1 Downstream --group2 Upstream \
      --pair-field Glacier --test wilcoxon \
      --correction benjamini-hochberg --significance 0.05 \
      --output reach.tsv --plot reach.png

In the window, File > Open example data > Glacier-fed streams opens the
data with this comparison set up.

The documentation works through the data set: docs/site/
example_glacier_streams.rst.


Citing
------

Please cite Michoud et al. (2025) and Ezzat et al. (2025) for the samples
and measurements, and SingleM and Sandpiper (Woodcroft et al. 2025, Nature
Biotechnology) for the profiles.
