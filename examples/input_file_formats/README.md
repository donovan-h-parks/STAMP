# Example inputs in every format STAMP reads

The files directly in each directory are made up (below); the
`stool_SRR17283923/` folders hold real outputs of one sample (see "Real
outputs"), and `PICRUSt2/` holds real PICRUSt2 output for a 16S data set
(see "PICRUSt2"). Every file is gzip-compressed (`.gz`). STAMP reads compressed
files as they are, so they need not be decompressed first; `gunzip -k` gives
the plain text to look at.

**The made-up files are illustrative.** The numbers are made up. They were drawn at
random by `make_examples.py`, which writes them in the exact output format of
each tool. No tool was run to make them.

Every directory describes the same made-up community: 20 species of the human
gut in ten metagenomes, five from healthy people (`H01`–`H05`) and five from
people with IBD (`I01`–`I05`). Some species were made more common in IBD
(*Escherichia coli*, *Klebsiella pneumoniae*, *Enterococcus faecalis*,
*Mediterraneibacter gnavus*, *Veillonella parvula*) and others less common
(*Faecalibacterium prausnitzii*, *Roseburia intestinalis*, *Agathobacter
rectalis*, *Akkermansia muciniphila*, *Methanobrevibacter smithii*). A test of
two groups should therefore find these species. `metadata.tsv.gz` gives each
sample's group (field `Group`) and an age.

| Directory | Tool and output | Files |
|---|---|---|
| `SingleM/` | SingleM `pipe -p` condensed profiles (coverage, GTDB) | one per sample |
| `Sandpiper/` | Sandpiper condensed profiles (JSON) | one per sample |
| `mOTUs/` | mOTUs 3.1 `profile -c -q` (insert counts, full NCBI taxonomy) | one per sample |
| `mOTUs/merged/` | `motus merge` of relative-abundance `-q` profiles | one table |
| `MetaPhlAn/` | MetaPhlAn 4 `-t rel_ab` profiles (`mpa_vJan21` database, with SGBs and `UNCLASSIFIED`) | one per sample |
| `MetaPhlAn/merged/` | `merge_metaphlan_tables.py` | one table |
| `Kraken2/` | Kraken 2 `--report` (NCBI taxonomy, with an `S1` strain) | one per sample |
| `Bracken/` | Bracken species-level abundance tables (`.bracken`) | one per sample |
| `Bracken/reports/` | the reports Bracken writes with `-w` | one per sample |
| `HUMAnN/` | HUMAnN 3 gene families, pathway abundance and pathway coverage | three per sample |
| `BIOM/` | a QIIME-style OTU table of 16S amplicons, BIOM 1.0 (JSON) and 2.1 (HDF5) | one table each |
| `mothur/` | mothur `classify.seqs` taxonomy, groups and names files | three files |
| `IMG-M_COG/` | an IMG/M COG profile (to which `stamp-cli convert cog` adds COG categories) | one table |

## Real outputs: one human stool sample

Each tool's directory also has a `stool_SRR17283923/` folder. It holds
**real** output of that tool for one human stool metagenome: run
SRR17283923 of BioProject PRJNA787665 ("Human gut microbiota survey of the
Hong Kong population"; Illumina NovaSeq 6000, 2 x 151 bp, 4.6 million read
pairs). Before profiling, the run was subsampled at random to **1,000,000
read pairs** (seqtk, seed 20261126). `make_stool_outputs.sh` records every
command. STAMP needs two samples for an analysis, so these files are for
checking the readers (`crates/stamp-core/tests/stool_sample.rs`), not for
analysis on their own. Directory loading does not look into these folders,
so they don't change the made-up example sets.

| Tool (version, database) | Files | What they show |
|---|---|---|
| SingleM 0.21.4 (metapackage S6.5.0, GTDB R232) | `pipe -p` profile; the same with extras (`summarise`) | 88.35x coverage in all |
| Sandpiper | the run's condensed profile (JSON) | **whole run**, not the subsample: Sandpiper is a web database of public runs and cannot profile new reads |
| mOTUs 3.1.0 | the default (mOTUs), `-c -q` (counts, full taxonomy), `-A` (all ranks), `-k genus` | 120 mOTUs, 42 genera; the `-A` profile adds up to 0.80, as mOTUs leaves out what it could not assign |
| mOTUs 4.1.0 (database 4.1) | counts and `.relab` files (`motus4_SRR17283923.motus*`) | 97 mOTUs; the unassigned share as `mOTUv4.0_unassigned` |
| MetaPhlAn 4.1.1 (mpa_vJun23_CHOCOPhlAnSGB_202403) | `-t rel_ab` with `--unclassified_estimation`; `-t rel_ab_w_read_stats`; `sgb_to_gtdb_profile.py` output | 6.6% unclassified; *Segatella copri* (16.5%) and *Faecalibacterium prausnitzii* (16.1%) the largest species |
| HUMAnN 3.9 (ChocoPhlAn v201901_v31, UniRef90 201901b) | gene families, pathway abundance, pathway coverage | 78,424 gene families (95,354 species strata); 206 pathways with an abundance, 86 with a coverage above zero |
| Kraken 2.17.2 (PlusPF-8, 2025-07-14) | `--report`; with `--report-minimizer-data`; `--use-mpa-style` | 63.4% of reads classified; *Segatella copri* 17% |
| Bracken 3.1 (same database, 150-mers) | species and genus tables; the reports of both (`-w`) | 281 species, 631,323 reads in the species report |

Two things to know about these files:

- MetaPhlAn's `sgb_to_gtdb_profile.py` rescales the classified clades to
  100 but keeps the `UNCLASSIFIED` line, so its profile adds up to 106.6.
  STAMP reads the values as they are written.
- mOTUs 3 lists every mOTU of its database, zeros included, so its
  mOTU-level tables are large (2.3 and 6.8 MB) though mostly zeros; STAMP
  leaves out the rows at zero. Compressed, they take 235 and 435 kB.

## PICRUSt2: a 16S data set

PICRUSt2 predicts functions from 16S sequences, so it is not run on the
stool metagenome. `PICRUSt2/` holds **real** output of PICRUSt2 2.6.4 for
the 16S data of PICRUSt2's own tutorial (`chemerin_16S.zip`): 37 ASVs in
24 mouse stool samples, from mice lacking the protein chemerin (`KO`) and
wild-type mice (`WT`), kept in two facilities (Dalhousie and Palo Alto).
`make_picrust2_outputs.sh` records every command.

| Folder | What it holds |
|---|---|
| `chemerin/EC_metagenome_out/`, `KO_metagenome_out/` | `pred_metagenome_unstrat` (functions), `pred_metagenome_strat` (contributions of each ASV, `--stratified --wide_table`), and `seqtab_norm` and `weighted_nsti`, which are not profiles of functions |
| `chemerin/pathways_out/` | `path_abun_unstrat`, `path_cov_unstrat` (`--coverage`), and with `--per_sequence_contrib` `path_abun_strat`, `path_abun_unstrat_per_seq` and `path_abun_predictions` (not a profile) |
| `chemerin/descriptions/` | the EC, KO and pathway tables with `add_descriptions.py`'s descriptions |
| `chemerin/metadata.tsv.gz` | the tutorial's metadata: fields `Genotype` and `Facility` |
| `chemerin_4_samples/` | the long contribution tables PICRUSt2 writes by default with `--stratified` (`pred_metagenome_contrib`, `path_abun_contrib`), from a run on four of the samples (`100CHE6KO`, `101CHE6WT`, `20CMK6KO`, `21CMK6WT`): for all 24 they take 4 to 7 MB each, compressed |

The folders are laid out as PICRUSt2 writes them, so loading
`EC_metagenome_out` as a directory finds two kinds of table and asks which
to load. Unlike the other real outputs these can be analysed, for example
the pathways of knockout against wild-type mice:

```bash
stamp-cli two-groups \
    --profile examples/input_file_formats/PICRUSt2/chemerin/pathways_out/path_abun_unstrat.tsv.gz \
    --metadata examples/input_file_formats/PICRUSt2/chemerin/metadata.tsv.gz \
    --field Genotype --group1 KO --group2 WT
```

`crates/stamp-core/tests/picrust2_chemerin.rs` reads every table and checks
its values against PICRUSt2's, and that the contributions add up to the
unstratified tables.

## Loading them

- **Window:** use *File > Load profile directory* and choose one of these
  directories, or use *File > Load profile file* and choose the files of all
  samples together. Then load `metadata.tsv.gz` with *File > Load metadata*. The
  `HUMAnN` directory holds three kinds of table, so STAMP asks which kind to
  load.
- **Command line:**

  ```bash
  stamp-cli two-groups --profile examples/input_file_formats/Kraken2 \
      --metadata examples/input_file_formats/metadata.tsv.gz --field Group \
      --group1 Healthy --group2 IBD --profile-level Species
  stamp-cli info --profile examples/input_file_formats/HUMAnN \
      --profile-format humann-pathabundance
  ```

The BIOM table loads with the `taxonomy` observation metadata. The mothur
files load together: open the taxonomy file and STAMP asks for the groups
and names files.

## How they were made

```bash
pixi exec --spec python=3.12 --spec h5py --spec numpy python make_examples.py
```

The script uses a fixed seed and writes each file gzip-compressed with no
time stamp in it, so it always writes the same bytes. It needs only
Python's standard library; h5py is used only for the HDF5 BIOM table.
Before Python 3.12, sums of floating-point numbers were computed slightly
differently, which changes the last digit of a few HUMAnN values.

The formats follow each tool's own writer, as described in
`docs/PROFILER_FORMATS.md`. The made-up parts are:

- the abundances;
- some identifiers: mOTU ids, SGB numbers, UniRef90 accessions, OTU and read
  names (the NCBI taxids are real);
- the assignments of genes and pathways to species.

The taxonomy is real but simplified. MetaPhlAn and mOTUs use older NCBI
names (*Firmicutes*, *Bacteroidetes*). Kraken uses current ones
(*Bacillota*, *Bacteroidota*). SingleM and Sandpiper use GTDB release 220.

`crates/stamp-core/tests/profiler_examples.rs`, `crates/stamp-cli/tests/cli.rs`
and `crates/stamp-gui/tests/ui.rs` load these files. They check the levels,
the samples and the IBD signal.
