"""
Headless ports of STAMP's file-import converters (no Qt).

Each function reproduces the parsing/writing logic of the corresponding
stamp/GUI/createProfile*Dlg.py dialog, writing a STAMP .spf profile. Kept out of app.py to
keep that file focused. BIOM lives in app.py; Append-COG reuses STAMP's Qt-free COG_IO.
"""


def _read_lines(path):
    with open(path) as f:
        return [ln.strip() for ln in f.readlines()]


# --- MG-RAST ----------------------------------------------------------------
def mgrast_to_spf(tsv_path, out_path):
    """Port of createProfileMgRastDlg (loadProfiles + createProfile)."""
    data = _read_lines(tsv_path)
    header = data[0].split('\t')
    if header[0].strip() != 'metagenome':
        raise ValueError("Not an MG-RAST profile (first column is not 'metagenome').")
    if 'abundance' not in header:
        raise ValueError("Not an MG-RAST profile (no 'abundance' column).")
    if header[1] not in ('level 1', 'source', 'domain'):
        raise ValueError("Not an MG-RAST profile (2nd column is not 'level 1', 'source', or 'domain').")

    start = 1 if header[1] in ('level 1', 'domain') else 2
    data_index = header.index('abundance')
    headings = header[start:data_index]

    sample_names = []
    for i in range(1, len(data)):
        sid = data[i].split('\t')[0]
        if sid not in sample_names:
            sample_names.append(sid)

    class Row:
        __slots__ = ('countData', 'hierarchy')

    profile = {}
    parent_map = {i: {} for i in range(1, data_index - start)}
    for i in range(1, len(data)):
        if data[i] == '':
            continue
        parts = data[i].split('\t')
        if len(parts) <= data_index:
            raise ValueError('Malformed MG-RAST row: too few columns.')
        count = int(parts[data_index])
        hierarchy = parts[start:data_index]
        # replace '-' categories with their parent
        for k in range(1, len(hierarchy)):
            if hierarchy[k] == '-':
                if header[1] == 'domain' and 'Unclassified' not in hierarchy[k - 1]:
                    hierarchy[k] = 'Unclassified ' + hierarchy[k - 1]
                else:
                    hierarchy[k] = hierarchy[k - 1]
        # force strictly tree-like (disambiguate a child with multiple parents)
        for k in range(1, len(hierarchy)):
            parent = '-'.join(hierarchy[0:k])
            child = hierarchy[k]
            plist = parent_map[k].get(child, [])
            if parent not in plist:
                plist.append(parent)
            parent_map[k][child] = plist
            idx = plist.index(parent)
            if idx != 0:
                hierarchy[k] = child + ' - #' + str(idx)

        sid = parts[0]
        col = sample_names.index(sid)
        row = profile.get(hierarchy[-1])
        if row is None:
            row = Row()
            row.countData = [0] * len(sample_names)
            row.hierarchy = hierarchy
            profile[hierarchy[-1]] = row
        row.countData[col] += count

    with open(out_path, 'w') as fout:
        out_headings = headings[:(data_index - start)]
        fout.write('\t'.join(out_headings + sample_names) + '\n')
        for key in profile:
            row = profile[key]
            fout.write('\t'.join(row.hierarchy) + '\t' + '\t'.join(str(c) for c in row.countData) + '\n')
    return out_headings


# --- Mothur -----------------------------------------------------------------
def mothur_to_spf(taxonomy_path, groups_path, out_path, names_path=None):
    """Port of createProfileMothurDlg.createProfile."""
    data = _read_lines(groups_path)
    seq_to_sample = {}
    sample_ids = set()
    for line in data:
        if not line:
            continue
        p = line.split('\t')
        seq_to_sample[p[0].strip()] = p[1].strip()
        sample_ids.add(p[1].strip())
    sample_ids = sorted(sample_ids)

    seq_to_seqs = None
    if names_path:
        seq_to_seqs = {}
        for line in _read_lines(names_path):
            if not line:
                continue
            p = line.split('\t')
            seq_to_seqs[p[0].strip()] = p[1].split(',')

    deepest = 0
    sample_profiles = {}
    for line in _read_lines(taxonomy_path):
        if not line:
            continue
        p = line.split('\t')
        seq_id = p[0]
        taxonomy = p[1].split(';')
        classification, depth = '', 0
        for t in taxonomy:
            if t.strip() != '':
                if t[-1] == ')':                       # strip trailing confidence score
                    t = t[0:t.rfind('(')]
                classification += t + '$'
                depth += 1
        deepest = max(deepest, depth)
        sample_profiles.setdefault(classification, {})
        seq_ids = seq_to_seqs[seq_id] if seq_to_seqs else [seq_id]
        for sid in seq_ids:
            sample = seq_to_sample[sid]
            sample_profiles[classification][sample] = sample_profiles[classification].get(sample, 0) + 1

    ranks = ['Level %d' % (i + 1) for i in range(max(deepest, 1))]
    with open(out_path, 'w') as fout:
        fout.write('\t'.join(ranks[:deepest]) + '\t' + '\t'.join(sample_ids) + '\n')
        for classification, counts in sample_profiles.items():
            classes = [c for c in classification.split('$') if c != '']
            classes += ['unclassified'] * (deepest - len(classes))
            row = classes + [str(counts.get(s, 0)) for s in sample_ids]
            fout.write('\t'.join(row) + '\n')
    return ranks[:deepest]


# --- CoMet ------------------------------------------------------------------
def comet_to_spf(file_paths, out_path):
    """Port of createProfileCoMetDlg.createProfile (one file per sample)."""
    profile = {}
    sample_names = []
    for idx, path in enumerate(file_paths):
        base = path[path.rfind('/') + 1:]
        sample = base[:base.find('.')] if '.' in base else base
        if '_' in sample:
            sample = sample[sample.find('_') + 1:]
        sample_names.append(sample)
        for line in _read_lines(path):
            if line == '':
                continue
            if line.find(' ') == -1 or line.rfind(':') == -1 or line.find('(') == -1 or line.rfind(')') == -1:
                raise ValueError('Not a valid CoMet profile line: ' + line[:40])
            category = line[line.find('(') + 1:line.rfind(')')].strip()
            if category == '':
                continue
            count = float(line[line.rfind(':') + 1:])
            row = profile.get(category)
            if row is None:
                row = [0] * len(file_paths)
                profile[category] = row
            profile[category][idx] += count

    with open(out_path, 'w') as fout:
        fout.write('Category\t' + '\t'.join(sample_names) + '\n')
        for category, counts in profile.items():
            fout.write(category + '\t' + '\t'.join(str(c) for c in counts) + '\n')
    return ['Category']


# --- RITA -------------------------------------------------------------------
RITA_METHODS = ['NB and D-BLASTN', 'D-BLASTN ratio', 'NB and BLASTN', 'BLASTN ratio',
                'NB and BLASTX', 'BLASTX ratio', 'NB ratio']


def rita_to_spf(file_paths, out_path, methods=None):
    """Port of createProfileRITADlg.createProfile (one classifier-output file per sample)."""
    methods = set(methods or RITA_METHODS)
    ranks = ['DOMAIN', 'PHYLUM', 'CLASS', 'ORDER', 'FAMILY', 'GENUS', 'SPECIES']
    out_ranks = ['Domain', 'Phylum', 'Class', 'Order', 'Family', 'Genus', 'Species']

    profile = {}
    sample_names = []
    deepest = 0
    for idx, path in enumerate(file_paths):
        data = _read_lines(path)
        base = path[path.rfind('/') + 1:]
        sample = base[:base.find('.')] if '.' in base else base
        if '_' in sample:
            sample = sample[sample.find('_') + 1:]
        sample_names.append(sample)
        for i in range(1, len(data)):
            line = data[i]
            if line == '':
                continue
            parts = line.split('\t')
            if len(parts) < 5 or parts[2] not in methods or parts[3] not in ranks:
                continue
            deepest = max(deepest, ranks.index(parts[3]))
            hierarchy = ';'.join(parts[4:])
            row = profile.get(hierarchy)
            if row is None:
                row = [0] * len(file_paths)
                profile[hierarchy] = row
            profile[hierarchy][idx] += 1

    if not profile:
        raise ValueError('No RITA classifications matched (check the file format / selected methods).')

    with open(out_path, 'w') as fout:
        fout.write('\t'.join(out_ranks[:deepest + 1]) + '\t' + '\t'.join(sample_names) + '\n')
        for hierarchy, counts in profile.items():
            levels = hierarchy.split(';')[::-1]                 # stored most-specific→domain
            levels += ['Unclassified'] * (deepest + 1 - len(levels))
            fout.write('\t'.join(levels) + '\t' + '\t'.join(str(c) for c in counts) + '\n')
    return out_ranks[:deepest + 1]


# --- Append COG categories (reuses STAMP's Qt-free COG_IO) -------------------
def append_cog(input_path, out_path, treatment, preferences):
    from stamp.metagenomics.fileIO.COG_IO import COG_IO
    COG_IO().appendCategories(input_path, treatment, out_path, preferences)
    return ['COG classes', 'COG category names', 'COG category codes', 'COG annotations', 'COG IDs']
