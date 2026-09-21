"""
STAMP-web proof-of-concept — FastAPI backend.

Demonstrates that STAMP's *existing* Python analysis core (stamp.metagenomics.*) can be
reused unchanged behind a web API. No Qt, no matplotlib: the backend loads a .spf profile +
metadata and runs the real STAMP plugins headlessly, returning plain JSON that the React
frontend renders with interactive Plotly.

Capabilities: two-group tests (Welch / Student / White) + multiple-comparison correction,
multi-group tests (ANOVA / Kruskal-Wallis), PCA ordination, and upload of your own data.

Run:  uvicorn app:app --reload --port 8000   (from webpoc/backend, with STAMP's .venv313)
"""
import os
import glob
import gzip
import math
import shutil
import tempfile
import uuid
from functools import lru_cache

import numpy as np
import scipy.cluster.hierarchy as _cluster
import scipy.spatial.distance as _dist
from typing import List

from fastapi import FastAPI, HTTPException, UploadFile, File, Form
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse, JSONResponse
from pydantic import BaseModel


def _sanitize(o):
    """Replace non-finite floats (inf/nan — e.g. some Holm-Bonferroni q-values) with None
    so every response is valid JSON, whatever a stats plugin returns."""
    if isinstance(o, float):
        return o if math.isfinite(o) else None
    if isinstance(o, dict):
        return {k: _sanitize(v) for k, v in o.items()}
    if isinstance(o, (list, tuple)):
        return [_sanitize(v) for v in o]
    return o


class SafeJSONResponse(JSONResponse):
    def render(self, content):
        return super().render(_sanitize(content))

# --- reuse STAMP's core unchanged -------------------------------------------
from stamp.metagenomics.fileIO.StampIO import StampIO
from stamp.metagenomics.fileIO.MetadataIO import MetadataIO
from stamp.metagenomics.stats.GroupStatsTests import GroupStatsTests
from stamp.metagenomics.stats.MultiGroupStatsTests import MultiGroupStatsTests
from stamp.metagenomics.stats.SampleStatsTests import SampleStatsTests
# two-sample statistical tests + CI method
from stamp.plugins.samples.statisticalTests.Fishers import Fishers
from stamp.plugins.samples.statisticalTests.GTest import GTest
from stamp.plugins.samples.statisticalTests.GTestYates import GTestYates
from stamp.plugins.samples.statisticalTests.GTestFisher import GTestFisher
from stamp.plugins.samples.statisticalTests.DiffBetweenProp import DiffBetweenProp
from stamp.plugins.samples.statisticalTests.ChiSquare import ChiSquare
from stamp.plugins.samples.statisticalTests.ChiSquareYates import ChiSquareYates
from stamp.plugins.samples.statisticalTests.Hypergeometric import Hypergeometric
from stamp.plugins.samples.statisticalTests.Bootstrap import Bootstrap
from stamp.plugins.samples.statisticalTests.Permutation import Permutation
from stamp.plugins.samples.confidenceIntervalMethods.DiffBetweenPropAsymptoticCC import DiffBetweenPropAsymptoticCC
# two-group statistical tests
from stamp.plugins.groups.statisticalTests.Welch import Welch
from stamp.plugins.groups.statisticalTests.Ttest import Ttest
from stamp.plugins.groups.statisticalTests.White import White
# multi-group statistical tests + effect size + post-hoc
from stamp.plugins.multiGroups.statisticalTests.ANOVA import ANOVA
from stamp.plugins.multiGroups.statisticalTests.KruskalWallis import KruskalWallis
from stamp.plugins.multiGroups.effectSizeFilters.EtaSquared import EtaSquared
from stamp.plugins.multiGroups.postHoc.TukeyKramer import TukeyKramer
from stamp.plugins.multiGroups.postHoc.GamesHowell import GamesHowell
from stamp.plugins.multiGroups.postHoc.Scheffe import Scheffe
from stamp.plugins.multiGroups.postHoc.WelchUncorrected import WelchUncorrected
# multiple-comparison correction plugins
from stamp.plugins.common.multipleComparisonCorrections.NoCorrection import NoCorrection
from stamp.plugins.common.multipleComparisonCorrections.BenjaminiHochbergFDR import BenjaminiHochbergFDR
from stamp.plugins.common.multipleComparisonCorrections.StoreyFDR import StoreyFDR
from stamp.plugins.common.multipleComparisonCorrections.Bonferroni import Bonferroni
from stamp.plugins.common.multipleComparisonCorrections.additional.HolmBonferroni import HolmBonferroni
from stamp.plugins.common.multipleComparisonCorrections.Sidak import Sidak

STAMP_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
EXAMPLES = os.path.join(STAMP_ROOT, "examples")

PREFERENCES = {
    'Pseudocount': 0.5,
    'Replicates': 1000,
    'Truncate feature names': False,
    'Length of truncated feature names': 50,
    'Minimum reported p-value exponent': -15,
}

SAMPLE_TESTS = {
    "Fisher's exact test": Fishers,
    "G-test (w/ Yates') + Fisher's": GTestFisher,
    "G-test": GTest,
    "G-test (w/ Yates')": GTestYates,
    "Difference between proportions": DiffBetweenProp,
    "Chi-square test": ChiSquare,
    "Chi-square test (w/ Yates')": ChiSquareYates,
    "Hypergeometric": Hypergeometric,
    "Bootstrap": Bootstrap,
    "Permutation": Permutation,
}
TESTS = {
    "Welch's t-test": Welch,
    "t-test (equal variance)": Ttest,
    "White's non-parametric t-test": White,
}
MG_TESTS = {
    "ANOVA": ANOVA,
    "Kruskal-Wallis H-test": KruskalWallis,
}
POSTHOC_TESTS = {
    "Tukey-Kramer": TukeyKramer,
    "Games-Howell": GamesHowell,
    "Scheffé": Scheffe,
    "Welch's (uncorrected)": WelchUncorrected,
}
CORRECTIONS = {
    "No correction": NoCorrection,
    "Benjamini-Hochberg FDR": BenjaminiHochbergFDR,
    "Storey FDR": StoreyFDR,
    "Bonferroni": Bonferroni,
    "Holm-Bonferroni": HolmBonferroni,
    "Šidák": Sidak,
}


class NullProgress:
    """Stand-in for the Qt QProgressDialog the desktop GUI passes to the stats engine."""
    def wasCanceled(self):
        return False
    def __getattr__(self, name):
        return lambda *a, **k: None


app = FastAPI(title="STAMP-web PoC", default_response_class=SafeJSONResponse)
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])


# --- dataset registry (bundled examples + uploads) ---------------------------
def discover_datasets():
    datasets = {}
    for spf in glob.glob(os.path.join(EXAMPLES, "**", "*.spf"), recursive=True):
        d = os.path.dirname(spf)
        metas = glob.glob(os.path.join(d, "*metadata*.tsv")) or glob.glob(os.path.join(d, "*.metadata.tsv"))
        if not metas:
            continue
        ds_id = os.path.relpath(spf, EXAMPLES)
        datasets[ds_id] = {"id": ds_id, "name": ds_id, "profile": spf, "metadata": metas[0], "uploaded": False}
    return dict(sorted(datasets.items()))


REGISTRY = discover_datasets()          # id -> {id, name, profile, metadata, uploaded}
_UPLOAD_ROOT = os.path.join(tempfile.gettempdir(), "stampweb_uploads")


@lru_cache(maxsize=16)
def load_dataset(ds_id: str):
    if ds_id not in REGISTRY:
        raise HTTPException(404, f"Unknown dataset: {ds_id}")
    ds = REGISTRY[ds_id]
    tree, err = StampIO(PREFERENCES).read(ds["profile"])
    if err:
        raise HTTPException(400, f"Failed to read profile: {err}")
    metadata = None
    if ds.get("metadata"):
        metadata, _warn = MetadataIO(PREFERENCES).read(ds["metadata"], tree)
        metadata.activeSamples = metadata.getSampleNames()
    return tree, metadata


def require_metadata(metadata):
    if metadata is None:
        raise HTTPException(400, "This analysis needs sample metadata — upload a metadata .tsv "
                                 "alongside the profile (two-sample mode works without it).")
    return metadata


# --- helpers -----------------------------------------------------------------
def usable_fields(tree, metadata):
    """Metadata fields that split the samples into >=2 groups."""
    out = []
    for field in metadata.getFeatures():
        metadata.setActiveField(field, tree)
        groups = {g: len(tree.groupDict[g]) for g in sorted(tree.groupDict.keys())}
        if len(groups) >= 2:
            out.append({"field": field, "groups": groups})
    return out


# --- API ---------------------------------------------------------------------
@app.get("/api/datasets")
def api_datasets():
    return [{"id": d["id"], "name": d["name"], "uploaded": d["uploaded"]} for d in REGISTRY.values()]


@app.get("/api/datasets/{ds_id:path}/schema")
def api_schema(ds_id: str):
    tree, metadata = load_dataset(ds_id)
    return {
        "id": ds_id,
        "levels": list(tree.hierarchyHeadings),
        "fields": usable_fields(tree, metadata) if metadata else [],
        "samples": sorted(tree.sampleNames),
        "tests": list(TESTS.keys()),
        "sampleTests": list(SAMPLE_TESTS.keys()),
        "mgTests": list(MG_TESTS.keys()),
        "postHocTests": list(POSTHOC_TESTS.keys()),
        "corrections": list(CORRECTIONS.keys()),
    }


class AnalyzeRequest(BaseModel):
    dataset: str
    field: str
    group1: str
    group2: str
    level: str
    test: str = "Welch's t-test"
    correction: str = "Benjamini-Hochberg FDR"
    coverage: float = 0.95


@app.post("/api/analyze")
def api_analyze(req: AnalyzeRequest):
    if req.test not in TESTS:
        raise HTTPException(400, f"Unknown test: {req.test}")
    if req.correction not in CORRECTIONS:
        raise HTTPException(400, f"Unknown correction: {req.correction}")
    tree, metadata = load_dataset(req.dataset)
    require_metadata(metadata)
    metadata.setActiveField(req.field, tree)
    if req.group1 not in tree.groupDict or req.group2 not in tree.groupDict:
        raise HTTPException(400, "Group not found in the selected field.")
    if req.level not in tree.hierarchyHeadings:
        raise HTTPException(400, "Unknown hierarchy level.")

    profile = tree.createGroupProfile(
        req.group1, req.group2, "Entire sample", req.level, metadata, "Retain unclassified reads")
    stats = GroupStatsTests(PREFERENCES)
    stats.run(TESTS[req.test](PREFERENCES), "Two-sided", "", req.coverage, profile, progress=NullProgress())
    stats.results.performMultCompCorrection(CORRECTIONS[req.correction](PREFERENCES))

    R = stats.results
    names = ["Features", "MeanRelFreq1", "MeanRelFreq2", "EffectSize",
             "pValues", "pValuesCorrected", "LowerCI", "UpperCI", "Note"]
    keys = ["feature", "mean1", "mean2", "effect", "pvalue", "corrected", "lowerCI", "upperCI", "note"]
    cols = {k: R.getColumn(n, False) for k, n in zip(keys, names)}
    rows = [{k: (cols[k][i] if k in ("feature", "note") else float(cols[k][i])) for k in keys}
            for i in range(len(cols["feature"]))]
    return {"mode": "two-group", "group1": req.group1, "group2": req.group2,
            "n1": len(profile.samplesInGroup1), "n2": len(profile.samplesInGroup2),
            "level": req.level, "test": req.test, "correction": req.correction,
            "count": len(rows), "rows": rows}


class SampleRequest(BaseModel):
    dataset: str
    sample1: str
    sample2: str
    level: str
    test: str = "Fisher's exact test"
    correction: str = "Benjamini-Hochberg FDR"
    coverage: float = 0.95


@app.post("/api/sample")
def api_sample(req: SampleRequest):
    if req.test not in SAMPLE_TESTS:
        raise HTTPException(400, f"Unknown test: {req.test}")
    if req.correction not in CORRECTIONS:
        raise HTTPException(400, f"Unknown correction: {req.correction}")
    tree, _metadata = load_dataset(req.dataset)
    if req.sample1 not in tree.sampleNames or req.sample2 not in tree.sampleNames:
        raise HTTPException(400, "Sample not found.")
    if req.sample1 == req.sample2:
        raise HTTPException(400, "Pick two different samples.")
    if req.level not in tree.hierarchyHeadings:
        raise HTTPException(400, "Unknown hierarchy level.")

    profile = tree.createSampleProfile(
        req.sample1, req.sample2, "Entire sample", req.level, "Retain unclassified reads")
    stats = SampleStatsTests(PREFERENCES)
    stats.run(SAMPLE_TESTS[req.test](PREFERENCES), "Two-sided", DiffBetweenPropAsymptoticCC(PREFERENCES),
              req.coverage, profile, progress=NullProgress())
    stats.results.performMultCompCorrection(CORRECTIONS[req.correction](PREFERENCES))

    R = stats.results
    names = ["Features", "RelFreq1", "RelFreq2", "EffectSize",
             "pValues", "pValuesCorrected", "LowerCI", "UpperCI", "Seq1", "Seq2", "Note"]
    keys = ["feature", "mean1", "mean2", "effect", "pvalue", "corrected", "lowerCI", "upperCI", "seq1", "seq2", "note"]
    cols = {k: R.getColumn(n, False) for k, n in zip(keys, names)}
    rows = [{k: (cols[k][i] if k in ("feature", "note") else float(cols[k][i])) for k in keys}
            for i in range(len(cols["feature"]))]
    rows.sort(key=lambda r: r["pvalue"])
    return {"mode": "two-sample", "group1": req.sample1, "group2": req.sample2, "n1": 1, "n2": 1,
            "level": req.level, "test": req.test, "correction": req.correction,
            "count": len(rows), "rows": rows}


class HeatmapRequest(BaseModel):
    dataset: str
    field: str
    level: str
    topN: int = 40


@app.post("/api/heatmap")
def api_heatmap(req: HeatmapRequest):
    tree, metadata = load_dataset(req.dataset)
    require_metadata(metadata)
    metadata.setActiveField(req.field, tree)
    if req.level not in tree.hierarchyHeadings:
        raise HTTPException(400, "Unknown hierarchy level.")

    profile = tree.createMultiGroupProfile(
        list(tree.groupDict.keys()), "Entire sample", req.level, metadata, "Retain unclassified reads")
    M = profile.getFeatureMatrix()          # samples x features (relative frequency)
    feat_names = list(profile.getFeatures())
    sample_names, sample_groups = [], []
    for gi, g in enumerate(profile.groupNames):
        for s in profile.samplesInGroups[gi]:
            sample_names.append(s)
            sample_groups.append(g)

    FM = M.T                                 # features x samples
    if FM.shape[0] < 2 or FM.shape[1] < 2:
        raise HTTPException(400, "Not enough features/samples for a heatmap.")
    # keep the most variable features so the map stays legible
    topN = max(5, min(req.topN, FM.shape[0]))
    keep = np.argsort(FM.var(axis=1))[::-1][:topN]
    Z = FM[keep]
    labels = [feat_names[i] for i in keep]

    # hierarchical clustering of rows (features) and columns (samples)
    row_order = _cluster.leaves_list(_cluster.linkage(_dist.pdist(Z), method="average"))
    col_order = _cluster.leaves_list(_cluster.linkage(_dist.pdist(Z.T), method="average"))
    Zr = Z[np.ix_(row_order, col_order)]

    return {"mode": "heatmap", "field": req.field, "level": req.level,
            "features": [labels[i] for i in row_order],
            "samples": [sample_names[i] for i in col_order],
            "sampleGroups": [sample_groups[i] for i in col_order],
            "groups": sorted(set(sample_groups)),
            "z": Zr.tolist()}


class MultiGroupRequest(BaseModel):
    dataset: str
    field: str
    level: str
    test: str = "ANOVA"
    correction: str = "Benjamini-Hochberg FDR"


@app.post("/api/multigroup")
def api_multigroup(req: MultiGroupRequest):
    if req.test not in MG_TESTS:
        raise HTTPException(400, f"Unknown test: {req.test}")
    tree, metadata = load_dataset(req.dataset)
    require_metadata(metadata)
    metadata.setActiveField(req.field, tree)
    if req.level not in tree.hierarchyHeadings:
        raise HTTPException(400, "Unknown hierarchy level.")

    profile = tree.createMultiGroupProfile(
        list(tree.groupDict.keys()), "Entire sample", req.level, metadata, "Retain unclassified reads")
    profile.setActiveGroups(tree.groupActive)
    groups = list(profile.activeGroupNames)
    if len(groups) < 2:
        raise HTTPException(400, "Need at least two groups.")

    stats = MultiGroupStatsTests(PREFERENCES)
    stats.run(MG_TESTS[req.test](PREFERENCES), EtaSquared(PREFERENCES), profile, progress=NullProgress())
    stats.results.performMultCompCorrection(CORRECTIONS[req.correction](PREFERENCES))
    R = stats.results
    feats = R.getColumn("Features", False)
    pv = R.getColumn("pValues", False)
    pvc = R.getColumn("pValuesCorrected", False)
    eff = R.getColumn("EffectSize", False)

    order = sorted(range(len(feats)), key=lambda i: pv[i])
    rows = []
    for i in order:
        f = feats[i]
        fc = profile.getActiveFeatureCounts(f)
        pc = profile.getActiveParentCounts(f)
        gmeans = []
        for gi in range(len(groups)):
            props = [fc[gi][j] * 100.0 / pc[gi][j] if pc[gi][j] > 0 else 0.0
                     for j in range(len(fc[gi]))]
            gmeans.append(float(np.mean(props)) if props else 0.0)
        rows.append({"feature": f, "pvalue": float(pv[i]), "corrected": float(pvc[i]),
                     "effect": float(eff[i]), "groupMeans": gmeans})
    return {"mode": "multi-group", "field": req.field, "level": req.level,
            "test": req.test, "correction": req.correction,
            "groups": groups, "groupSizes": [len(g) for g in profile.activeSamplesInGroups],
            "count": len(rows), "rows": rows}


class DistributionRequest(BaseModel):
    dataset: str
    field: str
    level: str
    feature: str


@app.post("/api/distribution")
def api_distribution(req: DistributionRequest):
    """Per-group, per-sample relative frequency (%) of one feature — for a box plot."""
    tree, metadata = load_dataset(req.dataset)
    require_metadata(metadata)
    metadata.setActiveField(req.field, tree)
    if req.level not in tree.hierarchyHeadings:
        raise HTTPException(400, "Unknown hierarchy level.")

    profile = tree.createMultiGroupProfile(
        list(tree.groupDict.keys()), "Entire sample", req.level, metadata, "Retain unclassified reads")
    profile.setActiveGroups(tree.groupActive)
    if req.feature not in profile.getFeatures():
        raise HTTPException(400, "Feature not found at this level.")

    fc = profile.getActiveFeatureCounts(req.feature)
    pc = profile.getActiveParentCounts(req.feature)
    groups = list(profile.activeGroupNames)
    data = []
    for gi in range(len(groups)):
        vals = [fc[gi][j] * 100.0 / pc[gi][j] if pc[gi][j] > 0 else 0.0 for j in range(len(fc[gi]))]
        data.append({"group": groups[gi], "samples": list(profile.activeSamplesInGroups[gi]), "values": vals})
    return {"mode": "distribution", "feature": req.feature, "field": req.field, "level": req.level,
            "groups": groups, "data": data}


class PostHocRequest(BaseModel):
    dataset: str
    field: str
    level: str
    feature: str
    test: str = "Tukey-Kramer"
    coverage: float = 0.95


@app.post("/api/posthoc")
def api_posthoc(req: PostHocRequest):
    if req.test not in POSTHOC_TESTS:
        raise HTTPException(400, f"Unknown post-hoc test: {req.test}")
    tree, metadata = load_dataset(req.dataset)
    require_metadata(metadata)
    metadata.setActiveField(req.field, tree)
    if req.level not in tree.hierarchyHeadings:
        raise HTTPException(400, "Unknown hierarchy level.")

    profile = tree.createMultiGroupProfile(
        list(tree.groupDict.keys()), "Entire sample", req.level, metadata, "Retain unclassified reads")
    profile.setActiveGroups(tree.groupActive)
    if req.feature not in profile.getFeatures():
        raise HTTPException(400, "Feature not found at this level.")

    data = profile.getActiveFeatureProportions(req.feature)
    pValues, effect, lowerCI, upperCI, labels, note = POSTHOC_TESTS[req.test](PREFERENCES).run(
        data, req.coverage, profile.activeGroupNames)

    rows = [{"comparison": labels[i], "effect": float(effect[i]),
             "lowerCI": float(lowerCI[i]), "upperCI": float(upperCI[i]),
             "pvalue": str(pValues[i])}
            for i in range(len(labels))]
    return {"mode": "posthoc", "feature": req.feature, "field": req.field, "level": req.level,
            "test": req.test, "groups": list(profile.activeGroupNames), "note": note, "rows": rows}


class PCARequest(BaseModel):
    dataset: str
    field: str
    level: str


@app.post("/api/pca")
def api_pca(req: PCARequest):
    tree, metadata = load_dataset(req.dataset)
    require_metadata(metadata)
    metadata.setActiveField(req.field, tree)
    if req.level not in tree.hierarchyHeadings:
        raise HTTPException(400, "Unknown hierarchy level.")

    profile = tree.createMultiGroupProfile(
        list(tree.groupDict.keys()), "Entire sample", req.level, metadata, "Retain unclassified reads")
    M = profile.getFeatureMatrix()          # samples x features (relative frequencies)
    names, groups = [], []
    for gi, g in enumerate(profile.groupNames):
        for s in profile.samplesInGroups[gi]:
            names.append(s)
            groups.append(g)

    if M.shape[0] < 3 or M.shape[1] < 2:
        raise HTTPException(400, "Not enough samples/features for a PCA.")
    Mc = M - M.mean(axis=0)
    U, S, _Vt = np.linalg.svd(Mc, full_matrices=False)
    scores = U * S
    var = (S ** 2) / float(np.sum(S ** 2)) * 100.0
    points = [{"name": names[i], "group": groups[i],
               "pc1": float(scores[i, 0]), "pc2": float(scores[i, 1])}
              for i in range(len(names))]
    return {"mode": "pca", "field": req.field, "level": req.level,
            "groups": sorted(set(groups)), "varPC1": float(var[0]), "varPC2": float(var[1]),
            "points": points}


def _is_number(s):
    try:
        float(s)
        return True
    except (TypeError, ValueError):
        return False


def _load_biom(biom_path):
    from biom.parse import parse_biom_table
    if biom_path.endswith(".gz"):
        return parse_biom_table(gzip.open(biom_path, "rb"))
    try:                                       # HDF5 (BIOM 2.x) or JSON (BIOM 1.0)
        import biom
        return biom.load_table(biom_path)
    except Exception:
        return parse_biom_table(open(biom_path))


def biom_to_spf(biom_path, out_path, force_flat=False):
    """Convert a BIOM table to a STAMP .spf profile (headless port of STAMP's
    createProfileBiomDlg.convertBiomFileToStampProfile — no Qt).

    Uses clean 'taxonomy'-style observation metadata as the hierarchy when present; otherwise
    (or when force_flat) a flat profile keyed by observation id. Returns the heading names used.
    """
    table = _load_biom(biom_path)
    obs_md = table.metadata(axis="observation")
    metadata_name = None
    if not force_flat and obs_md is not None and len(obs_md) and obs_md[0] is not None:
        # only flat string-list lineages — nested KEGG-style metadata isn't a strict hierarchy
        for cand in ("taxonomy", "Taxonomy", "Consensus Lineage"):
            if cand in obs_md[0]:
                metadata_name = cand
                break
    max_len = 0
    if metadata_name is not None:
        max_len = max(len(p.get(metadata_name, [])) for p in obs_md)

    def clean(x):
        s = str(x).strip()
        return s if s else "unclassified"

    headers = [f"Level_{i + 1}" for i in range(max_len)] + ["Observation Ids"]
    headers.extend(str(s) for s in table.ids(axis="sample"))
    with open(out_path, "w") as fout:
        fout.write("\t".join(headers) + "\n")
        for obs_vals, obs_id, obs_metadata in table.iter(axis="observation"):
            row = [clean(x) for x in obs_metadata.get(metadata_name, [])] if max_len else []
            row += ["unclassified"] * (max_len - len(row))
            row.append("ID" + str(obs_id) if _is_number(obs_id) else str(obs_id))
            row.extend(str(int(v)) if float(v).is_integer() else str(v) for v in obs_vals)
            fout.write("\t".join(row) + "\n")
    return headers[:max_len + 1]


@app.post("/api/import/biom")
async def api_import_biom(biom: UploadFile = File(...), metadata: UploadFile = File(None)):
    """Upload a BIOM table (+ optional STAMP metadata .tsv), convert it to a profile, and
    register it as a dataset usable in every mode."""
    os.makedirs(_UPLOAD_ROOT, exist_ok=True)
    d = tempfile.mkdtemp(prefix="biom_", dir=_UPLOAD_ROOT)
    biom_path = os.path.join(d, os.path.basename(biom.filename) or "table.biom")
    with open(biom_path, "wb") as f:
        shutil.copyfileobj(biom.file, f)
    spf_path = os.path.join(d, "converted.spf")
    try:
        biom_to_spf(biom_path, spf_path)
        tree, err = StampIO(PREFERENCES).read(spf_path)
        if err:                                 # taxonomy hierarchy isn't strict → flat profile
            biom_to_spf(biom_path, spf_path, force_flat=True)
            tree, err = StampIO(PREFERENCES).read(spf_path)
    except Exception as e:
        shutil.rmtree(d, ignore_errors=True)
        raise HTTPException(400, f"Could not convert BIOM: {e}")
    if err:
        shutil.rmtree(d, ignore_errors=True)
        raise HTTPException(400, f"Converted profile did not validate: {err}")

    meta_path = None
    if metadata is not None and metadata.filename:
        meta_path = os.path.join(d, os.path.basename(metadata.filename))
        with open(meta_path, "wb") as f:
            shutil.copyfileobj(metadata.file, f)

    ds_id = "upload/" + os.path.basename(d)
    REGISTRY[ds_id] = {"id": ds_id, "name": "⬆ " + (biom.filename or "table.biom") + " (BIOM)",
                       "profile": spf_path, "metadata": meta_path, "uploaded": True}
    return {"id": ds_id, "name": REGISTRY[ds_id]["name"], "hasMetadata": meta_path is not None,
            "levels": list(tree.hierarchyHeadings)}


IMPORTER_LABELS = {"mgrast": "MG-RAST", "mothur": "Mothur", "comet": "CoMet", "rita": "RITA", "cog": "COG"}


@app.post("/api/import/{kind}")
async def api_import(kind: str, files: List[UploadFile] = File(...),
                     metadata: UploadFile = File(None),
                     treatment: str = Form("Treat multi-code COGs as features")):
    """Convert a raw STAMP-supported format (MG-RAST / Mothur / CoMet / RITA / Append-COG) to a
    profile and register it. Mothur takes several files (identified by extension); CoMet and
    RITA take one file per sample; MG-RAST and COG take a single file."""
    import importers as IMP
    if kind not in IMPORTER_LABELS:
        raise HTTPException(404, f"Unknown importer: {kind}")
    os.makedirs(_UPLOAD_ROOT, exist_ok=True)
    d = tempfile.mkdtemp(prefix=f"{kind}_", dir=_UPLOAD_ROOT)
    saved = []
    for uf in files:
        p = os.path.join(d, os.path.basename(uf.filename) or "file")
        with open(p, "wb") as f:
            shutil.copyfileobj(uf.file, f)
        saved.append(p)

    spf = os.path.join(d, "converted.spf")
    try:
        if kind == "mgrast":
            IMP.mgrast_to_spf(saved[0], spf)
        elif kind == "cog":
            IMP.append_cog(saved[0], spf, treatment, PREFERENCES)
        elif kind == "comet":
            IMP.comet_to_spf(saved, spf)
        elif kind == "rita":
            IMP.rita_to_spf(saved, spf)
        elif kind == "mothur":
            by_ext = lambda e: next((p for p in saved if p.endswith(e)), None)
            tax, grp, nms = by_ext(".taxonomy"), by_ext(".groups"), by_ext(".names")
            if not tax or not grp:
                raise ValueError("Mothur import needs a .taxonomy and a .groups file (.names optional).")
            IMP.mothur_to_spf(tax, grp, spf, nms)
        tree, err = StampIO(PREFERENCES).read(spf)
    except Exception as e:
        shutil.rmtree(d, ignore_errors=True)
        raise HTTPException(400, f"Import failed: {e}")
    if err:
        shutil.rmtree(d, ignore_errors=True)
        raise HTTPException(400, f"Converted profile did not validate: {err}")

    meta_path = None
    if metadata is not None and metadata.filename:
        meta_path = os.path.join(d, os.path.basename(metadata.filename))
        with open(meta_path, "wb") as f:
            shutil.copyfileobj(metadata.file, f)

    ds_id = "upload/" + os.path.basename(d)
    REGISTRY[ds_id] = {"id": ds_id, "name": f"⬆ {files[0].filename} ({IMPORTER_LABELS[kind]})",
                       "profile": spf, "metadata": meta_path, "uploaded": True}
    return {"id": ds_id, "name": REGISTRY[ds_id]["name"], "hasMetadata": meta_path is not None,
            "levels": list(tree.hierarchyHeadings)}


@app.post("/api/upload")
async def api_upload(profile: UploadFile = File(...), metadata: UploadFile = File(None)):
    """Store an uploaded .spf (+ optional metadata .tsv) and register it as a dataset."""
    os.makedirs(_UPLOAD_ROOT, exist_ok=True)
    d = tempfile.mkdtemp(prefix="ds_", dir=_UPLOAD_ROOT)
    prof_path = os.path.join(d, os.path.basename(profile.filename) or "profile.spf")
    with open(prof_path, "wb") as f:
        shutil.copyfileobj(profile.file, f)
    meta_path = None
    if metadata is not None and metadata.filename:
        meta_path = os.path.join(d, os.path.basename(metadata.filename))
        with open(meta_path, "wb") as f:
            shutil.copyfileobj(metadata.file, f)

    # validate it parses before registering
    tree, err = StampIO(PREFERENCES).read(prof_path)
    if err:
        shutil.rmtree(d, ignore_errors=True)
        raise HTTPException(400, f"Could not parse profile: {err}")

    ds_id = "upload/" + os.path.basename(d)
    REGISTRY[ds_id] = {"id": ds_id, "name": "⬆ " + (profile.filename or "uploaded.spf"),
                       "profile": prof_path, "metadata": meta_path, "uploaded": True}
    return {"id": ds_id, "name": REGISTRY[ds_id]["name"], "hasMetadata": meta_path is not None,
            "levels": list(tree.hierarchyHeadings)}


# --- serve built frontend (single-server mode) -------------------------------
_DIST = os.path.join(os.path.dirname(__file__), "..", "frontend", "dist")
if os.path.isdir(_DIST):
    app.mount("/assets", StaticFiles(directory=os.path.join(_DIST, "assets")), name="assets")

    @app.get("/")
    def index():
        return FileResponse(os.path.join(_DIST, "index.html"))
