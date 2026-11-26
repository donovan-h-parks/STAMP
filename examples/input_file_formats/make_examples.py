#!/usr/bin/env python3
"""Write the illustrative example inputs in this directory.

One synthetic community of human gut metagenomes is drawn (20 species, five
healthy and five IBD samples, with a few species more or less common in IBD),
and written in the exact output format of each tool STAMP reads. The numbers
are made up; the formats follow each tool's own writer (see
docs/PROFILER_FORMATS.md). The same seed always gives the same files.

    python3 make_examples.py

Only the Python standard library is needed. The BIOM table in HDF5 format is
also written if h5py is installed. The files here were made with

    pixi exec --spec python=3.12 --spec h5py --spec numpy python make_examples.py

(Python before 3.12 adds up floating-point numbers slightly differently, which
changes the last digit of a few HUMAnN values.)
"""

import gzip
import json
import math
import os
import random

HERE = os.path.dirname(os.path.abspath(__file__))
RNG = random.Random(20261126)

SAMPLES = [f"H0{i}" for i in range(1, 6)] + [f"I0{i}" for i in range(1, 6)]
GROUP = {s: ("Healthy" if s.startswith("H") else "IBD") for s in SAMPLES}

# ------------------------------------------------------------ the taxonomy

# NCBI taxonomy: taxid -> (name, rank, parent taxid).
NCBI = {
    1: ("root", "no rank", None),
    131567: ("cellular organisms", "no rank", 1),
    2: ("Bacteria", "superkingdom", 131567),
    2157: ("Archaea", "superkingdom", 131567),
    1783270: ("FCB group", "clade", 2),
    68336: ("Bacteroidota/Chlorobiota group", "clade", 1783270),
    976: ("Bacteroidota", "phylum", 68336),
    200643: ("Bacteroidia", "class", 976),
    171549: ("Bacteroidales", "order", 200643),
    815: ("Bacteroidaceae", "family", 171549),
    816: ("Bacteroides", "genus", 815),
    817: ("Bacteroides fragilis", "species", 816),
    820: ("Bacteroides uniformis", "species", 816),
    909656: ("Phocaeicola", "genus", 815),
    821: ("Phocaeicola vulgatus", "species", 909656),
    171552: ("Prevotellaceae", "family", 171549),
    838: ("Prevotella", "genus", 171552),
    165179: ("Prevotella copri", "species", 838),
    171550: ("Rikenellaceae", "family", 171549),
    239759: ("Alistipes", "genus", 171550),
    28117: ("Alistipes putredinis", "species", 239759),
    2005525: ("Tannerellaceae", "family", 171549),
    375288: ("Parabacteroides", "genus", 2005525),
    823: ("Parabacteroides distasonis", "species", 375288),
    1783272: ("Terrabacteria group", "clade", 2),
    1239: ("Bacillota", "phylum", 1783272),
    186801: ("Clostridia", "class", 1239),
    186802: ("Eubacteriales", "order", 186801),
    216572: ("Oscillospiraceae", "family", 186802),
    216851: ("Faecalibacterium", "genus", 216572),
    853: ("Faecalibacterium prausnitzii", "species", 216851),
    1263: ("Ruminococcus", "genus", 216572),
    40518: ("Ruminococcus bromii", "species", 1263),
    186803: ("Lachnospiraceae", "family", 186802),
    841: ("Roseburia", "genus", 186803),
    166486: ("Roseburia intestinalis", "species", 841),
    1766253: ("Agathobacter", "genus", 186803),
    39491: ("Agathobacter rectalis", "species", 1766253),
    572511: ("Blautia", "genus", 186803),
    40520: ("Blautia obeum", "species", 572511),
    2316020: ("Mediterraneibacter", "genus", 186803),
    33038: ("Mediterraneibacter gnavus", "species", 2316020),
    91061: ("Bacilli", "class", 1239),
    186826: ("Lactobacillales", "order", 91061),
    81852: ("Enterococcaceae", "family", 186826),
    1350: ("Enterococcus", "genus", 81852),
    1351: ("Enterococcus faecalis", "species", 1350),
    909932: ("Negativicutes", "class", 1239),
    1843489: ("Veillonellales", "order", 909932),
    31977: ("Veillonellaceae", "family", 1843489),
    29465: ("Veillonella", "genus", 31977),
    29466: ("Veillonella parvula", "species", 29465),
    201174: ("Actinomycetota", "phylum", 1783272),
    1760: ("Actinomycetes", "class", 201174),
    85004: ("Bifidobacteriales", "order", 1760),
    31953: ("Bifidobacteriaceae", "family", 85004),
    1678: ("Bifidobacterium", "genus", 31953),
    216816: ("Bifidobacterium longum", "species", 1678),
    1680: ("Bifidobacterium adolescentis", "species", 1678),
    1224: ("Pseudomonadota", "phylum", 2),
    1236: ("Gammaproteobacteria", "class", 1224),
    91347: ("Enterobacterales", "order", 1236),
    543: ("Enterobacteriaceae", "family", 91347),
    561: ("Escherichia", "genus", 543),
    562: ("Escherichia coli", "species", 561),
    83333: ("Escherichia coli K-12", "strain", 562),
    570: ("Klebsiella", "genus", 543),
    573: ("Klebsiella pneumoniae", "species", 570),
    1783257: ("PVC group", "clade", 2),
    74201: ("Verrucomicrobiota", "phylum", 1783257),
    203494: ("Verrucomicrobiia", "class", 74201),
    48461: ("Verrucomicrobiales", "order", 203494),
    1647988: ("Akkermansiaceae", "family", 48461),
    239934: ("Akkermansia", "genus", 1647988),
    239935: ("Akkermansia muciniphila", "species", 239934),
    28890: ("Euryarchaeota", "phylum", 2157),
    183925: ("Methanobacteria", "class", 28890),
    2158: ("Methanobacteriales", "order", 183925),
    2159: ("Methanobacteriaceae", "family", 2158),
    2172: ("Methanobrevibacter", "genus", 2159),
    2173: ("Methanobrevibacter smithii", "species", 2172),
}
CHILDREN = {}
for _taxid, (_name, _rank, _parent) in NCBI.items():
    CHILDREN.setdefault(_parent, []).append(_taxid)

# Older NCBI names, which MetaPhlAn's and mOTUs' taxonomies still use.
OLD_NAMES = {
    "Bacteroidota": "Bacteroidetes",
    "Bacillota": "Firmicutes",
    "Pseudomonadota": "Proteobacteria",
    "Actinomycetota": "Actinobacteria",
    "Actinomycetes": "Actinobacteria",
    "Verrucomicrobiota": "Verrucomicrobia",
    "Verrucomicrobiia": "Verrucomicrobiae",
}

# The species: NCBI taxid, GTDB (r220) lineage below the domain, MetaPhlAn 4
# SGB, mOTUs 3 id, abundance in healthy guts (relative), and the factor by
# which IBD changes it.
SPECIES = [
    (817, "Bacteroidota;Bacteroidia;Bacteroidales;Bacteroidaceae;Bacteroides;Bacteroides fragilis", "SGB1855", "ref_mOTU_v31_00085", 6.0, 1.8),
    (820, "Bacteroidota;Bacteroidia;Bacteroidales;Bacteroidaceae;Bacteroides;Bacteroides uniformis", "SGB1836", "ref_mOTU_v31_00099", 8.0, 0.9),
    (821, "Bacteroidota;Bacteroidia;Bacteroidales;Bacteroidaceae;Phocaeicola;Phocaeicola vulgatus", "SGB1814", "ref_mOTU_v31_00128", 12.0, 1.3),
    (165179, "Bacteroidota;Bacteroidia;Bacteroidales;Bacteroidaceae;Segatella;Segatella copri", "SGB1626", "ref_mOTU_v31_00217", 7.0, 0.5),
    (28117, "Bacteroidota;Bacteroidia;Bacteroidales;Rikenellaceae;Alistipes;Alistipes putredinis", "SGB2318", "ref_mOTU_v31_00315", 4.0, 0.6),
    (823, "Bacteroidota;Bacteroidia;Bacteroidales;Tannerellaceae;Parabacteroides;Parabacteroides distasonis", "SGB1934", "ref_mOTU_v31_00406", 2.5, 1.2),
    (853, "Bacillota_A;Clostridia;Oscillospirales;Ruminococcaceae;Faecalibacterium;Faecalibacterium prausnitzii", "SGB15316", "ref_mOTU_v31_00861", 10.0, 0.25),
    (40518, "Bacillota_A;Clostridia;Oscillospirales;Ruminococcaceae;Ruminococcus_E;Ruminococcus_E bromii_B", "SGB4285", "ref_mOTU_v31_01012", 4.0, 0.5),
    (166486, "Bacillota_A;Clostridia;Lachnospirales;Lachnospiraceae;Roseburia;Roseburia intestinalis", "SGB4951", "ref_mOTU_v31_01185", 3.5, 0.35),
    (39491, "Bacillota_A;Clostridia;Lachnospirales;Lachnospiraceae;Agathobacter;Agathobacter rectalis", "SGB4933", "ref_mOTU_v31_01243", 7.0, 0.4),
    (40520, "Bacillota_A;Clostridia;Lachnospirales;Lachnospiraceae;Blautia_A;Blautia_A obeum", "SGB4810", "ref_mOTU_v31_01350", 3.0, 0.9),
    (33038, "Bacillota_A;Clostridia;Lachnospirales;Lachnospiraceae;Mediterraneibacter_A;Mediterraneibacter_A gnavus", "SGB4584", "ref_mOTU_v31_01398", 1.0, 6.0),
    (1351, "Bacillota;Bacilli;Lactobacillales;Enterococcaceae;Enterococcus;Enterococcus faecalis", "SGB7962", "ref_mOTU_v31_02127", 0.3, 5.0),
    (29466, "Bacillota_C;Negativicutes;Veillonellales;Veillonellaceae;Veillonella;Veillonella parvula", "SGB6939", "ref_mOTU_v31_02263", 0.6, 3.0),
    (216816, "Actinomycetota;Actinomycetes;Actinomycetales;Bifidobacteriaceae;Bifidobacterium;Bifidobacterium longum", "SGB17248", "ref_mOTU_v31_03142", 3.0, 0.7),
    (1680, "Actinomycetota;Actinomycetes;Actinomycetales;Bifidobacteriaceae;Bifidobacterium;Bifidobacterium adolescentis", "SGB17244", "ref_mOTU_v31_03158", 2.5, 0.8),
    (562, "Pseudomonadota;Gammaproteobacteria;Enterobacterales;Enterobacteriaceae;Escherichia;Escherichia coli", "SGB10068", "ref_mOTU_v31_00095", 1.0, 8.0),
    (573, "Pseudomonadota;Gammaproteobacteria;Enterobacterales;Enterobacteriaceae;Klebsiella;Klebsiella pneumoniae", "SGB10115", "ref_mOTU_v31_00109", 0.2, 5.0),
    (239935, "Verrucomicrobiota;Verrucomicrobiae;Verrucomicrobiales;Akkermansiaceae;Akkermansia;Akkermansia muciniphila", "SGB9226", "ref_mOTU_v31_03917", 2.0, 0.3),
    (2173, "Methanobacteriota;Methanobacteria;Methanobacteriales;Methanobacteriaceae;Methanobrevibacter_A;Methanobrevibacter_A smithii", "SGB714", "ref_mOTU_v31_04101", 1.0, 0.4),
]
TAXIDS = [s[0] for s in SPECIES]

STANDARD = {
    "superkingdom": "D", "kingdom": "K", "phylum": "P", "class": "C",
    "order": "O", "family": "F", "genus": "G", "species": "S",
}
PREFIX = {"D": "k__", "P": "p__", "C": "c__", "O": "o__", "F": "f__", "G": "g__", "S": "s__"}


def ancestors(taxid):
    """The taxids from the root down to `taxid`."""
    path = []
    while taxid is not None:
        path.append(taxid)
        taxid = NCBI[taxid][2]
    return path[::-1]


def rank_code(taxid):
    """Kraken 2's rank code of a taxon: R, D, P, ..., or one of these plus
    the distance below it (R1, D1, S1)."""
    if taxid == 1:
        return "R"
    distance = 0
    while True:
        name, rank, parent = NCBI[taxid]
        if rank in STANDARD:
            code = STANDARD[rank]
            return code if distance == 0 else f"{code}{distance}"
        if taxid == 1:
            return f"R{distance}"
        taxid, distance = parent, distance + 1


def ranked(taxid):
    """(code, taxid, name) of the taxa of standard ranks above and at `taxid`."""
    return [(STANDARD[NCBI[t][1]], t, NCBI[t][0]) for t in ancestors(taxid) if NCBI[t][1] in STANDARD]


def gtdb(index):
    taxid, lineage = SPECIES[index][0], SPECIES[index][1]
    domain = "Archaea" if 2157 in ancestors(taxid) else "Bacteria"
    return [domain] + lineage.split(";")


# ------------------------------------------------------- the abundances

def lognormal(sigma):
    return math.exp(RNG.gauss(0.0, sigma))


ABUNDANCE = {}
for sample in SAMPLES:
    ibd = GROUP[sample] == "IBD"
    values = []
    for taxid, _, _, _, base, factor in SPECIES:
        value = base * (factor if ibd else 1.0) * lognormal(0.45)
        # Rare species are missed in some samples.
        if base * (factor if ibd else 1.0) < 1.2 and RNG.random() < 0.3:
            value = 0.0
        values.append(value)
    total = sum(values)
    ABUNDANCE[sample] = [v / total for v in values]


def gzip_bytes(path, data):
    """Write `data` to `path` + ".gz", gzip-compressed with no time stamp or
    file name in the header, so the file is the same on every run."""
    with open(path + ".gz", "wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0, compresslevel=9) as f:
            f.write(data)


def write(path, text):
    """Write a text file, gzip-compressed: `path` gets ".gz" added."""
    path = os.path.join(HERE, path)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    gzip_bytes(path, text.encode("utf-8"))


def py_str(value, places):
    """A number as Python's str(round(value, places)) writes it."""
    return str(round(value, places))


# ------------------------------------------------------------ Kraken 2

def kraken_direct(sample, reads):
    """Reads assigned directly to each taxon: most at the species, some held
    back at higher ranks, a few at root, and some of E. coli's at K-12."""
    direct = {}
    for (taxid, *_), share in zip(SPECIES, ABUNDANCE[sample]):
        r = reads * share
        path = ancestors(taxid)
        split = {taxid: 0.74, path[-2]: 0.13, path[-3]: 0.05}
        rest = 0.08
        upper = [t for t in path[1:-3]]
        for t in upper:
            split[t] = split.get(t, 0.0) + rest / len(upper)
        if taxid == 562:
            split[83333] = 0.25 * split[562]
            split[562] *= 0.75
        for t, fraction in split.items():
            direct[t] = direct.get(t, 0) + int(round(r * fraction * RNG.uniform(0.85, 1.15)))
    direct[1] = int(reads * 0.004)
    direct[131567] = int(reads * 0.006)
    return direct


def clade_counts(direct):
    clade = {}
    for taxid, count in direct.items():
        for t in ancestors(taxid):
            clade[t] = clade.get(t, 0) + count
    return clade


def kraken_report(sample):
    total = RNG.randint(1_800_000, 3_200_000)
    unclassified = int(total * RNG.uniform(0.18, 0.32))
    direct = kraken_direct(sample, total - unclassified - int(total * 0.01))
    clade = clade_counts(direct)
    total = unclassified + clade[1]
    lines = [f"{100 * unclassified / total:6.2f}\t{unclassified}\t{unclassified}\tU\t0\tunclassified"]

    def visit(taxid, depth):
        if clade.get(taxid, 0) == 0:
            return
        name = NCBI[taxid][0]
        lines.append(
            f"{100 * clade[taxid] / total:6.2f}\t{clade[taxid]}\t{direct.get(taxid, 0)}\t"
            f"{rank_code(taxid)}\t{taxid}\t{'  ' * depth}{name}"
        )
        for child in sorted(CHILDREN.get(taxid, []), key=lambda c: (-clade.get(c, 0), c)):
            visit(child, depth + 1)

    visit(1, 0)
    return "\n".join(lines) + "\n", direct, clade, total


KRAKEN = {}
for sample in SAMPLES:
    text, direct, clade, total = kraken_report(sample)
    KRAKEN[sample] = (direct, clade)
    write(f"Kraken2/{sample}.k2report", text)


# ------------------------------------------------------------ Bracken

def bracken(sample):
    direct, clade = KRAKEN[sample]
    species_reads = {t: clade[t] for t in TAXIDS if clade.get(t, 0) > 0}
    estimate = dict(species_reads)
    # Reads held at a genus or family are shared among its species.
    for level in (2, 3):
        groups = {}
        for t in species_reads:
            groups.setdefault(ancestors(t)[-level], []).append(t)
        for parent, members in groups.items():
            held = direct.get(parent, 0)
            weight = sum(species_reads[m] for m in members)
            for m in members:
                estimate[m] += int(round(held * species_reads[m] / weight))
    estimate = {t: n for t, n in estimate.items() if n >= 10}
    total = sum(estimate.values())
    rows = ["name\ttaxonomy_id\ttaxonomy_lvl\tkraken_assigned_reads\tadded_reads\tnew_est_reads\tfraction_total_reads"]
    for t in sorted(estimate, key=lambda t: -estimate[t]):
        rows.append(
            f"{NCBI[t][0]}\t{t}\tS\t{species_reads[t]}\t{estimate[t] - species_reads[t]}\t"
            f"{estimate[t]}\t{estimate[t] / total:0.5f}"
        )
    write(f"Bracken/{sample}.bracken", "\n".join(rows) + "\n")

    # The report: estimated reads at the species, summed up the tree.
    report_clade = clade_counts(estimate)
    lines = []

    def visit(taxid, depth):
        if report_clade.get(taxid, 0) == 0 or NCBI[taxid][1] == "strain":
            return
        leaf = taxid in estimate
        lines.append(
            f"{100 * report_clade[taxid] / total:0.2f}\t{report_clade[taxid]}\t"
            f"{report_clade[taxid] if leaf else 0}\t{rank_code(taxid)}\t{taxid}\t"
            f"{'  ' * depth}{NCBI[taxid][0]}"
        )
        for child in sorted(CHILDREN.get(taxid, []), key=lambda c: (-report_clade.get(c, 0), c)):
            visit(child, depth + 1)

    visit(1, 0)
    write(f"Bracken/reports/{sample}_bracken_species.kreport", "\n".join(lines) + "\n")


for sample in SAMPLES:
    bracken(sample)


# ------------------------------------------------------- MetaPhlAn 4

def metaphlan_clades(sample):
    """Clade name -> (taxids, relative abundance %), and the unclassified %."""
    unclassified = RNG.uniform(6.0, 22.0)
    clades = {}
    for (taxid, _, sgb, *_), share in zip(SPECIES, ABUNDANCE[sample]):
        if share == 0.0:
            continue
        names, ids = [], []
        for code, t, name in ranked(taxid):
            name = OLD_NAMES.get(name, name).replace(" ", "_")
            names.append(PREFIX[code] + name)
            ids.append(str(t))
            clades.setdefault("|".join(names), ["|".join(ids), 0.0])[1] += share * (100 - unclassified)
        names.append("t__" + sgb)
        ids.append("")
        clades["|".join(names)] = ["|".join(ids), share * (100 - unclassified)]
    return clades, unclassified


METAPHLAN = {}
for number, sample in enumerate(SAMPLES):
    clades, unclassified = metaphlan_clades(sample)
    METAPHLAN[sample] = (clades, unclassified)
    reads = RNG.randint(9_000_000, 14_000_000)
    lines = [
        "#mpa_vJan21_CHOCOPhlAnSGB_202103",
        f"#/usr/local/bin/metaphlan {sample}_R1.fastq.gz,{sample}_R2.fastq.gz --input_type fastq "
        f"--nproc 8 --unclassified_estimation --bowtie2out {sample}.bowtie2.bz2 -o {sample}_profile.txt",
        f"#{reads} reads processed",
        "#SampleID\tMetaphlan_Analysis",
        "#clade_name\tNCBI_tax_id\trelative_abundance\tadditional_species",
        f"UNCLASSIFIED\t-1\t{py_str(unclassified, 5)}\t",
    ]
    order = sorted(clades, key=lambda c: (c.count("|"), -clades[c][1], c))
    for name in order:
        ids, value = clades[name]
        lines.append(f"{name}\t{ids}\t{py_str(value, 5)}\t")
    write(f"MetaPhlAn/{sample}_profile.txt", "\n".join(lines) + "\n")

# merge_metaphlan_tables.py
names = set()
for clades, _ in METAPHLAN.values():
    names.update(clades)
lines = ["#mpa_vJan21_CHOCOPhlAnSGB_202103", "clade_name\t" + "\t".join(SAMPLES)]
lines.append("UNCLASSIFIED\t" + "\t".join(py_str(METAPHLAN[s][1], 5) for s in SAMPLES))
for name in sorted(names, key=lambda c: (c.count("|"), c)):
    cells = []
    for s in SAMPLES:
        value = METAPHLAN[s][0].get(name, [None, 0.0])[1]
        cells.append(py_str(value, 5) if value else "0.0")
    lines.append(name + "\t" + "\t".join(cells))
write("MetaPhlAn/merged/merged_abundance_table.txt", "\n".join(lines) + "\n")


# ------------------------------------------------------------- mOTUs 3

def motus_lineage(index):
    taxid, *_, motu = SPECIES[index][:4]
    parts = []
    for code, t, name in ranked(taxid):
        name = OLD_NAMES.get(name, name)
        if code == "S":
            name = f"{name} [{motu}]"
        parts.append(PREFIX[code] + name)
    return "|".join(parts)


# mOTUs lists every mOTU of its database, most with nothing in a sample.
ABSENT = [
    ("ref_mOTU_v31_00470", "k__Bacteria|p__Firmicutes|c__Clostridia|o__Eubacteriales|f__Peptostreptococcaceae|g__Clostridioides|s__Clostridioides difficile [ref_mOTU_v31_00470]"),
    ("ref_mOTU_v31_00137", "k__Bacteria|p__Bacteroidetes|c__Bacteroidia|o__Bacteroidales|f__Bacteroidaceae|g__Bacteroides|s__Bacteroides thetaiotaomicron [ref_mOTU_v31_00137]"),
    ("ref_mOTU_v31_02470", "k__Bacteria|p__Firmicutes|c__Bacilli|o__Lactobacillales|f__Lactobacillaceae|g__Lactobacillus|s__Lactobacillus acidophilus [ref_mOTU_v31_02470]"),
]
HEADER = (
    "# git tag version 3.1.0 |  motus version 3.1.0 | map_tax 3.1.0 | gene database: nr3.1.0 | "
    "calc_mgc 3.1.0 -y insert.scaled_counts -l 75 | calc_motu 3.1.0 -k mOTU -C no_CAMI -g 3 {flags}| "
    "taxonomy: ref_mOTU_3.1.0 meta_mOTU_3.1.0"
)
MOTUS = {}
for sample in SAMPLES:
    inserts = RNG.randint(2_500, 4_500)
    counts = [int(round(inserts * share * lognormal(0.1))) for share in ABUNDANCE[sample]]
    unassigned = int(round(inserts * RNG.uniform(0.12, 0.25)))
    MOTUS[sample] = (counts, unassigned)
    lines = [
        HEADER.format(flags="-c -q "),
        f"# call: python /opt/mOTUs/motus profile -f {sample}_R1.fastq.gz -r {sample}_R2.fastq.gz "
        f"-n {sample} -c -q -t 8 -o {sample}.motus",
        f"#mOTU\tconsensus_taxonomy\t{sample}",
    ]
    rows = [(SPECIES[i][3], motus_lineage(i), counts[i]) for i in range(len(SPECIES))]
    rows += [(motu, lineage, 0) for motu, lineage in ABSENT]
    for motu, lineage, count in sorted(rows):
        lines.append(f"{motu}\t{lineage}\t{count}")
    lines.append(f"unassigned\tunassigned\t{unassigned}")
    write(f"mOTUs/{sample}.motus", "\n".join(lines) + "\n")

# motus merge of the relative-abundance profiles of the same samples.
lines = [
    "# motus version 3.1.0 | merge 3.1.0 | info merged profiles: " + HEADER.format(flags="-q "),
    "# call: python /opt/mOTUs/motus merge -d profiles -o merged.motus",
    "#mOTU\tconsensus_taxonomy\t" + "\t".join(SAMPLES),
]
rows = [(SPECIES[i][3], motus_lineage(i), i) for i in range(len(SPECIES))]
rows += [(motu, lineage, None) for motu, lineage in ABSENT]
fractions = {}
for sample in SAMPLES:
    counts, unassigned = MOTUS[sample]
    total = sum(counts) + unassigned
    fractions[sample] = ([c / total for c in counts], unassigned / total)
for motu, lineage, i in sorted(rows):
    values = [fractions[s][0][i] if i is not None else 0.0 for s in SAMPLES]
    lines.append(f"{motu}\t{lineage}\t" + "\t".join(f"{v:.10f}" for v in values))
lines.append("unassigned\tunassigned\t" + "\t".join(f"{fractions[s][1]:.10f}" for s in SAMPLES))
write("mOTUs/merged/merged.motus", "\n".join(lines) + "\n")


# ------------------------------------------------- SingleM and Sandpiper

def singlem_coverage(sample):
    """Own coverage of each GTDB lineage (tuple of names, without prefixes)."""
    total = RNG.uniform(60.0, 160.0)
    own = {(): total * 0.01}
    for i, share in enumerate(ABUNDANCE[sample]):
        lineage = tuple(gtdb(i))
        c = total * share
        for depth, fraction in ((7, 0.62), (6, 0.2), (5, 0.1), (4, 0.04), (2, 0.02), (1, 0.02)):
            key = lineage[:depth]
            own[key] = own.get(key, 0.0) + c * fraction * RNG.uniform(0.8, 1.2)
    return own


GTDB_PREFIX = ["d__", "p__", "c__", "o__", "f__", "g__", "s__"]
for sample in SAMPLES:
    own = singlem_coverage(sample)
    # SingleM writes rounded coverages, breadth first, leaving out zeros.
    lines = ["sample\tcoverage\ttaxonomy"]
    for lineage in sorted(own, key=lambda l: (len(l), l)):
        value = round(own[lineage], 2)
        if value == 0:
            continue
        taxonomy = "; ".join(["Root"] + [GTDB_PREFIX[d] + n for d, n in enumerate(lineage)])
        lines.append(f"{sample}\t{py_str(value, 2)}\t{taxonomy}")
    write(f"SingleM/{sample}.profile.tsv", "\n".join(lines) + "\n")

    # The Sandpiper condensed tree of the same coverages.
    def node(lineage, order):
        children = sorted({l[: len(lineage) + 1] for l in own if len(l) > len(lineage) and l[: len(lineage)] == lineage})
        return {
            "name": lineage[-1] if lineage else "",
            "size": round(own.get(lineage, 0.0), 6),
            "condensed_depth": len(lineage),
            "order": order,
            "children": [node(c, i) for i, c in enumerate(children)],
        }

    write(
        f"Sandpiper/{sample}.json",
        json.dumps({"condensed": node((), 0), "sample_name": sample}, separators=(",", ":")),
    )


# ----------------------------------------------------------------- HUMAnN 3

PATHWAYS = [
    "PWY-1042: glycolysis IV",
    "PWY-5100: pyruvate fermentation to acetate and lactate II",
    "ARGSYN-PWY: L-arginine biosynthesis I (via L-ornithine)",
    "CENTFERM-PWY: pyruvate fermentation to butanoate",
    "PWY-6123: inosine-5'-phosphate biosynthesis I",
    "PWY-7219: adenosine ribonucleotides de novo biosynthesis",
    "VALSYN-PWY: L-valine biosynthesis",
    "PWY-5097: L-lysine biosynthesis VI",
    "GLYCOGENSYNTH-PWY: glycogen biosynthesis I (from ADP-D-Glucose)",
    "PWY-6527: stachyose degradation",
    "HISDEG-PWY: L-histidine degradation I",
    "PWY-5677: succinate fermentation to butanoate",
    "FUC-RHAMCAT-PWY: superpathway of fucose and rhamnose degradation",
    "ENTBACSYN-PWY: enterobactin biosynthesis",
]
UNIPROT = "ABCDEFGHIJKLMNPQRSTUVWXYZ"


def uniref():
    """A UniProt accession of the A0A... form, as UniRef90 clusters are named."""
    alnum = UNIPROT + "0123456789"
    a = lambda: RNG.choice(alnum)
    d = lambda: str(RNG.randint(0, 9))
    return "UniRef90_A0A" + a() + a() + d() + RNG.choice(UNIPROT) + a() + a() + d()


def stratum(index):
    taxid = SPECIES[index][0]
    genus, species = (n for c, t, n in ranked(taxid) if c in "GS")
    return f"g__{genus}.s__{species.replace(' ', '_')}"


FAMILIES = []
for _ in range(30):
    members = RNG.sample(range(len(SPECIES)), RNG.randint(1, 4))
    FAMILIES.append((uniref(), members, {m: RNG.uniform(5, 60) for m in members}))
PATHWAY_MEMBERS = []
for name in PATHWAYS:
    members = RNG.sample(range(len(SPECIES)), RNG.randint(2, 6))
    if name.startswith("ENTBACSYN"):
        members = [16, 17]  # E. coli and Klebsiella
    PATHWAY_MEMBERS.append((name, members, {m: RNG.uniform(2, 40) for m in members}))


def ten(value):
    return f"{value:.10f}"


def humann_rows(features, extra=()):
    """Lines of a HUMAnN table: special rows, then each feature's community
    row followed by its strata, by decreasing value."""
    lines = list(extra)
    for name, total, strata in sorted(features, key=lambda f: (-f[1], f[0])):
        lines.append(f"{name}\t{ten(total)}")
        for label, value in sorted(strata, key=lambda s: (-s[1], s[0])):
            lines.append(f"{name}|{label}\t{ten(value)}")
    return lines


for sample in SAMPLES:
    shares = ABUNDANCE[sample]
    scale = RNG.uniform(800, 1500)
    families = []
    for name, members, weights in FAMILIES:
        strata = [(stratum(m), shares[m] * weights[m] * scale) for m in members if shares[m] > 0]
        unclassified = RNG.uniform(0, 0.2) * sum(v for _, v in strata) + RNG.uniform(0, 3)
        strata.append(("unclassified", unclassified))
        families.append((name, sum(v for _, v in strata), [s for s in strata if s[1] > 0]))
    unmapped = scale * RNG.uniform(250, 400)
    write(
        f"HUMAnN/{sample}_genefamilies.tsv",
        "\n".join([f"# Gene Family\t{sample}_Abundance-RPKs", f"UNMAPPED\t{ten(unmapped)}"]
                  + humann_rows(families)),
    )

    abundance, coverage = [], []
    for name, members, weights in PATHWAY_MEMBERS:
        strata = [(stratum(m), shares[m] * weights[m] * scale / 40) for m in members if shares[m] > 0]
        if not strata:
            continue
        strata.append(("unclassified", RNG.uniform(0.0, 0.3) * sum(v for _, v in strata)))
        # HUMAnN computes the community abundance separately: not the sum.
        community = sum(v for _, v in strata) * RNG.uniform(0.85, 1.05)
        abundance.append((name, community, strata))
        coverage.append((
            name,
            min(1.0, RNG.uniform(0.55, 1.0)),
            [(label, min(1.0, RNG.uniform(0.2, 1.0))) for label, _ in strata],
        ))
    unintegrated = scale * RNG.uniform(60, 120)
    integrated = [
        f"UNMAPPED\t{ten(unmapped / 40)}",
        f"UNINTEGRATED\t{ten(unintegrated)}",
    ] + [f"UNINTEGRATED|{stratum(i)}\t{ten(unintegrated * share * 0.8)}"
         for i, share in sorted(enumerate(shares), key=lambda x: -x[1]) if share > 0] + [
        f"UNINTEGRATED|unclassified\t{ten(unintegrated * 0.2)}"
    ]
    write(
        f"HUMAnN/{sample}_pathabundance.tsv",
        "\n".join(humann_rows(abundance, [f"# Pathway\t{sample}_Abundance"] + integrated)),
    )
    special = ["UNMAPPED\t1.0000000000", "UNINTEGRATED\t1.0000000000"] + [
        line.split("\t")[0] + "\t1.0000000000" for line in integrated[2:]
    ]
    write(
        f"HUMAnN/{sample}_pathcoverage.tsv",
        "\n".join(humann_rows(coverage, [f"# Pathway\t{sample}_Coverage"] + special)),
    )


# -------------------------------------------------------------------- BIOM

# A QIIME-style OTU table of 16S amplicons from the same samples: several
# OTUs per species, with Greengenes-style taxonomy.
OTUS = []
for i, (taxid, *_) in enumerate(SPECIES):
    lineage = [PREFIX[c] + OLD_NAMES.get(n, n) for c, _, n in ranked(taxid)]
    # Greengenes gives the species epithet alone.
    lineage[-1] = "s__" + lineage[-1].split()[-1]
    for _ in range(RNG.randint(1, 3)):
        OTUS.append((f"OTU_{len(OTUS) + 1}", i, lineage, RNG.uniform(0.2, 1.0)))
reads16s = {s: RNG.randint(18_000, 30_000) for s in SAMPLES}
data = []
for row, (otu, i, lineage, weight) in enumerate(OTUS):
    for column, sample in enumerate(SAMPLES):
        count = int(round(reads16s[sample] * ABUNDANCE[sample][i] * weight * lognormal(0.2) / 1.6))
        if count:
            data.append([row, column, count])
biom = {
    "id": "otu_table_mc2_w_tax",
    "format": "Biological Observation Matrix 1.0.0",
    "format_url": "http://biom-format.org",
    "type": "OTU table",
    "generated_by": "QIIME 1.9.1",
    "date": "2026-11-26T09:00:00",
    "rows": [{"id": otu, "metadata": {"taxonomy": lineage}} for otu, _, lineage, _ in OTUS],
    "columns": [{"id": s, "metadata": None} for s in SAMPLES],
    "matrix_type": "sparse",
    "matrix_element_type": "int",
    "shape": [len(OTUS), len(SAMPLES)],
    "data": data,
}
write("BIOM/otu_table.biom", json.dumps(biom, indent=1))

try:
    import h5py  # noqa: F401  (optional)
except ImportError:
    h5py = None
if h5py is not None:
    import numpy as np

    path = os.path.join(HERE, "BIOM/otu_table_hdf5.biom")
    dense = np.zeros((len(OTUS), len(SAMPLES)))
    for r, c, v in data:
        dense[r, c] = v
    with h5py.File(path, "w") as f:
        f.attrs["id"] = "otu_table_mc2_w_tax"
        f.attrs["type"] = "OTU table"
        f.attrs["format-url"] = "http://biom-format.org"
        f.attrs["format-version"] = (2, 1)
        f.attrs["generated-by"] = "make_examples.py"
        f.attrs["creation-date"] = "2026-11-26T09:00:00"
        f.attrs["nnz"] = len(data)
        f.attrs["shape"] = dense.shape
        strings = h5py.string_dtype()
        for axis, ids, matrix in (
            ("observation", [o[0] for o in OTUS], dense),
            ("sample", SAMPLES, dense.T),
        ):
            group = f.create_group(axis)
            group.create_dataset("ids", data=np.array(ids, dtype=object), dtype=strings)
            meta = group.create_group("metadata")
            if axis == "observation":
                meta.create_dataset(
                    "taxonomy",
                    data=np.array([o[2] for o in OTUS], dtype=object),
                    dtype=strings,
                )
            group.create_group("group-metadata")
            mdata = group.create_group("matrix")
            indptr, indices, values = [0], [], []
            for row in matrix:
                nz = np.nonzero(row)[0]
                indices.extend(nz.tolist())
                values.extend(row[nz].tolist())
                indptr.append(len(indices))
            mdata.create_dataset("data", data=np.array(values, dtype=float))
            mdata.create_dataset("indices", data=np.array(indices, dtype=np.int32))
            mdata.create_dataset("indptr", data=np.array(indptr, dtype=np.int32))
    with open(path, "rb") as f:
        gzip_bytes(path, f.read())
    os.remove(path)


# ------------------------------------------------------------------- mothur

# classify.seqs output for the unique sequences, the groups file of every
# read, and the names file mapping each unique sequence to its duplicates.
taxonomy_lines, group_lines, name_lines = [], [], []
read_number = 0
for u in range(260):
    i = RNG.choices(range(len(SPECIES)), weights=[s[4] for s in SPECIES])[0]
    levels = []
    for code, _, name in ranked(SPECIES[i][0]):
        if code == "S":
            break
        name = OLD_NAMES.get(name, name)
        levels.append(name)
    confidence = [100, 100, 100, 100, RNG.randint(85, 100), RNG.randint(60, 100)]
    if confidence[-1] < 80:
        levels[-1] = levels[-2] + "_unclassified"
        confidence[-1] = 100
    unique = f"M00{read_number + 1:05d}"
    taxonomy_lines.append(
        unique + "\t" + "".join(f"{n}({c});" for n, c in zip(levels, confidence))
    )
    members = []
    for _ in range(RNG.randint(1, 12)):
        read_number += 1
        sample_weights = [ABUNDANCE[s][i] + 1e-4 for s in SAMPLES]
        sample = RNG.choices(SAMPLES, weights=sample_weights)[0]
        read = f"M00{read_number:05d}"
        members.append(read)
        group_lines.append(f"{read}\t{sample}")
    name_lines.append(f"{unique}\t{','.join(members)}")
write("mothur/stool.unique.rdp.wang.taxonomy", "\n".join(taxonomy_lines) + "\n")
write("mothur/stool.groups", "\n".join(group_lines) + "\n")
write("mothur/stool.names", "\n".join(name_lines) + "\n")


# -------------------------------------------------------------- IMG/M COG

COGS = []
with open(os.path.join(HERE, "../../legacy/stamp/data/whog.txt")) as whog:
    for line in whog:
        if line.startswith("[") and " COG" in line:
            code, cog, name = line.rstrip("\n").split(" ", 2)
            COGS.append((cog, name))
RNG.shuffle(COGS)
lines = ["Func_id\tFunc_name\t" + "\t".join(SAMPLES)]
for cog, name in sorted(COGS[:60]):
    base = RNG.uniform(20, 400)
    ibd_factor = RNG.choice([1.0, 1.0, 1.0, 0.5, 2.0])
    counts = [
        int(round(base * (ibd_factor if GROUP[s] == "IBD" else 1.0) * lognormal(0.35)))
        for s in SAMPLES
    ]
    lines.append(f"{cog}\t{name}\t" + "\t".join(map(str, counts)))
write("IMG-M_COG/cog_profile.tsv", "\n".join(lines) + "\n")


# ----------------------------------------------------------------- metadata

lines = ["Sample\tGroup\tSubject age"]
for s in SAMPLES:
    lines.append(f"{s}\t{GROUP[s]}\t{RNG.randint(22, 68)}")
write("metadata.tsv", "\n".join(lines) + "\n")
