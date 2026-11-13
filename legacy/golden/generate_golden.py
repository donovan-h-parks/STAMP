"""Generate reference ("golden") outputs from the legacy Python STAMP code.

Runs the original statistics headlessly over the datasets in examples/ and
writes one TSV per case, plus manifest.json, into tests/golden/. The Rust
port is checked against these files.

Run with:  pixi run golden      (from legacy/golden)

Conventions of the output tables
  * Columns are STAMP's own results-table columns (with "No correction"
    applied), followed by extra columns:
      corr:<name>   p-values after each deterministic multiple-test correction
      es:<name>     each effect-size measure used by the filtering sidebar
  * Rows are sorted by the hierarchy columns. STAMP sorts by p-value, but its
    tie order depends on Python 2 dict ordering, so it is not reproducible.
  * Floats are written with repr() (shortest round-trip form).
  * Cases flagged "stochastic" in the manifest use random resampling and can
    only be compared within a tolerance.
"""
import io
import json
import os
import random
import sys
import time

import legacy_env
legacy_env.setup()

import numpy
numpy.seterr(all='ignore')

from stamp.metagenomics.fileIO.StampIO import StampIO
from stamp.metagenomics.fileIO.MetadataIO import MetadataIO
from stamp.metagenomics.stats.SampleStatsTests import SampleStatsTests
from stamp.metagenomics.stats.GroupStatsTests import GroupStatsTests
from stamp.metagenomics.stats.MultiGroupStatsTests import MultiGroupStatsTests

REPO = legacy_env.REPO_DIR
OUT_DIR = os.path.join(REPO, 'tests', 'golden')

PREFERENCES = {
    'Pseudocount': 0.5,
    'Replicates': 1000,
    'Executable directory': legacy_env.LEGACY_DIR,
    'Minimum reported p-value exponent': -15,
}

RETAIN = 'Retain unclassified reads'
REMOVE = 'Remove unclassified reads'
FREQ_ONLY = 'Use only for calculating frequency profiles'
ALL_OTHER = '<All other samples>'


# ---------------------------------------------------------------- plugins

def load_plugins(package, names):
    plugins = {}
    for name in names:
        module = __import__(package + '.' + name, fromlist=['*'])
        obj = getattr(module, name)(PREFERENCES)
        plugins[obj.name] = obj
    return plugins

P = 'stamp.plugins.'
SAMPLE_TESTS = load_plugins(P + 'samples.statisticalTests',
    ['ChiSquare', 'ChiSquareYates', 'DiffBetweenProp', 'Fishers', 'GTest',
     'GTestYates', 'GTestFisher', 'Hypergeometric', 'Bootstrap', 'Permutation'])
SAMPLE_CI = load_plugins(P + 'samples.confidenceIntervalMethods',
    ['DiffBetweenPropAsymptotic', 'DiffBetweenPropAsymptoticCC', 'NewcombeWilson',
     'OddsRatio', 'RatioProportions'])
SAMPLE_ES = load_plugins(P + 'samples.effectSizeFilters',
    ['DiffBetweenProp', 'OddsRatio', 'RatioProportions'])
GROUP_TESTS = load_plugins(P + 'groups.statisticalTests', ['Ttest', 'Welch', 'White'])
GROUP_ES = load_plugins(P + 'groups.effectSizeFilters', ['DiffBetweenProp', 'RatioProportions'])
MULTI_TESTS = load_plugins(P + 'multiGroups.statisticalTests', ['ANOVA', 'KruskalWallis'])
MULTI_ES = load_plugins(P + 'multiGroups.effectSizeFilters', ['EtaSquared'])
POST_HOC = load_plugins(P + 'multiGroups.postHoc',
    ['GamesHowell', 'Scheffe', 'TukeyKramer', 'WelchUncorrected'])
CORRECTIONS = load_plugins(P + 'common.multipleComparisonCorrections',
    ['NoCorrection', 'BenjaminiHochbergFDR', 'Bonferroni', 'Sidak'])
# Holm-Bonferroni (in additional/) is not loaded by the GUI and does not return
# adjusted p-values, so it is deliberately left out of the reference outputs.
STOREY = load_plugins(P + 'common.multipleComparisonCorrections', ['StoreyFDR'])['Storey FDR']

STOCHASTIC_TESTS = set(['Bootstrap', 'Permutation', "White's non-parametric t-test"])
DETERMINISTIC_CORRECTIONS = ['Benjamini-Hochberg FDR', 'Bonferroni', 'Sidak']


# ---------------------------------------------------------------- output

def fmt(value):
    if isinstance(value, float) or isinstance(value, numpy.floating):
        return repr(float(value))
    if isinstance(value, (int, long, numpy.integer)):
        return str(int(value))
    if isinstance(value, unicode):
        return value
    return str(value).decode('utf-8', 'replace')


manifest = []
_used_ids = set()


def write_case(case_id, params, headings, rows, n_key_cols):
    assert case_id not in _used_ids, case_id
    _used_ids.add(case_id)

    rows = sorted(rows, key=lambda r: tuple(fmt(v) for v in r[0:n_key_cols]))
    filename = case_id + '.tsv'
    with io.open(os.path.join(OUT_DIR, filename), 'w', encoding='utf-8', newline='\n') as fout:
        fout.write(u'\t'.join(fmt(h) for h in headings) + u'\n')
        for row in rows:
            fout.write(u'\t'.join(fmt(v) for v in row) + u'\n')

    entry = dict(params)
    entry['id'] = case_id
    entry['file'] = filename
    entry['rows'] = len(rows)
    entry['key_columns'] = n_key_cols
    manifest.append(entry)
    sys.stdout.write('  %-70s %6d rows\n' % (case_id, len(rows)))
    sys.stdout.flush()


def add_corrections(headings, rows, p_index):
    """Append p-values corrected by each deterministic method."""
    pvalues = [row[p_index] for row in rows]
    for name in DETERMINISTIC_CORRECTIONS:
        headings.append('corr:' + name)
        corrected = CORRECTIONS[name].correct(list(pvalues), 0.05) if pvalues else []
        for row, value in zip(rows, corrected):
            row.append(value)


# ---------------------------------------------------------------- loading

_tree_cache = {}


def load(profile, metadata=None, field=None):
    key = (profile, metadata, field)
    if key not in _tree_cache:
        tree, err = StampIO(PREFERENCES).read(os.path.join(REPO, profile))
        if err is not None:
            raise RuntimeError('%s: %s' % (profile, err))
        meta = None
        if metadata is not None:
            meta, _warning = MetadataIO(PREFERENCES).read(os.path.join(REPO, metadata), tree)
            meta.setActiveField(field, tree)
        _tree_cache[key] = (tree, meta)
    return _tree_cache[key]


def seed(case_id):
    value = hash(case_id) & 0x7fffffff
    random.seed(value)
    numpy.random.seed(value)


# ---------------------------------------------------------------- two samples

def sample_case(case_id, profile, sample1, sample2, parent, level, unclassified,
                test, test_type, ci, coverage):
    tree, _ = load(profile)
    prof = tree.createSampleProfile(sample1, sample2, parent, level, unclassified)

    seed(case_id)
    runner = SampleStatsTests(PREFERENCES)
    runner.run(SAMPLE_TESTS[test], test_type, SAMPLE_CI[ci], coverage, prof, None)
    res = runner.results
    res.performMultCompCorrection(CORRECTIONS['No correction'])

    rows, headings = res.tableData(False)
    rows = [list(r) for r in rows]
    headings = list(headings)
    n_key = len(prof.hierarchyHeadings)

    add_corrections(headings, rows, n_key - 1 + res.dataHeadings['pValues'])
    for name in sorted(SAMPLE_ES.keys()):
        headings.append('es:' + name)
        for row in rows:
            seq1, seq2, parent1, parent2 = row[n_key:n_key + 4]
            row.append(SAMPLE_ES[name].run(seq1, seq2, parent1, parent2))

    write_case(case_id, {
        'mode': 'two_samples', 'profile': profile,
        'sample1': sample1, 'sample2': sample2,
        'parent_level': parent, 'profile_level': level, 'unclassified': unclassified,
        'test': test, 'test_type': test_type, 'ci_method': ci, 'coverage': coverage,
        'replicates': PREFERENCES['Replicates'], 'pseudocount': PREFERENCES['Pseudocount'],
        'stochastic': test in STOCHASTIC_TESTS,
    }, headings, rows, n_key)


# ---------------------------------------------------------------- two groups

def group_case(case_id, profile, metadata, field, group1, group2, parent, level,
               unclassified, test, test_type, coverage):
    tree, meta = load(profile, metadata, field)
    prof = tree.createGroupProfile(group1, group2, parent, level, meta, unclassified)

    seed(case_id)
    stat_test = GROUP_TESTS[test]
    ci = stat_test.confIntervMethods[0]
    runner = GroupStatsTests(PREFERENCES)
    runner.run(stat_test, test_type, ci, coverage, prof, None)
    res = runner.results
    res.performMultCompCorrection(CORRECTIONS['No correction'])

    rows, headings = res.tableData(False)
    rows = [list(r) for r in rows]
    headings = list(headings)
    n_key = len(prof.hierarchyHeadings)

    add_corrections(headings, rows, n_key - 1 + res.dataHeadings['pValues'])
    for name in sorted(GROUP_ES.keys()):
        headings.append('es:' + name)
        for row in rows:
            prop1, prop2 = prof.getFeatureProportions(row[n_key - 1])
            try:
                a = GROUP_ES[name].run(prop1, prop2)
                b = GROUP_ES[name].run(prop2, prop1)
                row.append(max(a, b))
            except ZeroDivisionError:
                # legacy bug: ratio of proportions crashes when the feature
                # is absent from both groups
                row.append('error: ZeroDivisionError')

    write_case(case_id, {
        'mode': 'two_groups', 'profile': profile, 'metadata': metadata, 'group_field': field,
        'group1': group1, 'group2': group2,
        'samples_in_group1': prof.samplesInGroup1, 'samples_in_group2': prof.samplesInGroup2,
        'parent_level': parent, 'profile_level': level, 'unclassified': unclassified,
        'test': test, 'test_type': test_type, 'ci_method': ci, 'coverage': coverage,
        'replicates': PREFERENCES['Replicates'],
        'stochastic': test in STOCHASTIC_TESTS,
    }, headings, rows, n_key)


# ---------------------------------------------------------------- multiple groups

def multi_group_case(case_id, profile, metadata, field, parent, level, unclassified,
                     test, post_hoc_features=5, coverage=0.95, active_groups=None):
    tree, meta = load(profile, metadata, field)
    for name in tree.groupActive:
        tree.groupActive[name] = (active_groups is None or name in active_groups)
    prof = tree.createMultiGroupProfile(tree.groupDict.keys(), parent, level, meta, unclassified)
    prof.setActiveGroups(tree.groupActive)

    runner = MultiGroupStatsTests(PREFERENCES)
    runner.run(MULTI_TESTS[test], MULTI_ES['Eta-squared'], prof, None)
    res = runner.results
    res.performMultCompCorrection(CORRECTIONS['No correction'])

    rows, headings = res.tableData(False)
    rows = [list(r) for r in rows]
    headings = list(headings)
    n_key = len(prof.hierarchyHeadings)
    p_index = n_key - 1 + res.dataHeadings['pValues']
    add_corrections(headings, rows, p_index)

    params = {
        'mode': 'multiple_groups', 'profile': profile, 'metadata': metadata, 'group_field': field,
        'groups': list(prof.activeGroupNames),
        'samples_in_groups': [list(s) for s in prof.activeSamplesInGroups],
        'parent_level': parent, 'profile_level': level, 'unclassified': unclassified,
        'test': test, 'effect_size': 'Eta-squared', 'stochastic': False,
    }
    write_case(case_id, params, headings, rows, n_key)

    # post-hoc tests on the most significant features (ties broken by name)
    ranked = sorted(rows, key=lambda r: (r[p_index], tuple(r[0:n_key])))
    features = [r[n_key - 1] for r in ranked[0:post_hoc_features]]
    ph_rows = []
    for feature in features:
        for name in sorted(POST_HOC.keys()):
            try:
                runner.runPostHocTest(POST_HOC[name], prof, feature, coverage)
            except Exception as e:
                # legacy bug: e.g. Games-Howell crashes when a group has one sample
                ph_rows.append([feature, name, 'error: ' + e.__class__.__name__, '', '', '', '', str(e)])
                continue
            ph = runner.results.postHocResults
            for i in xrange(0, len(ph.labels)):
                ph_rows.append([feature, name, ph.labels[i], ph.pValues[i], ph.effectSizes[i],
                                ph.lowerCIs[i], ph.upperCIs[i], ph.note])
    ph_params = dict(params)
    ph_params.update({'mode': 'post_hoc', 'coverage': coverage, 'parent_case': case_id})
    write_case(case_id + '__posthoc', ph_params,
               ['Feature', 'Post-hoc test', 'Groups', 'p-value', 'Effect size',
                'Lower CI', 'Upper CI', 'Note'], ph_rows, 3)


# ---------------------------------------------------------------- Storey FDR

def storey_case(case_id, source_case):
    """Storey q-values for the p-values of an existing case (stochastic lambda choice)."""
    entry = [m for m in manifest if m['id'] == source_case][0]
    with io.open(os.path.join(OUT_DIR, entry['file']), encoding='utf-8') as fin:
        lines = [line.rstrip(u'\n').split(u'\t') for line in fin]
    headings, rows = lines[0], lines[1:]
    n_key = entry['key_columns']
    p_index = headings.index(u'p-values')
    pvalues = [float(r[p_index]) for r in rows]

    seed(case_id)
    qvalues = STOREY.correct(pvalues, 0.05)
    info = dict((k, v) for k, v in STOREY.additionalInfo())
    out = [r[0:n_key] + [p, q] for r, p, q in zip(rows, pvalues, qvalues)]
    write_case(case_id, {
        'mode': 'storey_fdr', 'source_case': source_case, 'stochastic': True,
        'estimated_pi0': info['Estimated pi_0'],
        'significant_features': info['Number of significant features'],
    }, headings[0:n_key] + ['p-values', 'q-values'], out, n_key)


# ---------------------------------------------------------------- case list

def main():
    if not os.path.isdir(OUT_DIR):
        os.makedirs(OUT_DIR)
    for name in os.listdir(OUT_DIR):
        if name.endswith('.tsv') or name == 'manifest.json':
            os.remove(os.path.join(OUT_DIR, name))

    start = time.time()
    E = 'Entire sample'
    det_tests = ['Chi-square test', "Chi-square test (w/ Yates')", 'Difference between proportions',
                 "Fisher's exact test", 'G-test', "G-test (w/ Yates')",
                 "G-test (w/ Yates') + Fisher's", 'Hypergeometric']
    ci_methods = ['DP: Asymptotic', 'DP: Asymptotic-CC', 'DP: Newcombe-Wilson',
                  'OR: Haldane adjustment', 'RP: Asymptotic']
    slug = {
        'Chi-square test': 'chi2', "Chi-square test (w/ Yates')": 'chi2yates',
        'Difference between proportions': 'dp', "Fisher's exact test": 'fisher',
        'G-test': 'gtest', "G-test (w/ Yates')": 'gtestyates',
        "G-test (w/ Yates') + Fisher's": 'gtestfisher', 'Hypergeometric': 'hypergeom',
        'Bootstrap': 'bootstrap', 'Permutation': 'permutation',
        'DP: Asymptotic': 'dpasym', 'DP: Asymptotic-CC': 'dpasymcc',
        'DP: Newcombe-Wilson': 'newcombe', 'OR: Haldane adjustment': 'or', 'RP: Asymptotic': 'rp',
        't-test (equal variance)': 'ttest', "Welch's t-test": 'welch',
        "White's non-parametric t-test": 'white',
        'ANOVA': 'anova', 'Kruskal-Wallis H-test': 'kruskal',
    }

    print 'Two-sample cases'
    iron = 'examples/IronMineEdwards/EdwardsIronMine.spf'
    coal_spf = 'examples/CoalSeamGas/coal.spf'
    ebpr_small = 'examples/EBPR/MartinEBPR_A_phosphatis_Phosphate_Metabolism.spf'
    cow = 'examples/CowRumen-MG-RAST/CowRumen.spf'
    ci_example = 'examples/CI_example.spf'

    # every deterministic test, and every CI method, on two datasets
    for test in det_tests:
        sample_case('s_ironmine_subsystem_%s' % slug[test], iron, 'Red', 'Black',
                    E, 'Subsystem', RETAIN, test, 'Two-sided', 'DP: Asymptotic', 0.95)
        sample_case('s_cowrumen_level1_%s' % slug[test], cow, '4441682.3', '4441681.3',
                    E, 'level 1', RETAIN, test, 'Two-sided', 'DP: Asymptotic', 0.95)
    for ci in ci_methods:
        sample_case('s_ironmine_class_fisher_ci_%s' % slug[ci], iron, 'Red', 'Black',
                    E, 'Classification', RETAIN, "Fisher's exact test", 'Two-sided', ci, 0.95)
        sample_case('s_ciexample_fisher_ci_%s' % slug[ci], ci_example, 'Sample 1', 'Sample 2',
                    'Level 1', 'Level 2', RETAIN, "Fisher's exact test", 'Two-sided', ci, 0.95)

    # one-sided, other coverages, parent levels and unclassified treatments
    sample_case('s_ironmine_class_fisher_onesided', iron, 'Red', 'Black',
                E, 'Classification', RETAIN, "Fisher's exact test", 'One-sided', 'DP: Asymptotic', 0.95)
    sample_case('s_ironmine_class_gtest_onesided_cov99', iron, 'Red', 'Black',
                E, 'Classification', RETAIN, 'G-test', 'One-sided', 'DP: Newcombe-Wilson', 0.99)
    sample_case('s_ironmine_subsystem_in_class_fisher', iron, 'Red', 'Black',
                'Classification', 'Subsystem', RETAIN, "Fisher's exact test", 'Two-sided',
                'DP: Asymptotic', 0.95)
    sample_case('s_ironmine_subclass_in_class_chi2_cov90', iron, 'Red', 'Black',
                'Classification', 'Subclassification', RETAIN, 'Chi-square test', 'Two-sided',
                'RP: Asymptotic', 0.90)
    for treatment, tag in [(RETAIN, 'retain'), (REMOVE, 'remove'), (FREQ_ONLY, 'freqonly')]:
        sample_case('s_coal_class_fisher_unclassified_%s' % tag, coal_spf, '573891', '573889',
                    E, 'Class', treatment, "Fisher's exact test", 'Two-sided',
                    'DP: Newcombe-Wilson', 0.95)
        sample_case('s_coal_family_in_phylum_gtestfisher_unclassified_%s' % tag, coal_spf, '573875', '573878',
                    'Phylum', 'Family', treatment,
                    "G-test (w/ Yates') + Fisher's", 'Two-sided', 'DP: Asymptotic-CC', 0.95)
    sample_case('s_cowrumen_level3_fisher', cow, '4441682.3', '4441680.3',
                E, 'level 3', RETAIN, "Fisher's exact test", 'Two-sided', 'DP: Newcombe-Wilson', 0.95)
    sample_case('s_cowrumen_function_in_level1_gtestfisher', cow, '4441679.3', '4441681.3',
                'level 1', 'function', RETAIN, "G-test (w/ Yates') + Fisher's", 'Two-sided',
                'DP: Asymptotic', 0.95)

    # resampling tests (stochastic)
    for test in ['Bootstrap', 'Permutation']:
        sample_case('s_ebprsmall_%s' % slug[test], ebpr_small, 'USA', 'Australia',
                    E, 'Functional Role Assignment', RETAIN, test, 'Two-sided', 'DP: Asymptotic', 0.95)
        sample_case('s_ironmine_class_%s' % slug[test], iron, 'Red', 'Black',
                    E, 'Classification', RETAIN, test, 'Two-sided', 'DP: Asymptotic', 0.95)

    print 'Two-group cases'
    ent = ('examples/EnterotypesArumugam/Enterotypes.profile.spf',
           'examples/EnterotypesArumugam/Enterotypes.metadata.tsv', 'Enterotype')
    cyano = ('examples/Melainabacteria/cog_category_table.spf',
             'examples/Melainabacteria/cyano.metadata.tsv', 'Class')
    coal = ('examples/CoalSeamGas/coal.spf',
            'examples/CoalSeamGas/coal.metadata.tsv', 'Environment')
    diet = ('examples/table_stamp.txt', 'examples/mapping_file.txt', 'Diet')
    heat = ('examples/heatmap.spf', 'examples/heatmap.metadata.tsv', 'Group')

    for test in ['t-test (equal variance)', "Welch's t-test"]:
        t = slug[test]
        group_case('g_enterotypes_genera_e1_e3_%s' % t, ent[0], ent[1], ent[2],
                   'Enterotype 1', 'Enterotype 3', E, 'Genera', RETAIN, test, 'Two-sided', 0.95)
        group_case('g_enterotypes_phyla_e1_others_%s' % t, ent[0], ent[1], ent[2],
                   'Enterotype 1', ALL_OTHER, E, 'Phyla', RETAIN, test, 'Two-sided', 0.95)
        group_case('g_enterotypes_genera_in_phyla_e2_e3_%s_onesided_cov99' % t, ent[0], ent[1], ent[2],
                   'Enterotype 2', 'Enterotype 3', 'Phyla', 'Genera', RETAIN, test, 'One-sided', 0.99)
        group_case('g_diet_genus_hfhs_others_%s' % t, diet[0], diet[1], diet[2],
                   'HFHS', ALL_OTHER, E, 'Genus', RETAIN, test, 'Two-sided', 0.95)
        group_case('g_diet_family_in_phylum_cc_hfhs_%s_cov90' % t, diet[0], diet[1], diet[2],
                   'CC', 'HFHS', 'Phylum', 'Family', RETAIN, test, 'Two-sided', 0.90)
        group_case('g_cyano_cog_%s' % t, cyano[0], cyano[1], cyano[2],
                   'Melainabacteria', 'Oxyphotobacteria', E, 'COG Category', RETAIN, test,
                   'Two-sided', 0.95)
        group_case('g_diet_cc_chow_%s' % t, diet[0], diet[1], diet[2],
                   'CC', 'Chow', E, 'Class', RETAIN, test, 'Two-sided', 0.95)
        group_case('g_heatmap_a_b_%s' % t, heat[0], heat[1], heat[2],
                   'A', 'B', E, 'Category', RETAIN, test, 'Two-sided', 0.95)
        for treatment, tag in [(RETAIN, 'retain'), (REMOVE, 'remove'), (FREQ_ONLY, 'freqonly')]:
            group_case('g_coal_phylum_core_water_%s_%s' % (t, tag), coal[0], coal[1], coal[2],
                       'Core', 'Produced water', E, 'Phylum', treatment, test, 'Two-sided', 0.95)
        group_case('g_coal_class_in_phylum_deep_shallow_%s' % t, coal[0], coal[1], coal[2],
                   'Deep cutting', 'Shallow cutting', 'Phylum', 'Class', REMOVE, test, 'Two-sided', 0.95)

    # White's test (stochastic)
    group_case('g_enterotypes_phyla_e1_e3_white', ent[0], ent[1], ent[2],
               'Enterotype 1', 'Enterotype 3', E, 'Phyla', RETAIN,
               "White's non-parametric t-test", 'Two-sided', 0.95)
    group_case('g_cyano_cog_white', cyano[0], cyano[1], cyano[2],
               'Melainabacteria', 'Oxyphotobacteria', E, 'COG Category', RETAIN,
               "White's non-parametric t-test", 'Two-sided', 0.95)

    print 'Multiple-group cases'
    for test in ['ANOVA', 'Kruskal-Wallis H-test']:
        t = slug[test]
        multi_group_case('m_enterotypes_genera_%s' % t, ent[0], ent[1], ent[2], E, 'Genera', RETAIN, test)
        multi_group_case('m_enterotypes_genera_%s_e123' % t, ent[0], ent[1], ent[2], E, 'Genera', RETAIN, test,
                         active_groups=['Enterotype 1', 'Enterotype 2', 'Enterotype 3'])
        multi_group_case('m_enterotypes_genera_in_phyla_%s' % t, ent[0], ent[1], ent[2],
                         'Phyla', 'Genera', RETAIN, test)
        multi_group_case('m_diet_genus_in_family_%s' % t, diet[0], diet[1], diet[2],
                         'Family', 'Genus', RETAIN, test)
        multi_group_case('m_coal_order_%s_remove' % t, coal[0], coal[1], coal[2], E, 'Order', REMOVE, test)
        multi_group_case('m_coal_phylum_%s' % t, coal[0], coal[1], coal[2], E, 'Phylum', RETAIN, test)
        multi_group_case('m_coal_class_%s_freqonly' % t, coal[0], coal[1], coal[2], E, 'Class', FREQ_ONLY, test)
        multi_group_case('m_diet_class_%s' % t, diet[0], diet[1], diet[2], E, 'Class', RETAIN, test)
        multi_group_case('m_cyano_cog_%s' % t, cyano[0], cyano[1], cyano[2], E, 'COG Category', RETAIN, test)

    print 'Storey FDR cases'
    storey_case('storey_s_ironmine_subsystem_fisher', 's_ironmine_subsystem_fisher')
    storey_case('storey_g_enterotypes_genera_e1_e3_welch', 'g_enterotypes_genera_e1_e3_welch')
    storey_case('storey_m_coal_phylum_anova', 'm_coal_phylum_anova')

    print 'Profiles the legacy reader rejects'
    load_errors = []
    for profile in ['examples/EBPR/MartinEBPR.spf', 'examples/EBPR/MartinEBPR_A_phosphatis.spf',
                    'examples/FinegoldAutism/Autism.spf',
                    'examples/MouseGut-IMG-M/MouseGut.COG.profile.tsv']:
        _tree, err = StampIO(PREFERENCES).read(os.path.join(REPO, profile))
        assert err is not None, profile
        load_errors.append({'profile': profile, 'error': err})
        print '  %s: %s' % (profile, err)

    with io.open(os.path.join(OUT_DIR, 'manifest.json'), 'w', encoding='utf-8', newline='\n') as fout:
        text = json.dumps({
            'generator': 'legacy/golden/generate_golden.py',
            'python': sys.version.split()[0],
            'numpy': numpy.__version__,
            'scipy': __import__('scipy').__version__,
            'cases': manifest,
            'load_errors': load_errors,
        }, indent=1, sort_keys=True, ensure_ascii=False)
        fout.write(text if isinstance(text, unicode) else text.decode('utf-8'))
        fout.write(u'\n')

    print 'Wrote %d cases to %s in %.0f s' % (len(manifest), OUT_DIR, time.time() - start)


if __name__ == '__main__':
    main()
