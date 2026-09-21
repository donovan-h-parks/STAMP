import React, { useEffect, useMemo, useRef, useState } from 'react'
import Plot from 'react-plotly.js'

const G1 = '#2b6cb0'   // higher in group/sample 1
const G2 = '#dd6b20'   // higher in group/sample 2
const PALETTE = ['#2b6cb0', '#dd6b20', '#38a169', '#805ad5', '#d53f8c', '#319795', '#b7791f', '#e53e3e', '#4a5568']

// null-safe number formatting (the backend sends null for non-finite stats, e.g. some q-values)
const nf = (x, d = 3) => (x == null || Number.isNaN(x) ? '—' : x.toFixed(d))
const ne = x => (x == null || Number.isNaN(x) ? '—' : x.toExponential(2))
const half = (a, b) => (a == null || b == null ? 0 : (a - b) / 2)

function pvalueHistogram(rows, key = 'pvalue') {
  const vals = rows.map(r => r[key]).filter(v => v != null && !Number.isNaN(v))
  return {
    data: [{ type: 'histogram', x: vals, xbins: { start: 0, end: 1, size: 0.05 }, marker: { color: '#2b6cb0' } }],
    layout: {
      margin: { l: 54, r: 20, t: 10, b: 44 }, height: 340, bargap: 0.03,
      xaxis: { title: `${key === 'corrected' ? 'corrected q' : 'p'}-value`, range: [0, 1] },
      yaxis: { title: 'features' }, font: { family: 'inherit' },
    },
    config: { displaylogo: false, responsive: true },
  }
}

const jget = p => fetch(p).then(r => r.ok ? r.json() : r.json().then(e => Promise.reject(e)))
const jpost = (p, body) => fetch(p, {
  method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
}).then(r => r.ok ? r.json() : r.json().then(e => Promise.reject(e)))

const MODES = [['sample', 'Two samples'], ['two', 'Two groups'], ['multi', 'Multi-group'], ['pca', 'PCA'], ['heatmap', 'Heatmap']]

// data-import formats: single- or multi-file, each posting to its endpoint under `field`
const IMPORTERS = [
  { id: 'spf', label: 'STAMP profile (.spf)', url: '/api/upload', field: 'profile', accept: '.spf', multiple: false, help: 'A ready-made STAMP profile.' },
  { id: 'biom', label: 'BIOM table', url: '/api/import/biom', field: 'biom', accept: '.biom,.gz', multiple: false, help: 'Taxonomy hierarchy when present, else flat.' },
  { id: 'mgrast', label: 'MG-RAST profile', url: '/api/import/mgrast', field: 'files', accept: '.tsv,.txt', multiple: false, help: 'One MG-RAST .tsv (function or phylogeny).' },
  { id: 'mothur', label: 'Mothur', url: '/api/import/mothur', field: 'files', accept: '.taxonomy,.groups,.names', multiple: true, help: 'Select the .taxonomy and .groups files (.names optional).' },
  { id: 'comet', label: 'CoMet profiles', url: '/api/import/comet', field: 'files', accept: '.txt', multiple: true, help: 'One CoMet file per sample — select them all.' },
  { id: 'rita', label: 'RITA profiles', url: '/api/import/rita', field: 'files', accept: '.txt', multiple: true, help: 'One RITA classifier file per sample — select them all.' },
  { id: 'cog', label: 'Append COG categories', url: '/api/import/cog', field: 'files', accept: '.tsv,.txt', multiple: false, help: 'An IMG/M COG profile — categories are appended.' },
]

function Field({ label, children, hint }) {
  return <><label>{label}</label>{children}{hint && <div className="hint">{hint}</div>}</>
}
function Sel({ value, onChange, options }) {
  return <select value={value} onChange={e => onChange(e.target.value)}>
    {options.map(o => <option key={o} value={o}>{o}</option>)}
  </select>
}

export default function App() {
  const [datasets, setDatasets] = useState([])
  const [dataset, setDataset] = useState('')
  const [schema, setSchema] = useState(null)
  const [mode, setMode] = useState('two')

  const [field, setField] = useState('')
  const [level, setLevel] = useState('')
  // two-group
  const [group1, setGroup1] = useState(''); const [group2, setGroup2] = useState('')
  const [test, setTest] = useState(''); const [correction, setCorrection] = useState('')
  const [topN, setTopN] = useState(25)
  const [alpha, setAlpha] = useState(0.05)
  const [useCorrected, setUseCorrected] = useState(false)
  const [minEffect, setMinEffect] = useState(0)
  const [sigOnly, setSigOnly] = useState(false)
  // two-sample
  const [sample1, setSample1] = useState(''); const [sample2, setSample2] = useState('')
  const [sampleTest, setSampleTest] = useState('')
  // multi-group
  const [mgTest, setMgTest] = useState(''); const [mgCorrection, setMgCorrection] = useState('')
  // post-hoc (within multi-group)
  const [phTest, setPhTest] = useState(''); const [phFeature, setPhFeature] = useState('')
  const [phResult, setPhResult] = useState(null); const [phBusy, setPhBusy] = useState(false)
  // heatmap
  const [hmTopN, setHmTopN] = useState(40)
  // plot-type switchers + box-plot distribution
  const [pairPlotKind, setPairPlotKind] = useState('effect')   // effect | scatter | pvalue
  const [mgPlotKind, setMgPlotKind] = useState('bar')          // bar | box | pvalue
  const [boxFeature, setBoxFeature] = useState('')
  const [boxResult, setBoxResult] = useState(null); const [boxBusy, setBoxBusy] = useState(false)

  const [result, setResult] = useState(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(null)
  const fileRef = useRef(null), upMeta = useRef(null)
  const [importerKind, setImporterKind] = useState('spf')
  const [cogTreatment, setCogTreatment] = useState('Treat multi-code COGs as features')

  useEffect(() => {
    jget('/api/datasets').then(d => {
      setDatasets(d)
      const pref = d.find(x => x.id.includes('Enterotypes')) || d[0]
      if (pref) setDataset(pref.id)
    }).catch(e => setError(e.detail || String(e)))
  }, [])

  useEffect(() => {
    if (!dataset) return
    setSchema(null); setResult(null); setError(null)
    jget(`/api/datasets/${dataset}/schema`).then(s => {
      setSchema(s)
      setLevel(s.levels[s.levels.length - 1] || '')
      setTest(s.tests[0] || ''); setCorrection(s.corrections[1] || s.corrections[0] || '')
      setMgTest(s.mgTests[0] || ''); setMgCorrection(s.corrections[1] || s.corrections[0] || '')
      setPhTest(s.postHocTests?.[0] || '')
      setSampleTest(s.sampleTests[0] || '')
      if (s.samples?.length >= 2) { setSample1(s.samples[0]); setSample2(s.samples[1]) }
      if (s.fields[0]) setField(s.fields[0].field)
      else setMode('sample')   // no metadata → only two-sample analysis is possible
    }).catch(e => setError(e.detail || String(e)))
  }, [dataset])

  const fieldObj = useMemo(() => schema?.fields.find(f => f.field === field), [schema, field])
  const groupNames = useMemo(() => fieldObj ? Object.keys(fieldObj.groups) : [], [fieldObj])
  useEffect(() => {
    if (groupNames.length >= 2) { setGroup1(groupNames[0]); setGroup2(groupNames[1]) }
  }, [field, schema]) // eslint-disable-line

  async function refreshDatasets(selectId) {
    const d = await jget('/api/datasets'); setDatasets(d); if (selectId) setDataset(selectId)
  }
  const importer = IMPORTERS.find(i => i.id === importerKind)
  async function doImport() {
    const fs = fileRef.current?.files
    if (!fs || !fs.length) { setError('Choose file(s) to import.'); return }
    setBusy(true); setError(null)
    try {
      const fd = new FormData()
      for (const f of fs) fd.append(importer.field, f)   // single or multiple, same field
      if (upMeta.current?.files?.[0]) fd.append('metadata', upMeta.current.files[0])
      if (importer.id === 'cog') fd.append('treatment', cogTreatment)
      const r = await fetch(importer.url, { method: 'POST', body: fd })
      const j = await r.json(); if (!r.ok) throw j
      await refreshDatasets(j.id)
    } catch (e) { setError(e.detail || String(e)) }
    setBusy(false)
  }

  async function run() {
    setBusy(true); setError(null)
    try {
      let r
      if (mode === 'two') r = await jpost('/api/analyze', { dataset, field, group1, group2, level, test, correction })
      else if (mode === 'sample') r = await jpost('/api/sample', { dataset, sample1, sample2, level, test: sampleTest, correction })
      else if (mode === 'multi') r = await jpost('/api/multigroup', { dataset, field, level, test: mgTest, correction: mgCorrection })
      else if (mode === 'pca') r = await jpost('/api/pca', { dataset, field, level })
      else r = await jpost('/api/heatmap', { dataset, field, level, topN: hmTopN })
      setResult(r)
    } catch (e) { setError(e.detail || String(e)); setResult(null) }
    setBusy(false)
  }

  // reset post-hoc & box when a new analysis runs; default the feature to the top multi-group hit
  useEffect(() => {
    setPhResult(null); setBoxResult(null); setPairPlotKind('effect'); setMgPlotKind('bar')
    if (result?.mode === 'multi-group' && result.rows.length) {
      setPhFeature(result.rows[0].feature); setBoxFeature(result.rows[0].feature)
    }
  }, [result])

  async function runPostHoc() {
    setPhBusy(true); setError(null)
    try {
      const r = await jpost('/api/posthoc', { dataset, field, level, feature: phFeature, test: phTest })
      setPhResult(r)
    } catch (e) { setError(e.detail || String(e)) }
    setPhBusy(false)
  }

  const phPlot = useMemo(() => {
    if (!phResult) return null
    const rows = phResult.rows.slice().reverse()
    return {
      data: [{
        type: 'scatter', mode: 'markers', x: rows.map(r => r.effect), y: rows.map(r => r.comparison),
        marker: { color: rows.map(r => r.effect >= 0 ? G1 : G2), size: 9 },
        error_x: { type: 'data', array: rows.map(r => half(r.upperCI, r.lowerCI)), thickness: 1.4, width: 4, color: '#9aa5b1' },
        hovertext: rows.map(r => `${r.comparison}<br>diff: ${nf(r.effect)} (CI ${nf(r.lowerCI, 2)}…${nf(r.upperCI, 2)})<br>p ${r.pvalue}`),
        hoverinfo: 'text',
      }],
      layout: {
        margin: { l: 150, r: 20, t: 10, b: 44 }, height: Math.max(240, rows.length * 24 + 60),
        xaxis: { title: `Difference between groups — ${phResult.feature}` },
        yaxis: { automargin: true, tickfont: { size: 11 } },
        shapes: [{ type: 'line', x0: 0, x1: 0, yref: 'paper', y0: 0, y1: 1, line: { color: '#b0b7c0', width: 1 } }],
        showlegend: false, font: { family: 'inherit' },
      },
      config: { displaylogo: false, responsive: true, modeBarButtonsToRemove: ['lasso2d', 'select2d'] },
    }
  }, [phResult])

  const isPair = result && (result.mode === 'two-group' || result.mode === 'two-sample')
  const isSample = result?.mode === 'two-sample'

  // ---- two-group / two-sample filtering + plot ------------------------------
  const pairFiltered = useMemo(() => {
    if (!isPair) return []
    let rows = result.rows
    if (sigOnly) rows = rows.filter(r => (useCorrected ? r.corrected : r.pvalue) < alpha)
    if (minEffect > 0) rows = rows.filter(r => Math.abs(r.effect) >= minEffect)
    return rows
  }, [result, isPair, sigOnly, useCorrected, alpha, minEffect])

  const pairPlot = useMemo(() => {
    if (!isPair) return null
    const top = pairFiltered.slice(0, topN).slice().reverse()
    if (!top.length) return { empty: true }
    const unit = isSample ? 'proportion' : 'mean proportion'
    return {
      data: [{
        type: 'scatter', mode: 'markers', x: top.map(r => r.effect), y: top.map(r => r.feature),
        marker: { color: top.map(r => r.effect >= 0 ? G1 : G2), size: 9 },
        error_x: { type: 'data', array: top.map(r => half(r.upperCI, r.lowerCI)), thickness: 1.4, width: 4, color: '#9aa5b1' },
        hovertext: top.map(r => `${r.feature}<br>${result.group1}: ${nf(r.mean1)}%<br>${result.group2}: ${nf(r.mean2)}%<br>diff: ${nf(r.effect)}% (95% CI ${nf(r.lowerCI, 2)}…${nf(r.upperCI, 2)})<br>p = ${ne(r.pvalue)}   q = ${ne(r.corrected)}`),
        hoverinfo: 'text',
      }],
      layout: {
        margin: { l: 175, r: 20, t: 10, b: 44 }, height: Math.max(300, top.length * 22 + 60),
        xaxis: { title: `Difference in ${unit} (%) — ${result.group1} vs ${result.group2}` },
        yaxis: { automargin: true, tickfont: { size: 11 } },
        shapes: [{ type: 'line', x0: 0, x1: 0, yref: 'paper', y0: 0, y1: 1, line: { color: '#b0b7c0', width: 1 } }],
        showlegend: false, font: { family: 'inherit' },
      },
      config: { displaylogo: false, responsive: true, modeBarButtonsToRemove: ['lasso2d', 'select2d'] },
    }
  }, [result, isPair, isSample, pairFiltered, topN])

  // scatter: group/sample 1 vs 2 mean proportion, coloured by significance
  const pairScatter = useMemo(() => {
    if (!isPair) return null
    const rows = result.rows
    const mk = (rs, color, name) => ({
      type: 'scatter', mode: 'markers', name,
      x: rs.map(r => r.mean1), y: rs.map(r => r.mean2),
      text: rs.map(r => `${r.feature}<br>${result.group1}: ${nf(r.mean1)}%<br>${result.group2}: ${nf(r.mean2)}%<br>p = ${ne(r.pvalue)}`),
      hoverinfo: 'text', marker: { color, size: 6, opacity: 0.75 },
    })
    const sig = rows.filter(r => r.pvalue < 0.05), ns = rows.filter(r => !(r.pvalue < 0.05))
    const mx = Math.max(1, ...rows.map(r => Math.max(r.mean1 || 0, r.mean2 || 0)))
    return {
      data: [mk(ns, '#b0b7c0', 'n.s.'), mk(sig, '#2b6cb0', 'p < 0.05')],
      layout: {
        margin: { l: 58, r: 20, t: 10, b: 50 }, height: 460,
        xaxis: { title: `${result.group1} — ${isSample ? 'proportion' : 'mean proportion'} (%)` },
        yaxis: { title: `${result.group2} — ${isSample ? 'proportion' : 'mean proportion'} (%)`, scaleanchor: 'x' },
        shapes: [{ type: 'line', x0: 0, y0: 0, x1: mx, y1: mx, line: { color: '#cbd2d9', dash: 'dot' } }],
        legend: { orientation: 'h', y: 1.05 }, font: { family: 'inherit' },
      },
      config: { displaylogo: false, responsive: true },
    }
  }, [result, isPair, isSample])

  // profile bar: top features as a paired bar of the two groups'/samples' proportions
  const profileBar = useMemo(() => {
    if (!isPair) return null
    const top = result.rows.slice()
      .sort((a, b) => Math.max(b.mean1 || 0, b.mean2 || 0) - Math.max(a.mean1 || 0, a.mean2 || 0))
      .slice(0, 20).reverse()
    const bar = (name, key, color) => ({
      type: 'bar', orientation: 'h', name, marker: { color },
      y: top.map(r => r.feature), x: top.map(r => r[key]),
      hovertemplate: `${name}: %{x:.3f}%<extra>%{y}</extra>`,
    })
    return {
      data: [bar(result.group1, 'mean1', G1), bar(result.group2, 'mean2', G2)],
      layout: {
        barmode: 'group', margin: { l: 175, r: 20, t: 10, b: 44 }, height: Math.max(320, top.length * 30 + 80),
        xaxis: { title: `${isSample ? 'Proportion' : 'Mean proportion'} (%)` },
        yaxis: { automargin: true, tickfont: { size: 11 } }, legend: { orientation: 'h', y: 1.05 }, font: { family: 'inherit' },
      },
      config: { displaylogo: false, responsive: true },
    }
  }, [result, isPair, isSample])

  // multiple-comparison plots: raw-vs-corrected scatter + cumulative significant count
  const multComp = useMemo(() => {
    if (!isPair) return null
    const rows = result.rows.filter(r => r.pvalue != null)
    const raw = rows.map(r => r.pvalue), corr = rows.map(r => r.corrected)
    const sorted = raw.slice().sort((a, b) => a - b)
    const cum = sorted.map((_v, i) => i + 1)
    return {
      scatter: {
        data: [{ type: 'scatter', mode: 'markers', x: raw, y: corr, marker: { color: '#7f7f7f', size: 5, opacity: 0.7 },
          hovertemplate: 'raw p %{x:.2e}<br>corrected q %{y:.2e}<extra></extra>' }],
        layout: { margin: { l: 60, r: 16, t: 10, b: 44 }, height: 300,
          xaxis: { title: 'raw p-value' }, yaxis: { title: 'corrected q-value' },
          shapes: [{ type: 'line', x0: 0, y0: 0, x1: 1, y1: 1, line: { color: '#cbd2d9', dash: 'dot' } }], font: { family: 'inherit' } },
        config: { displaylogo: false, responsive: true },
      },
      cumulative: {
        data: [{ type: 'scatter', mode: 'lines', x: sorted, y: cum, line: { color: '#2b6cb0' } }],
        layout: { margin: { l: 60, r: 16, t: 10, b: 44 }, height: 300,
          xaxis: { title: 'p-value threshold', range: [0, Math.min(1, (sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * 0.5))] || 0.1) * 2 + 0.05)] },
          yaxis: { title: '# features below threshold' }, font: { family: 'inherit' } },
        config: { displaylogo: false, responsive: true },
      },
    }
  }, [result, isPair])

  // sequence histogram (two-sample): distribution of per-feature sequence counts
  const seqHist = useMemo(() => {
    if (result?.mode !== 'two-sample') return null
    return {
      data: [
        { type: 'histogram', name: result.group1, x: result.rows.map(r => r.seq1), opacity: 0.6, marker: { color: G1 } },
        { type: 'histogram', name: result.group2, x: result.rows.map(r => r.seq2), opacity: 0.6, marker: { color: G2 } },
      ],
      layout: { barmode: 'overlay', margin: { l: 58, r: 20, t: 10, b: 44 }, height: 360,
        xaxis: { title: 'Sequences per feature' }, yaxis: { title: 'Number of features' },
        legend: { orientation: 'h', y: 1.05 }, font: { family: 'inherit' } },
      config: { displaylogo: false, responsive: true },
    }
  }, [result])

  // box plot: per-group distribution of one feature (from /api/distribution)
  const boxPlot = useMemo(() => {
    if (!boxResult) return null
    return {
      data: boxResult.data.map((d, i) => ({
        type: 'box', name: d.group, y: d.values, boxpoints: 'all', jitter: 0.4, pointpos: 0,
        marker: { color: PALETTE[i % PALETTE.length], size: 5 }, line: { color: PALETTE[i % PALETTE.length] },
        text: d.samples, hoverinfo: 'y+text',
      })),
      layout: {
        margin: { l: 58, r: 20, t: 10, b: 70 }, height: 420,
        yaxis: { title: 'Relative frequency (%)' }, xaxis: { tickangle: -30 },
        showlegend: false, font: { family: 'inherit' },
      },
      config: { displaylogo: false, responsive: true },
    }
  }, [boxResult])

  async function fetchBox(feature) {
    setBoxBusy(true); setError(null)
    try { setBoxResult(await jpost('/api/distribution', { dataset, field, level, feature })) }
    catch (e) { setError(e.detail || String(e)) }
    setBoxBusy(false)
  }

  // ---- multi-group grouped-bar ----------------------------------------------
  const multiPlot = useMemo(() => {
    if (result?.mode !== 'multi-group') return null
    const top = result.rows.slice(0, 12).slice().reverse()
    return {
      data: result.groups.map((g, gi) => ({
        type: 'bar', orientation: 'h', name: g, marker: { color: PALETTE[gi % PALETTE.length] },
        x: top.map(r => r.groupMeans[gi]), y: top.map(r => r.feature),
        hovertemplate: `${g}: %{x:.3f}%<extra>%{y}</extra>`,
      })),
      layout: {
        barmode: 'group', margin: { l: 175, r: 20, t: 10, b: 44 }, height: Math.max(320, top.length * 34 + 80),
        xaxis: { title: 'Mean proportion within group (%)' }, yaxis: { automargin: true, tickfont: { size: 11 } },
        legend: { orientation: 'h', y: 1.08 }, font: { family: 'inherit' },
      },
      config: { displaylogo: false, responsive: true, modeBarButtonsToRemove: ['lasso2d', 'select2d'] },
    }
  }, [result])

  // ---- PCA scatter ----------------------------------------------------------
  const pcaPlot = useMemo(() => {
    if (result?.mode !== 'pca') return null
    return {
      data: result.groups.map((g, gi) => {
        const pts = result.points.filter(p => p.group === g)
        return { type: 'scatter', mode: 'markers', name: g,
          marker: { color: PALETTE[gi % PALETTE.length], size: 10, line: { width: 0.5, color: '#fff' } },
          x: pts.map(p => p.pc1), y: pts.map(p => p.pc2), text: pts.map(p => p.name), hoverinfo: 'text+name' }
      }),
      layout: {
        margin: { l: 60, r: 20, t: 10, b: 50 }, height: 520,
        xaxis: { title: `PC1 (${result.varPC1.toFixed(1)}%)`, zeroline: true },
        yaxis: { title: `PC2 (${result.varPC2.toFixed(1)}%)`, zeroline: true, scaleanchor: 'x' },
        legend: { orientation: 'h', y: 1.06 }, font: { family: 'inherit' },
      },
      config: { displaylogo: false, responsive: true },
    }
  }, [result])

  // ---- heatmap (+ group colour strip) ---------------------------------------
  const heatmap = useMemo(() => {
    if (result?.mode !== 'heatmap') return null
    const gi = g => result.groups.indexOf(g)
    const G = result.groups.length
    // discrete colourscale for the group strip
    const strip = []
    for (let i = 0; i < G; i++) {
      const c = PALETTE[i % PALETTE.length]
      strip.push([i / G, c]); strip.push([(i + 1) / G, c])
    }
    return {
      main: {
        data: [{
          type: 'heatmap', z: result.z, x: result.samples, y: result.features,
          colorscale: 'YlGnBu', reversescale: true, colorbar: { title: '% ', thickness: 12, len: 0.7 },
          hovertemplate: '%{y}<br>%{x}<br>%{z:.3f}%<extra></extra>',
        }],
        layout: { margin: { l: 175, r: 20, t: 6, b: 90 }, height: Math.max(380, result.features.length * 15 + 120),
          xaxis: { tickangle: -90, tickfont: { size: 8 }, automargin: true },
          yaxis: { automargin: true, tickfont: { size: 10 } }, font: { family: 'inherit' } },
        config: { displaylogo: false, responsive: true },
      },
      strip: {
        data: [{
          type: 'heatmap', z: [result.sampleGroups.map(g => gi(g) + 0.5)], x: result.samples, y: ['group'],
          colorscale: strip, zmin: 0, zmax: G, showscale: false,
          hovertext: [result.sampleGroups], hoverinfo: 'x+text',
        }],
        layout: { margin: { l: 175, r: 20, t: 4, b: 2 }, height: 34,
          xaxis: { showticklabels: false, showgrid: false }, yaxis: { showticklabels: true, tickfont: { size: 9 } },
          font: { family: 'inherit' } },
        config: { displaylogo: false, responsive: true, displayModeBar: false },
      },
    }
  }, [result])

  const canRun = schema && (
    mode === 'sample' ? (sample1 && sample2 && sample1 !== sample2)
    : mode === 'two' ? (field && group1 && group2 && group1 !== group2)
    : field)

  return (
    <>
      <header>
        <h1>STAMP<span>-web · proof of concept</span></h1>
        <span className="meta">FastAPI + React · reusing STAMP’s Python analysis core (no Qt, no matplotlib)</span>
      </header>

      <div className="wrap">
        <div className="panel">
          <Field label="Dataset"><Sel value={dataset} onChange={setDataset} options={datasets.map(d => d.id)} /></Field>
          <div className="filebox">
            <label style={{ margin: 0 }}>Import your own data</label>
            <select value={importerKind} onChange={e => setImporterKind(e.target.value)}>
              {IMPORTERS.map(i => <option key={i.id} value={i.id}>{i.label}</option>)}
            </select>
            <input key={importerKind} ref={fileRef} type="file"
                   accept={importer.accept} multiple={importer.multiple} />
            {importer.id === 'cog' &&
              <select value={cogTreatment} onChange={e => setCogTreatment(e.target.value)}>
                <option>Treat multi-code COGs as features</option>
                <option>Assign sequence to each COG code</option>
              </select>}
            <input ref={upMeta} type="file" accept=".tsv,.txt" title="metadata (.tsv, optional)" />
            <button onClick={doImport} disabled={busy}>{busy ? 'Importing…' : 'Import'}</button>
            <div className="hint">{importer.help} Metadata .tsv optional (needed for grouping).</div>
          </div>

          {schema && <>
            <div className="tabs" style={{ marginTop: 16, flexWrap: 'wrap' }}>
              {MODES.map(([m, lbl]) =>
                <div key={m} className={'tab' + (mode === m ? ' active' : '')}
                     onClick={() => { setMode(m); setResult(null) }}>{lbl}</div>)}
            </div>

            {schema.fields.length === 0 &&
              <div className="hint">No metadata for this dataset — only <b>Two samples</b> works.
                Upload a metadata .tsv to enable grouping.</div>}
            {mode !== 'sample' && schema.fields.length > 0 &&
              <Field label="Metadata field"><Sel value={field} onChange={setField} options={schema.fields.map(f => f.field)} /></Field>}
            <Field label="Hierarchy level"><Sel value={level} onChange={setLevel} options={schema.levels} /></Field>

            {mode === 'sample' && <>
              <div className="row2">
                <div><Field label="Sample 1"><Sel value={sample1} onChange={setSample1} options={schema.samples} /></Field></div>
                <div><Field label="Sample 2"><Sel value={sample2} onChange={setSample2} options={schema.samples} /></Field></div>
              </div>
              <Field label="Statistical test"><Sel value={sampleTest} onChange={setSampleTest} options={schema.sampleTests} /></Field>
              <Field label="Multiple-test correction"><Sel value={correction} onChange={setCorrection} options={schema.corrections} /></Field>
            </>}

            {mode === 'two' && <>
              <div className="row2">
                <div><Field label="Group 1" hint={fieldObj ? `n = ${fieldObj.groups[group1] ?? '?'}` : ''}>
                  <Sel value={group1} onChange={setGroup1} options={groupNames} /></Field></div>
                <div><Field label="Group 2" hint={fieldObj ? `n = ${fieldObj.groups[group2] ?? '?'}` : ''}>
                  <Sel value={group2} onChange={setGroup2} options={groupNames} /></Field></div>
              </div>
              <Field label="Statistical test"><Sel value={test} onChange={setTest} options={schema.tests} /></Field>
              <Field label="Multiple-test correction"><Sel value={correction} onChange={setCorrection} options={schema.corrections} /></Field>
            </>}

            {mode === 'multi' && <>
              <Field label="Statistical test"><Sel value={mgTest} onChange={setMgTest} options={schema.mgTests} /></Field>
              <Field label="Multiple-test correction"><Sel value={mgCorrection} onChange={setMgCorrection} options={schema.corrections} /></Field>
              <div className="hint">Compares all {groupNames.length} groups in “{field}”.</div>
            </>}

            {mode === 'pca' && <div className="hint" style={{ marginTop: 10 }}>Ordination of all samples in “{field}”, coloured by group.</div>}

            {mode === 'heatmap' && <>
              <Field label="Most-variable features"><input className="num" type="number" min="5" max="100" value={hmTopN}
                onChange={e => setHmTopN(+e.target.value)} /></Field>
              <div className="hint">Clustered heatmap of samples × top features, grouped by “{field}”.</div>
            </>}

            <button className="run" onClick={run} disabled={busy || !canRun}>
              {busy ? 'Running…' : mode === 'pca' ? 'Compute PCA' : mode === 'heatmap' ? 'Compute heatmap' : 'Run analysis'}
            </button>
            {mode === 'sample' && sample1 === sample2 && <div className="hint">Pick two different samples.</div>}
            {mode === 'two' && group1 === group2 && <div className="hint">Pick two different groups.</div>}
            {mode === 'two' && test.includes('White') && <div className="hint">White’s test is permutation-based — a few seconds.</div>}
          </>}
          {error && <div className="err">⚠ {error}</div>}
        </div>

        <div className="panel">
          {!result && <p className="meta">Pick a dataset, choose a mode, and run — the analysis
            comes straight from STAMP’s Python core and renders as an interactive Plotly chart.</p>}

          {isPair && <>
            <div className="stat">
              <div><b>{result.group1}</b>{isSample ? '' : ` (n=${result.n1})`} vs <b>{result.group2}</b>{isSample ? '' : ` (n=${result.n2})`}</div>
              <div>{result.level} · {result.count} features</div>
              <div>{result.test} · {result.correction}</div>
            </div>
            <div className="row2" style={{ alignItems: 'end', gap: 16 }}>
              <div><label>Features shown</label>
                <input type="range" min="5" max={Math.min(60, Math.max(5, pairFiltered.length))} value={topN}
                       onChange={e => setTopN(+e.target.value)} style={{ width: '100%' }} /> {topN}</div>
              <div><label>Significance α</label><input className="num" type="number" step="0.01" min="0" max="1" value={alpha} onChange={e => setAlpha(+e.target.value)} /></div>
              <div><label>Min |diff| %</label><input className="num" type="number" step="0.1" min="0" value={minEffect} onChange={e => setMinEffect(+e.target.value)} /></div>
            </div>
            <label className="chk"><input type="checkbox" checked={sigOnly} onChange={e => setSigOnly(e.target.checked)} /> significant only</label>
            <label className="chk"><input type="checkbox" checked={useCorrected} onChange={e => setUseCorrected(e.target.checked)} /> use corrected q-value for the threshold</label>
            <div className="hint">{pairFiltered.length} of {result.count} features pass the current filter.</div>
            <div className="tabs" style={{ marginTop: 6, flexWrap: 'wrap' }}>
              {[['effect', 'Effect size'], ['scatter', 'Scatter'], ['profilebar', 'Profile bar'],
                ['pvalue', 'p-value histogram'], ['multcomp', 'Mult. comparison'],
                ...(isSample ? [['seqhist', 'Seq. histogram']] : [])].map(([k, l]) =>
                <div key={k} className={'tab' + (pairPlotKind === k ? ' active' : '')} onClick={() => setPairPlotKind(k)}>{l}</div>)}
            </div>
            {pairPlotKind === 'effect' && (pairPlot?.empty ? <p className="err">No features pass the filter.</p>
              : pairPlot && <Plot data={pairPlot.data} layout={pairPlot.layout} config={pairPlot.config} style={{ width: '100%' }} useResizeHandler />)}
            {pairPlotKind === 'scatter' && pairScatter &&
              <Plot data={pairScatter.data} layout={pairScatter.layout} config={pairScatter.config} style={{ width: '100%' }} useResizeHandler />}
            {pairPlotKind === 'profilebar' && profileBar &&
              <Plot data={profileBar.data} layout={profileBar.layout} config={profileBar.config} style={{ width: '100%' }} useResizeHandler />}
            {pairPlotKind === 'pvalue' && (() => { const h = pvalueHistogram(result.rows); return <Plot data={h.data} layout={h.layout} config={h.config} style={{ width: '100%' }} useResizeHandler /> })()}
            {pairPlotKind === 'multcomp' && multComp && <>
              <p className="hint">Effect of the correction: raw p vs corrected q, and how many features pass each threshold.</p>
              <Plot data={multComp.scatter.data} layout={multComp.scatter.layout} config={multComp.scatter.config} style={{ width: '100%' }} useResizeHandler />
              <Plot data={multComp.cumulative.data} layout={multComp.cumulative.layout} config={multComp.cumulative.config} style={{ width: '100%' }} useResizeHandler />
            </>}
            {pairPlotKind === 'seqhist' && seqHist &&
              <Plot data={seqHist.data} layout={seqHist.layout} config={seqHist.config} style={{ width: '100%' }} useResizeHandler />}
            <div className="tablewrap">
              <table><thead><tr>
                <th>Feature</th><th>{isSample ? '' : 'mean '}{result.group1} (%)</th><th>{isSample ? '' : 'mean '}{result.group2} (%)</th>
                <th>diff (%)</th><th>p-value</th><th>q-value</th>
              </tr></thead><tbody>
                {pairFiltered.slice(0, topN).map((r, i) => <tr key={i}>
                  <td>{r.feature}</td><td>{nf(r.mean1)}</td><td>{nf(r.mean2)}</td>
                  <td style={{ color: r.effect >= 0 ? G1 : G2 }}>{nf(r.effect)}</td>
                  <td className={r.pvalue < 0.05 ? 'sig' : ''}>{ne(r.pvalue)}</td>
                  <td className={r.corrected < 0.05 ? 'sig' : ''}>{ne(r.corrected)}</td>
                </tr>)}
              </tbody></table>
            </div>
          </>}

          {result?.mode === 'multi-group' && <>
            <div className="stat">
              <div><b>{result.groups.length}</b> groups · {result.level} · {result.count} features</div>
              <div>{result.test} · {result.correction}</div>
            </div>
            <div className="legend">{result.groups.map((g, gi) =>
              <span key={g}><span className="sw" style={{ background: PALETTE[gi % PALETTE.length] }} />{g} (n={result.groupSizes[gi]})</span>)}</div>
            <div className="tabs" style={{ marginTop: 4 }}>
              {[['bar', 'Group means'], ['box', 'Box plot'], ['pvalue', 'p-value histogram']].map(([k, l]) =>
                <div key={k} className={'tab' + (mgPlotKind === k ? ' active' : '')} onClick={() => setMgPlotKind(k)}>{l}</div>)}
            </div>
            {mgPlotKind === 'bar' && <>
              <p className="hint">Top 12 features by p-value — mean proportion in each group.</p>
              {multiPlot && <Plot data={multiPlot.data} layout={multiPlot.layout} config={multiPlot.config} style={{ width: '100%' }} useResizeHandler />}
            </>}
            {mgPlotKind === 'pvalue' && (() => { const h = pvalueHistogram(result.rows); return <Plot data={h.data} layout={h.layout} config={h.config} style={{ width: '100%' }} useResizeHandler /> })()}
            {mgPlotKind === 'box' && <>
              <div className="row2" style={{ alignItems: 'end', marginTop: 6 }}>
                <div><label>Feature</label><Sel value={boxFeature} onChange={setBoxFeature} options={result.rows.slice(0, 50).map(r => r.feature)} /></div>
                <div><button className="run" style={{ marginTop: 0 }} onClick={() => fetchBox(boxFeature)} disabled={boxBusy}>{boxBusy ? 'Loading…' : 'Show distribution'}</button></div>
              </div>
              {boxResult && boxPlot && <>
                <p className="hint" style={{ marginTop: 8 }}>Per-sample relative frequency of <b>{boxResult.feature}</b> in each group.</p>
                <Plot data={boxPlot.data} layout={boxPlot.layout} config={boxPlot.config} style={{ width: '100%' }} useResizeHandler />
              </>}
            </>}
            <div className="tablewrap">
              <table><thead><tr><th>Feature</th><th>p-value</th><th>q-value</th><th>η² (effect)</th></tr></thead>
                <tbody>{result.rows.slice(0, 30).map((r, i) => <tr key={i}>
                  <td>{r.feature}</td><td className={r.pvalue < 0.05 ? 'sig' : ''}>{ne(r.pvalue)}</td>
                  <td className={r.corrected < 0.05 ? 'sig' : ''}>{ne(r.corrected)}</td><td>{nf(r.effect)}</td></tr>)}
                </tbody></table>
            </div>

            <div style={{ marginTop: 18, borderTop: '1px solid var(--line)', paddingTop: 14 }}>
              <b style={{ fontSize: 13 }}>Post-hoc — pairwise group comparisons</b>
              <div className="row2" style={{ alignItems: 'end', marginTop: 8 }}>
                <div><label>Feature</label>
                  <Sel value={phFeature} onChange={setPhFeature} options={result.rows.slice(0, 50).map(r => r.feature)} /></div>
                <div><label>Post-hoc test</label>
                  <Sel value={phTest} onChange={setPhTest} options={schema.postHocTests} /></div>
                <div><button className="run" style={{ marginTop: 0 }} onClick={runPostHoc} disabled={phBusy}>
                  {phBusy ? 'Running…' : 'Run post-hoc'}</button></div>
              </div>
              {phResult && phPlot && <>
                <p className="hint" style={{ marginTop: 10 }}>{phResult.test} · {phResult.rows.length} pairwise
                  comparisons of <b>{phResult.feature}</b> across {phResult.groups.length} groups.</p>
                <Plot data={phPlot.data} layout={phPlot.layout} config={phPlot.config} style={{ width: '100%' }} useResizeHandler />
              </>}
            </div>
          </>}

          {result?.mode === 'pca' && <>
            <div className="stat">
              <div><b>{result.points.length}</b> samples · {result.groups.length} groups</div>
              <div>{result.level} · coloured by “{result.field}”</div>
              <div>PC1 {result.varPC1.toFixed(1)}% · PC2 {result.varPC2.toFixed(1)}%</div>
            </div>
            {pcaPlot && <Plot data={pcaPlot.data} layout={pcaPlot.layout} config={pcaPlot.config} style={{ width: '100%' }} useResizeHandler />}
            <p className="hint">PCA of the sample × feature relative-abundance matrix (NumPy SVD in the backend). Hover a point for its sample id.</p>
          </>}

          {result?.mode === 'heatmap' && heatmap && <>
            <div className="stat">
              <div><b>{result.features.length}</b> features × <b>{result.samples.length}</b> samples</div>
              <div>{result.level} · grouped by “{result.field}”</div>
            </div>
            <div className="legend">{result.groups.map((g, gi) =>
              <span key={g}><span className="sw" style={{ background: PALETTE[gi % PALETTE.length] }} />{g}</span>)}</div>
            <Plot data={heatmap.strip.data} layout={heatmap.strip.layout} config={heatmap.strip.config} style={{ width: '100%' }} useResizeHandler />
            <Plot data={heatmap.main.data} layout={heatmap.main.layout} config={heatmap.main.config} style={{ width: '100%' }} useResizeHandler />
            <p className="hint">Relative abundance (%) of the {result.features.length} most-variable features,
              hierarchically clustered on both axes (SciPy). The strip above colours each sample by its group.</p>
          </>}
        </div>
      </div>
    </>
  )
}
