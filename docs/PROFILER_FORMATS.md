# Profiles written by taxonomic and functional profilers

Design note for the readers in `crates/stamp-core/src/import/` that load the
output of SingleM, Sandpiper, mOTUs, MetaPhlAn, Kraken 2, Bracken and HUMAnN
without conversion. It records what each tool writes, as found in the tools'
own source and documentation (November 2026; the research read mOTUs 2.6,
3.1 and 4.1, MetaPhlAn 3.0.14 to 4.2.6, Kraken 1 and 2, Bracken, KrakenTools,
SingleM at commit 08a4d1b, and HUMAnN 2.8.2, 3.0, 3.9 and 4.0.0a2), and how
STAMP turns it into a profile. All readers are reached through `import::profiler::ProfilerFormat`,
which recognises a file from its first 64 KiB, and they all build the profile
with `import::lineage::Lineages`.

## Common rules

* **Own values.** STAMP profiles hold each row's own value; parents are sums
  of their children. Tools that report *clade* (cumulative) values for every
  rank (MetaPhlAn, mOTUs `-A`, Kraken's MPA-style reports, HUMAnN's community
  totals) are turned into own values: a row's own value is its value less the
  values of the rows nearest below it. A remainder smaller than the rounding
  of the printed numbers (half a unit in the last printed digit of each value
  involved) is taken as zero, so rounding noise makes no rows.
* **Higher-rank values.** A value that stops at a rank above the lowest
  (a read Kraken assigned to a genus, MetaPhlAn abundance left at a genus
  after its species) becomes a row padded with `unclassified <prefix><taxon>`
  at every lower rank, as for SingleM (for example `unclassified
  g__Bacteroides`).
* **Unclassified.** What a tool could not place at all (Kraken's
  `unclassified` row and reads assigned to `root`; MetaPhlAn's
  `UNCLASSIFIED`/`UNKNOWN`; mOTUs' `unassigned`/`-1`) becomes the row
  `unclassified Root` at every rank, the name SingleM's unplaced coverage
  already had. Names containing "unclassified" are what STAMP's
  *unclassified* option removes or keeps.
* **Missing ranks.** NCBI lineages skip ranks (a genus with no family). The
  gap is filled with `<parent> (no <rank>)`, for example `Bacillales (no
  family)`, so that the hierarchy stays strict and the row is not mistaken
  for an unclassified one.
* **Counts.** Read counts are written as whole numbers; coverages and
  relative abundances to six significant digits.
* **Samples.** A table with several sample columns gives one sample per
  column, named by its header. A single-sample file is named by the sample
  name the file states, or else by the file name without its extension and
  without a tool's usual suffix (`_profile`, `_bracken_species`,
  `_genefamilies`, ...). A sample name used twice is an error.
* **Several files and directories.** Files of one format are combined, one
  sample per file or per column. Files of different formats are refused
  (`The files are a mixture of Kraken and Bracken profiles; load profiles of
  one kind at a time.`). A directory is read without its subdirectories;
  files that are none of these formats are skipped and named.

## SingleM and Sandpiper

Already read before this work (`singlem.rs`, `sandpiper.rs`).

* `singlem pipe -p` / `singlem summarise --output-taxonomic-profile`: header
  `sample\tcoverage\ttaxonomy`, no comment lines; taxonomy `Root; d__...;
  s__...`; coverage is the node's own; rows breadth-first; any number of
  samples per file; zero-coverage rows may be absent, as may the `Root`
  row. `--output-taxonomic-profile-with-extras` adds `full_coverage`,
  `relative_abundance` and `level` columns (ignored).
* `--taxonomic-profile-krona` is a Krona HTML page built by `ktImportText`;
  it holds nothing the condensed profile does not, and is not read.
* Other `singlem summarise` tables (species-by-site, filled profile, level
  coverage) are derived from the condensed profile and are not read.
* Sandpiper: JSON `{"condensed": <tree>, "sample_name": ...}`, one sample per
  file.

Ranks: Domain, Phylum, Class, Order, Family, Genus, Species (GTDB).

## mOTUs (`motus.rs`)

Two generations with different files.

**mOTUs 2 and 3** (`motus profile`, `motus merge`): three `#` lines,
the last being the column header:

```
# git tag version 3.1.0 |  motus version 3.1.0 | map_tax 3.1.0 | ... | calc_motu 3.1.0 -k mOTU -C no_CAMI -g 3 | taxonomy: ref_mOTU_3.1.0 meta_mOTU_3.1.0
# call: python motus profile -s reads.fq -n gut1
#consensus_taxonomy	gut1
Kandleria vitulina [ref_mOTU_v31_00001]	0.0688211617
unassigned	0.2307163722
```

* Column header: `#consensus_taxonomy\t<sample>...`; `#consensus_taxonomy\tNCBI_tax_id\t...`
  (`-p`); `#mOTU\tconsensus_taxonomy\t...` (`-u`, `-q`);
  `#mOTUs2_clade\t<sample>` (`-A`, the only header line). `motus merge`
  writes the same three lines (the first starting `# motus version`) with one
  column per sample, named by each profile's `-n`. A sample named `unnamed
  sample` (no `-n`) takes the file's name instead.
* Values: relative abundances (10 decimals, summing to 1 with `unassigned`),
  or with `-c` whole read counts. Every mOTU is listed, zeros included.
* Rows: `<name> [<mOTU id>]` by default; a name at the `-k` rank otherwise;
  with `-q` (or `-A`) a lineage `k__Bacteria|p__...|s__<name> [<id>]`. `-A`
  lists every rank with clade values. `unassigned` (`-1` before 2.6) is what
  could not be assigned to a species-level mOTU.
* CAMI (`-C`) and BIOM (`-B`) outputs are other formats and are not read as
  mOTUs profiles (a BIOM file loads as BIOM).

Profiles: with lineages, seven ranks Kingdom ... Species (`k__` prefixes,
NCBI); the species name keeps its mOTU id, so each mOTU is its own species.
Without lineages, one level named after the `-k` rank (`mOTU` by default).

**mOTUs 4** (`motus profile -o FILE` writes counts to `FILE` and relative
abundances to `FILE.relab`; `motus merge` alike):

```
#tool_version=4.1.0	database_version=4.1	min_alignment_length=75	min_mgcs=3	count_mode=INSERT_SCALED	value_type=counts
mOTU	Taxonomy	ERR4507416	ERR4507418
mOTUv4.0_000255	d__Bacteria;p__Bacillota;...;s__Flavonifractor plautii	48	43
```

GTDB taxonomy joined with `;`; zero rows left out; the unassigned entry has
an id containing `unassigned` (its taxonomy is empty, `d__;p___A;c__;...`).
Profiles: seven ranks Domain ... Species, the species written
`<GTDB species> [<mOTU id>]` as mOTUs 3 does, so that each mOTU stays a
species of its own (mOTUs 4 already puts the id in the names of unnamed
species, `Unknown Limosilactobacillus mOTUv4.0_000611`, which are kept as
they are).

## MetaPhlAn (`metaphlan.rs`)

```
#mpa_vJan21_CHOCOPhlAnSGB_202103
#/usr/bin/metaphlan reads.fq --input_type fastq -o profile.txt
#19048 reads processed
#SampleID	Metaphlan_Analysis
#clade_name	NCBI_tax_id	relative_abundance	additional_species
UNCLASSIFIED	-1	12.5	
k__Bacteria	2	87.5	
k__Bacteria|p__Firmicutes	2|1239	50.1	
...|s__Bacteroides_dorei|t__SGB1814	2|976|...|357276|	10.2	
```

* Header lines start with `#`: database (`#mpa_v30_...`, `#mpa_v31_...`,
  `#mpa_vJan21_...` to `#mpa_vJan26_...`), the command, `#N reads processed`
  (4.x; `bases` for long reads), `#SampleID\t<id>`, and the column header.
  MetaPhlAn 3 has no "reads processed" line.
* `-t rel_ab` columns: `clade_name`, `NCBI_tax_id` (`|`-joined), and
  `relative_abundance` (percent), `additional_species`.
  `-t rel_ab_w_read_stats`: `clade_name`, `clade_taxid`,
  `relative_abundance`, `coverage`, `estimated_number_of_reads_from_the_clade`
  (preceded by `#Estimated reads mapped to known clades: N` or, before 4.2,
  `#estimated_reads_mapped_to_known_clades:N`). STAMP reads the estimated
  reads (counts) when they are there, else the relative abundances.
* Rows: every rank with clade (cumulative) values, prefixes `k p c o f g s
  t`; `t__SGBnnnn` (4.x) is the species-level genome bin; MetaPhlAn 3 stops
  at `s__`. `UNCLASSIFIED` (4.x, default since 4.2) or `UNKNOWN` (3.x) is the
  share of unclassified reads.
* `merge_metaphlan_tables.py`: the database line, then
  `clade_name\t<sample>...` (no `#`; 4.0.3 and later) or
  `clade_name\tNCBI_tax_id\t<sample>...` (3.x and early 4.0); sample names are
  file names without extension and `_profile`.
* `sgb_to_gtdb_profile.py`: `#mpa_...\tr220`, `#clade_name\trelative_abundance`,
  `;`-joined GTDB lineages with `d__` prefixes.
* `--CAMI_format_output`, `--biom_format_output` and the separate viral
  (`--profile_vsc`) table are not read.

Profiles: Kingdom, Phylum, Class, Order, Family, Genus, Species and, when a
file has `t__` rows, SGB (Domain ... Species for GTDB-converted profiles).
Names are kept as MetaPhlAn writes them (`Bacteroides_dorei`, `GGB1331_SGB1788`).

## Kraken 2 and Kraken (`kraken.rs`)

Report (`--report`), no header:

```
 12.50	1250	1250	U	0	unclassified
 87.50	8750	12	R	1	root
 87.40	8740	30	R1	131567	  cellular organisms
 80.10	8010	105	D	2	    Bacteria
```

* Tab-separated: percentage of all reads (`%6.2f`, so padded), clade reads,
  direct reads, rank code, NCBI taxid, name indented two spaces per depth.
* `--report-minimizer-data` adds two columns (minimizers, distinct
  minimizers) after the direct reads: rank, taxid and name are always the
  last three columns.
* Rank codes `U R D K P C O F G S`, and a rank code plus a number for unranked
  taxa below it (`R1`, `G1`, `S1`, `S2`). Kraken 1 writes `-` for every
  other rank. `--report-zero-counts` adds zero rows.
* `--use-mpa-style` (and KrakenTools `kreport2mpa.py`): `d__Bacteria|p__...
  \t<clade reads>`, ranked taxa only, no unclassified row;
  `combine_mpa.py` adds `#Classification\t<sample>...`.

Profiles: Domain, Phylum, Class, Order, Family, Genus, Species and, when any
read was assigned below a species (`S1`), Strain (deeper `S2`... are summed
into their `S1`). Direct reads are the values. Kingdoms (`K`) and other
unranked taxa are not levels: their reads go to the nearest ranked taxon
above. A taxon with no domain above it (viruses in recent NCBI taxonomies,
under the unranked `Viruses`) takes the top unranked taxon below `root` other
than `cellular organisms` as its domain. Reads of `unclassified`, `root` and
`cellular organisms` become `unclassified Root`.

## Bracken (`kraken.rs`)

* Abundance table (`.bracken`), one rank:
  `name\ttaxonomy_id\ttaxonomy_lvl\tkraken_assigned_reads\tadded_reads\tnew_est_reads\tfraction_total_reads`.
  `combine_bracken_outputs.py`: `name\ttaxonomy_id\ttaxonomy_lvl` then
  `<sample>_num\t<sample>_frac` per sample. STAMP reads `new_est_reads`
  (`_num`): one level, named after the rank (`S` Species, `G` Genus, ...).
  No unclassified row.
* Report (`-w`, `<name>_bracken_species.kreport`): the Kraken report layout,
  percentages unpadded (`%0.2f`) and of the Bracken total, direct reads only
  on leaves, ranks below the estimated level removed; current Bracken leaves
  out the `unclassified` row, older versions kept it. Read as a Kraken
  report, but told apart from one (no padded percentage and no direct reads
  on internal taxa) so that a directory holding both is refused rather than
  every sample counted twice. A Kraken report is taken for
  Bracken's only if every percentage is 10 or more and no taxon with taxa
  below it has reads of its own, which real Kraken reports never are (root
  and the higher ranks always hold some reads).
  Bracken 3.1's direct reads are not reliable: in a genus-level report every
  genus that had species in the Kraken report shows 0 direct reads, though
  its species are dropped (found on real output, below). A Bracken report is
  therefore read by clade reads: a taxon's value is its clade reads less
  those of the taxa printed below it.

## HUMAnN (`humann.rs`)

HUMAnN 2.x and 3.0–3.9 write the same tables. Each starts with a header
line, and its rows are tab-separated. The writers end the file without a
newline. Values are printed with `{:.10f}` unless `--output-max-decimals`
changes that.

```
# Gene Family	demo_Abundance-RPKs
UNMAPPED	100.0000000000
UniRef50_R5IH84	37.0370370370
UniRef50_R5IH84|g__Bacteroides.s__Bacteroides_stercoris	37.0370370370
```

* **`_genefamilies.tsv`**
  - Header: `# Gene Family	<sample>_Abundance-RPKs`.
  - Rows: `UNMAPPED` first, then each family's community total followed by
    its strata, in decreasing order. A stratum is `g__Genus.s__Genus_species`
    or `unclassified`.
  - The strata of a family add up to its community total.
* **`_pathabundance.tsv`**
  - Header: `# Pathway	<sample>_Abundance`.
  - Rows: `UNMAPPED`, then `UNINTEGRATED` and its strata, then each pathway
    (`ID: name`) followed by its strata.
  - A pathway's community value is computed on its own and is **not** the
    sum of its strata.
* **`_pathcoverage.tsv`**
  - Header: `# Pathway	<sample>_Coverage`.
  - Values lie in [0, 1]. `UNMAPPED` and `UNINTEGRATED` rows are 1.
* **`humann_join_tables`** keeps each file's column headings. It fills
  missing values with 0 and puts `UNMAPPED`, `UNGROUPED` and `UNINTEGRATED`
  first.
* **`humann_renorm_table`** writes `%.6g` values. With `-p` it renames the
  columns (`_Abundance-CPM`, `-RELAB`).
* **`humann_regroup_table`** and **`humann_rename_table`** keep the first
  heading. Their rows become `K01838`, `5.4.2.6`, `GO:...` and so on, with a
  name after `: ` when renamed, and they add `UNGROUPED`.
* **`humann_split_stratified_table`** writes `_stratified.tsv` (rows with
  `|` only) and `_unstratified.tsv`.
* **HUMAnN 4** is in alpha (4.0.0a2) and differs:
  - File names are `_2_genefamilies`, `_4_pathabundance` and so on.
  - The first heading carries the version and units, for example
    `# Gene Family HUMAnN v4.0.0.alpha.2 Adjusted CPMs`.
  - Gene family columns are the bare sample name, and the unmapped row is
    `READS_UNMAPPED`.
  - Strata look like `s__Bacteroides_dorei.t__SGB1862`.
  - STAMP accepts these headers and strata, taking the genus from the
    species name. This is not checked against real output.

Profiles:

* **Gene families** with strata have three levels: Gene family, Genus and
  Species. The `unclassified` stratum, and any part of a total that is in no
  stratum, become `unclassified <family>`. Without strata, the profile has
  the gene family level only.
* **Pathway abundance** holds the community values, one level: Pathway. A
  table of strata alone gives Pathway, Genus and Species.
* **Pathway coverage** holds the community values. A table of strata alone
  is refused, since coverage cannot be added up.
* `UNMAPPED`, `UNINTEGRATED` and `UNGROUPED` are kept as features.
* Sample names are the column headings without `_Abundance...` or
  `_Coverage...`.

## Supported versions

| Tool | Versions | Outputs read |
|---|---|---|
| SingleM | 0.13 and later | `pipe -p`, `summarise --output-taxonomic-profile` (also `-with-extras`) |
| Sandpiper | 1.0 (2024) and later | condensed profile JSON |
| mOTUs | 2.x, 3.x, 4.x | `profile` (default, `-c`, `-p`, `-u`, `-q`, `-k`, `-A`; 4.x counts and `.relab`), `merge` |
| MetaPhlAn | 3.x, 4.x (to 4.2) | `-t rel_ab`, `-t rel_ab_w_read_stats`, `merge_metaphlan_tables.py`, `sgb_to_gtdb_profile.py` |
| Kraken | 1.x, 2.x (KrakenUniq not) | `--report` (6 columns, 8 with `--report-minimizer-data`), `--use-mpa-style`, KrakenTools `kreport2mpa.py`/`combine_mpa.py` |
| Bracken | 2.x | `.bracken` tables, `combine_bracken_outputs.py`, `-w` reports |
| HUMAnN | 3.x (2.x writes the same tables) | gene families, pathway abundance, pathway coverage; `humann_join_tables`, `humann_renorm_table`, `humann_regroup_table`, `humann_rename_table`, `humann_split_stratified_table` output |

Not read: KrakenUniq reports, KrakenTools `combine_kreports.py` tables, CAMI
formats (mOTUs `-C`, MetaPhlAn `--CAMI_format_output`), SingleM's Krona
pages, and HUMAnN 4 except as noted above.

## Checked against real output

`examples/input_file_formats/<tool>/stool_SRR17283923/` holds real output
of every tool for one human stool metagenome (SRR17283923, from a survey
of the Hong Kong population; NovaSeq, subsampled to 1,000,000 read pairs),
and
`crates/stamp-core/tests/stool_sample.rs` reads every file and checks the
values against the tool's own numbers. The versions run were:

- SingleM 0.21.4
- mOTUs 3.1.0 and 4.1.0
- MetaPhlAn 4.1.1
- HUMAnN 3.9
- Kraken 2.17.2
- Bracken 3.1

The Sandpiper file is the run's own profile (the whole run).

What real output turned up (some of it first seen with a glacier-fed
stream sediment sample, used before the stool sample, in which MetaPhlAn
and HUMAnN found almost nothing):

- the Bracken 3.1 genus-report behaviour above;
- Kraken 2's reports with the 2025 NCBI taxonomy have bacterial kingdoms
  (`K`, Bacillati, Pseudomonadati), which are not a level;
- mOTUs 4 writes its unassigned share as `mOTUv4.0_unassigned` with the
  lineage `d__;p___A;c__;o__;f__;g__;s__`; it becomes `unclassified Root`;
- MetaPhlAn's `sgb_to_gtdb_profile.py` rescales the classified clades to
  100 but keeps the `UNCLASSIFIED` line, so its profiles add up to more than
  100 (106.6 for this sample). STAMP reads the values as written;
- a profile can hold nothing above zero (MetaPhlAn and HUMAnN on a sample
  with no known organisms). STAMP refuses such files with "Nothing in the
  files has a value above zero".

## Directories and choosing a kind

`profiler::directory_to_spf(directory, files, only)` reads the files directly
in a directory. Hidden files are left out, and each file is detected from its
first 64 KiB.

* If the directory holds one format, all files of that format are combined
  and the rest are skipped.
* If it holds several formats, the call fails unless `only` names one. In the
  window, STAMP asks which kind to load (`PendingProfile::Directory`); on the
  command line, `--profile-format` names it. This happens with HUMAnN's three
  tables per sample, or with Kraken and Bracken reports in one directory.
* Files chosen together (not a directory) must all be of one format.
  Otherwise they are refused with "The files are a mixture of ... profiles".
