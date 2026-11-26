#!/bin/bash
# How the real profiler outputs in */stool_SRR17283923/ were made
# (2026-11-26, on the UQ ACE server "page"). Kept as a record: the paths are
# those of that machine ($W held the pixi environments and databases, $ST the
# reads and outputs).
#
# The run: SRR17283923 (BioProject PRJNA787665, "Human gut microbiota survey
# of the Hong Kong population"; sample SAMN23839011, stool, Illumina
# NovaSeq 6000, 2 x 151 bp, 4,611,505 read pairs), downloaded from ENA
# (MD5s d71065163b6077d0db6be5b21addf90e, 7a54ba8cc7546e83bec73c379c2ab301)
# and subsampled to 1,000,000 read pairs with
#
#   seqtk sample -2 -s 20261126 SRR17283923_1.fastq.gz 1000000 | gzip > SRR17283923_1M_1.fastq.gz
#   seqtk sample -2 -s 20261126 SRR17283923_2.fastq.gz 1000000 | gzip > SRR17283923_1M_2.fastq.gz
#
# (seqtk 1.4, both mates with the same seed, so the pairs stay together).
# The pixi environments used:
#
#   profilers: kraken2 2.17.2, bracken 3.1, humann 3.9, metaphlan 4.1.1,
#              python 3.12, setuptools <81
#   motus3:    motus 3.1.0 (database from `motus downloadDB`)
#   motus4:    motus 4.1.0 (database 4.1 from `motus downloadMGDB`)
#   MetaPhlAn database: $W/mpa_db holds links to mpa_vJun23_CHOCOPhlAnSGB_202403
#   SingleM:   singlem 0.21.4, metapackage S6.5.0.GTDB_r232
#
# Sandpiper's profile was downloaded from
# https://sandpiper.qut.edu.au/api/condensed/SRR17283923 (it is of the whole
# run). The outputs were then compressed for the repository with
#
#   gzip -9 -n <file>
#
# (-n: no name or time stamp in the header). The BAM, .mgc and inserts files
# of mOTUs and SingleM's OTU table are not kept.
#
# Usage: make_stool_outputs.sh STEP...  (singlem sandpiper motus3 motus4
# metaphlan humann kraken bracken), or no argument for all.
set -euo pipefail
export PATH=$HOME/.pixi/bin:$PATH
# MetaPhlAn 4.1.1 reads the first line its read_fastx.py writes to stderr as
# the read count; under Python 3.12 SyntaxWarnings would come first.
export PYTHONWARNINGS=ignore
W=$HOME/stamp_profilers   # pixi environments and databases
ST=$HOME/stamp_stool       # reads and outputs
RUN=SRR17283923
R1=$ST/reads/${RUN}_1M_1.fastq.gz
R2=$ST/reads/${RUN}_1M_2.fastq.gz
OUT=$ST/out
T=16
SINGLEM=/srv/projects/donovan/stamp/glacier-fed/tools   # pixi env with singlem 0.21.4 and its metapackage
KRAKEN_DB=/srv/db/kraken2/2.1.6/k2_pluspf_08_GB_20250714
MPA_DB=$W/mpa_db
MPA_INDEX=mpa_vJun23_CHOCOPhlAnSGB_202403
HUMANN_DB=/srv/db/humann/230612
P="pixi run --manifest-path $W/profilers/pixi.toml"
mkdir -p "$OUT"/{SingleM,Sandpiper,mOTUs3,mOTUs4,MetaPhlAn,HUMAnN,Kraken2,Bracken,logs}
steps=${*:-"singlem sandpiper motus3 motus4 metaphlan humann kraken bracken"}

for step in $steps; do
  echo "$(date '+%F %T') $step"
  case $step in
  singlem)
    S="pixi run --manifest-path $SINGLEM/pixi.toml singlem"
    PKG=$(ls -d $SINGLEM/metapackage/*.smpkg.zb | head -1)
    # SingleM names the sample after the forward-read file, so it reads
    # links named after the run: reads/singlem/$RUN{,_2}.fastq.gz.
    $S pipe -1 "$ST/reads/singlem/$RUN.fastq.gz" -2 "$ST/reads/singlem/${RUN}_2.fastq.gz" --metapackage "$PKG" --threads $T \
      -p "$OUT/SingleM/$RUN.profile.tsv" \
      --otu-table "$OUT/SingleM/$RUN.otu_table.tsv" \
      > "$OUT/logs/singlem.log" 2>&1
    $S summarise --input-taxonomic-profiles "$OUT/SingleM/$RUN.profile.tsv" \
      --output-taxonomic-profile-with-extras "$OUT/SingleM/$RUN.profile_with_extras.tsv" \
      >> "$OUT/logs/singlem.log" 2>&1
    ;;
  sandpiper)
    # Sandpiper is a web database: its profile is of the whole run.
    curl -s -o "$OUT/Sandpiper/$RUN.json" "https://sandpiper.qut.edu.au/api/condensed/$RUN"
    ;;
  motus3)
    # 100,000 read pairs are too few for mOTUs' default of 3 marker genes
    # per mOTU (nothing is found), so -g 1 ("higher recall") is used.
    M="pixi run --manifest-path $W/motus3/pixi.toml motus"
    rm -f "$OUT"/mOTUs3/*
    $M profile -f "$R1" -r "$R2" -n $RUN -t $T -I "$OUT/mOTUs3/$RUN.bam"       -o "$OUT/mOTUs3/$RUN.motus" > "$OUT/logs/motus3.log" 2>&1
    $M profile -i "$OUT/mOTUs3/$RUN.bam" -n $RUN -t $T -c -q       -o "$OUT/mOTUs3/${RUN}_counts_taxonomy.motus" >> "$OUT/logs/motus3.log" 2>&1
    $M profile -i "$OUT/mOTUs3/$RUN.bam" -n $RUN -t $T -k genus       -o "$OUT/mOTUs3/${RUN}_genus.motus" >> "$OUT/logs/motus3.log" 2>&1
    $M profile -i "$OUT/mOTUs3/$RUN.bam" -n $RUN -t $T -A       -o "$OUT/mOTUs3/${RUN}_all_ranks.motus" >> "$OUT/logs/motus3.log" 2>&1
    rm -f "$OUT/mOTUs3/$RUN.bam"
    ;;
  motus4)
    M="pixi run --manifest-path $W/motus4/pixi.toml motus"
    rm -f "$OUT"/mOTUs4/*
    $M profile -f "$R1" -r "$R2" -n $RUN -t $T -db "$W/motus4/db"       -o "$OUT/mOTUs4/$RUN.motus" > "$OUT/logs/motus4.log" 2>&1
    ;;
  metaphlan)
    rm -f "$OUT/MetaPhlAn/$RUN.bowtie2.bz2"
    $P metaphlan "$R1,$R2" --input_type fastq --bowtie2db "$MPA_DB" --index $MPA_INDEX \
      --nproc $T --offline --unclassified_estimation --bowtie2out "$OUT/MetaPhlAn/$RUN.bowtie2.bz2" \
      -o "$OUT/MetaPhlAn/${RUN}_profile.txt" > "$OUT/logs/metaphlan.log" 2>&1
    $P metaphlan "$OUT/MetaPhlAn/$RUN.bowtie2.bz2" --input_type bowtie2out \
      --bowtie2db "$MPA_DB" --index $MPA_INDEX --nproc $T --unclassified_estimation \
      --offline -t rel_ab_w_read_stats -o "$OUT/MetaPhlAn/${RUN}_read_stats_profile.txt" \
      >> "$OUT/logs/metaphlan.log" 2>&1
    $P sgb_to_gtdb_profile.py -i "$OUT/MetaPhlAn/${RUN}_profile.txt" \
      -o "$OUT/MetaPhlAn/${RUN}_gtdb_profile.txt" -d "$MPA_DB/$MPA_INDEX.pkl" \
      >> "$OUT/logs/metaphlan.log" 2>&1 || echo "sgb_to_gtdb_profile.py failed" >&2
    ;;
  humann)
    zcat "$R1" "$R2" > "$ST/reads/${RUN}_1M.fastq"
    $P humann --input "$ST/reads/${RUN}_1M.fastq" --output "$ST/humann_work" \
      --output-basename $RUN --threads $T \
      --taxonomic-profile "$OUT/MetaPhlAn/${RUN}_profile.txt" \
      --nucleotide-database "$HUMANN_DB/chocophlan" \
      --protein-database "$HUMANN_DB/uniref" \
      --metaphlan-options "--bowtie2db $MPA_DB --index $MPA_INDEX" \
      > "$OUT/logs/humann.log" 2>&1
    cp "$ST/humann_work/${RUN}_genefamilies.tsv" "$ST/humann_work/${RUN}_pathabundance.tsv" \
      "$ST/humann_work/${RUN}_pathcoverage.tsv" "$OUT/HUMAnN/"
    ;;
  kraken)
    K="$P kraken2 --db $KRAKEN_DB --threads $T --paired --gzip-compressed"
    $K --report "$OUT/Kraken2/$RUN.k2report" --output /dev/null "$R1" "$R2" \
      > "$OUT/logs/kraken2.log" 2>&1
    $K --report "$OUT/Kraken2/${RUN}_minimizers.k2report" --report-minimizer-data \
      --output /dev/null "$R1" "$R2" >> "$OUT/logs/kraken2.log" 2>&1
    $K --report "$OUT/Kraken2/${RUN}_mpa.txt" --use-mpa-style \
      --output /dev/null "$R1" "$R2" >> "$OUT/logs/kraken2.log" 2>&1
    ;;
  bracken)
    $P bracken -d $KRAKEN_DB -i "$OUT/Kraken2/$RUN.k2report" -r 150 -l S \
      -o "$OUT/Bracken/$RUN.bracken" -w "$OUT/Bracken/${RUN}_bracken_species.kreport" \
      > "$OUT/logs/bracken.log" 2>&1
    $P bracken -d $KRAKEN_DB -i "$OUT/Kraken2/$RUN.k2report" -r 150 -l G \
      -o "$OUT/Bracken/${RUN}_genus.bracken" -w "$OUT/Bracken/${RUN}_bracken_genus.kreport" \
      >> "$OUT/logs/bracken.log" 2>&1
    ;;
  *) echo "unknown step $step" >&2; exit 1 ;;
  esac
done
echo "$(date '+%F %T') done"
