#!/bin/bash
# How the PICRUSt2 outputs in PICRUSt2/ were made (2026-11-26, on the UQ ACE
# server "page"). Kept as a record; $W is a working folder.
#
# PICRUSt2 2.6.4 from bioconda, in a pixi environment (pixi.toml: channels
# conda-forge and bioconda, dependency picrust2). The package's post-link
# script, which pixi does not run, downloads PICRUSt2's source release
# (https://github.com/picrust/picrust2/archive/v2.6.4.tar.gz, SHA-256
# b982acc67114027951d3a7fec8eee9619ee2af3c15051a5cdf4fee5ee62af697) and
# moves its default_files (the reference trees and trait tables) into the
# installed package; it was run by hand:
#
#   PREFIX=$W/.pixi/envs/default bash $W/.pixi/envs/default/bin/.picrust2-post-link.sh
#
# Data: the 16S tutorial data of PICRUSt2's wiki,
# https://kronos.pharmacology.dal.ca/public_files/picrust/picrust2_tutorial_files/chemerin_16S.zip
# (seqs.fna: 37 ASVs; table.biom: 24 samples; metadata.tsv).
#
# The outputs were recompressed with `gzip -9 -n` for the repository.
set -euo pipefail
export PATH=$HOME/.pixi/bin:$PATH
D=$W/chemerin_16S
P="pixi run --manifest-path $W/pixi.toml"
cd $W

# Functions and pathways, with pathway coverage.
$P picrust2_pipeline.py -s $D/seqs.fna -i $D/table.biom -o out_default -p 16 --coverage
# Contributions as wide tables, and pathways per sequence.
$P picrust2_pipeline.py -s $D/seqs.fna -i $D/table.biom -o out_wide -p 16 \
  --stratified --wide_table --per_sequence_contrib
# Descriptions.
for m in EC KO; do
  $P add_descriptions.py -i out_default/${m}_metagenome_out/pred_metagenome_unstrat.tsv.gz -m $m \
    -o out_default/${m}_metagenome_out/pred_metagenome_unstrat_descrip.tsv.gz
done
$P add_descriptions.py -i out_default/pathways_out/path_abun_unstrat.tsv.gz -m METACYC \
  -o out_default/pathways_out/path_abun_unstrat_descrip.tsv.gz

# The long contribution tables (--stratified alone) for four samples.
printf "100CHE6KO\n101CHE6WT\n20CMK6KO\n21CMK6WT\n" > four.txt
$P biom subset-table -i $D/table.biom -a sample -s four.txt -o table_4.biom
$P picrust2_pipeline.py -s $D/seqs.fna -i table_4.biom -o out_strat4 -p 16 --stratified

# chemerin/: out_wide's EC_metagenome_out, KO_metagenome_out and
# pathways_out (but path_abun_contrib), out_default's path_cov_unstrat, the
# three _descrip tables in descriptions/, and the tutorial's metadata.tsv.
# chemerin_4_samples/: out_strat4's EC pred_metagenome_contrib and
# pred_metagenome_unstrat, path_abun_contrib and path_abun_unstrat (the
# KO tables are left out: 1.1 MB, and the EC ones show the same layout).
