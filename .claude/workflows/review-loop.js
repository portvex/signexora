export const meta = {
  name: 'review-loop',
  description: 'Rust, DSP and UI/UX reviewers review the changes, findings are verified and fixed, and the fixes are re-reviewed until a round finds nothing',
  whenToUse: 'After a phase or any substantial change. args: {base: git ref to review from (default HEAD~1), maxRounds (default 4), sliceLines (default 4000), startRound/previous/wontfix to continue a run with re-review rounds}',
  phases: [
    { title: 'Scope', detail: 'list changed Rust files and split them into review slices' },
    { title: 'Review', detail: 'rust-reviewer on every slice, dsp-researcher on DSP crates, ui-ux-designer on sn-gui and sn-ui-core' },
    { title: 'Verify', detail: 'a second agent of the same role checks every finding against the code' },
    { title: 'Fix', detail: 'one fixer per crate in parallel, then cross-crate and workspace fixes' },
    { title: 'Gate', detail: 'scripts/check.sh must pass' },
  ],
}

const A = args || {}
const BASE = A.base || 'HEAD~1'
const MAX_ROUNDS = A.maxRounds || 4
const SLICE_LINES = A.sliceLines || 4000
// Continue an earlier run with re-review rounds only: startRound > 1 skips the
// scope and full review; previous lists that run's last fixes ({file, title})
// and wontfix its declined items, so they are not reported again.
const START_ROUND = A.startRound || 1

const ROLES = {
  rust: { agentType: 'rust-reviewer', file: '.claude/agents/rust-reviewer.md', crates: null },
  dsp: {
    agentType: 'dsp-researcher',
    file: '.claude/agents/dsp-researcher.md',
    crates: ['sn-dsp', 'sn-analysis', 'sn-demod', 'sn-ofdm', 'sn-synth'],
  },
  ui: { agentType: 'ui-ux-designer', file: '.claude/agents/ui-ux-designer.md', crates: ['sn-gui', 'sn-ui-core'] },
}

const RULES = `Project rules that every change must keep:
- No unwrap, expect, panic!, unreachable!, todo! or panicking indexing/slicing in non-test code (clippy denies them). Handle every error with a clear message.
- In docs, comments and UI strings: no em dashes, no decorative en dashes, no Unicode arrows, bullets, checkmarks, stars or emoticons. Use plain hyphens and ASCII.
- Do not commit; the caller commits after the loop.`

const FINDINGS = {
  type: 'object',
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          severity: { type: 'string', enum: ['blocker', 'major', 'minor', 'nit'] },
          file: { type: 'string', description: 'repo-relative path' },
          line: { type: 'integer' },
          title: { type: 'string', description: 'short label of the issue' },
          problem: { type: 'string' },
          fix: { type: 'string', description: 'the concrete change to make' },
        },
        required: ['severity', 'file', 'title', 'problem', 'fix'],
      },
    },
  },
  required: ['findings'],
}

const VERDICTS = {
  type: 'object',
  properties: {
    verdicts: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          index: { type: 'integer' },
          real: { type: 'boolean' },
          reason: { type: 'string' },
        },
        required: ['index', 'real', 'reason'],
      },
    },
  },
  required: ['verdicts'],
}

const SLICES = {
  type: 'object',
  properties: {
    slices: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          name: { type: 'string' },
          crate: { type: 'string', description: 'crate directory name such as sn-dsp, or "workspace"' },
          lines: { type: 'integer' },
          paths: { type: 'array', items: { type: 'string' } },
        },
        required: ['name', 'crate', 'paths'],
      },
    },
  },
  required: ['slices'],
}

const FIXED = {
  type: 'object',
  properties: {
    fixed: {
      type: 'array',
      items: {
        type: 'object',
        properties: { title: { type: 'string' }, file: { type: 'string' }, summary: { type: 'string' } },
        required: ['title', 'summary'],
      },
    },
    wontfix: {
      type: 'array',
      items: {
        type: 'object',
        properties: { title: { type: 'string' }, file: { type: 'string' }, reason: { type: 'string' } },
        required: ['title', 'reason'],
      },
    },
    deferred: {
      type: 'array',
      description: 'fixes that need edits outside this fixer\'s crate',
      items: {
        type: 'object',
        properties: { title: { type: 'string' }, file: { type: 'string' }, change: { type: 'string' } },
        required: ['title', 'change'],
      },
    },
    checks: { type: 'string', description: 'which checks were run and their result' },
  },
  required: ['fixed', 'wontfix', 'deferred', 'checks'],
}

const GATE = {
  type: 'object',
  properties: {
    passed: { type: 'boolean' },
    summary: { type: 'string', description: 'what failed and what was changed to make it pass' },
  },
  required: ['passed', 'summary'],
}

// Run an agent as one of the project subagents. If the agent type is not
// registered in this session, run a general agent that loads the role file.
async function asRole(role, prompt, opts) {
  const r = ROLES[role]
  const full = `Your role is defined in ${r.file}. If you are not already running as that agent, read that file first and follow it exactly.\n\n${prompt}`
  try {
    const res = await agent(full, { ...opts, agentType: r.agentType })
    if (res) return res
  } catch (e) {
    log(`${r.agentType} not available as an agent type (${e && e.message ? e.message : e}); using the role file`)
  }
  return agent(full, opts)
}

const crateOf = (file) => {
  const m = /^crates\/([^/]+)\//.exec(file || '')
  return m ? m[1] : 'workspace'
}
const keyOf = (f) => `${f.file}::${(f.title || '').toLowerCase().trim()}`

function reviewPrompt(scope, known) {
  return `${scope}

Report concrete, fixable issues only, most severe first, at most 30. For each: severity, repo-relative file, line, a short title, the problem, and the concrete fix.
${known.length ? `\nAlready decided as won't fix (do not report again):\n${known.map((k) => `- ${k.file}: ${k.title} (${k.reason})`).join('\n')}\n` : ''}
${RULES}`
}

function verifyPrompt(findings) {
  return `Another reviewer with your role reported the findings below. Check each one against the code (and run code or tests when that settles it). Mark real=false when the claim is wrong, the case is already handled, the issue is outside what the code is meant to do, or the proposed fix would make things worse. Be skeptical: a plausible finding you cannot confirm in the code is not real. Return one verdict per index.

${findings.map((f, i) => `[${i}] ${f.severity} ${f.file}${f.line ? ':' + f.line : ''} - ${f.title}\nProblem: ${f.problem}\nFix: ${f.fix}`).join('\n\n')}`
}

async function reviewAndVerify(role, label, scope, known) {
  const review = await asRole(role, reviewPrompt(scope, known), {
    label: `review:${label}`,
    phase: 'Review',
    schema: FINDINGS,
  })
  const found = (review && review.findings) || []
  if (!found.length) return { label, found: 0, confirmed: [] }
  const v = await asRole(role, verifyPrompt(found), {
    label: `verify:${label}`,
    phase: 'Verify',
    schema: VERDICTS,
  })
  const real = new Set(((v && v.verdicts) || []).filter((x) => x.real).map((x) => x.index))
  const confirmed = found.filter((_, i) => real.has(i)).map((f) => ({ ...f, role, reviewer: label }))
  return { label, found: found.length, confirmed }
}

function fixPrompt(crate, findings) {
  const where = crate === 'workspace' ? 'anywhere in the repository' : `crates/${crate}/`
  return `Fix these confirmed review findings. Work in ${where}.

${findings.map((f, i) => `[${i}] (${f.role}, ${f.severity}) ${f.file}${f.line ? ':' + f.line : ''} - ${f.title}\nProblem: ${f.problem}\nSuggested fix: ${f.fix}`).join('\n\n')}

How to work:
- Read the code first. Fix each finding properly at its root; add or update a test when the finding is about behaviour. Several findings may describe the same problem: fix it once and list each as fixed.
- If after reading the code a finding is wrong or the fix would make things worse, leave it and list it under wontfix with the reason. A UI or DSP change that needs a product decision also goes under wontfix, with the options.
${crate === 'workspace' ? '' : `- Edit only files under crates/${crate}/. Other agents are fixing other crates at the same time. If a fix needs a change elsewhere (a caller in another crate, a workspace file), make your side of it and list the outside change under deferred, precisely. If the build breaks because of files outside your crate, wait a minute and retry; never edit them.\n`}- Finish with \`cargo fmt\` on what you changed, then clippy with warnings as errors and the tests for ${crate === 'workspace' ? 'the workspace (scripts/check.sh)' : `package ${crate} (cargo clippy -p ${crate} --all-targets -- -D warnings; cargo test -p ${crate})`}. Report what you ran and the result.

${RULES}`
}

// ---------------------------------------------------------------------------

phase('Scope')
const scope = START_ROUND > 1 ? { slices: [{ name: 'continued', paths: [] }] } : await agent(
  `List the Rust source files changed between git ref ${BASE} and the working tree: \`git diff --numstat ${BASE} -- '*.rs'\` plus untracked .rs files (\`git ls-files --others --exclude-standard -- '*.rs'\`, counting all their lines). Ignore fuzz/ and target/.
Group them into review slices of at most about ${SLICE_LINES} changed lines each. Keep a crate in one slice when it fits; split a larger crate by module or file. Several small crates may NOT share a slice. Give each slice a name (crate plus part, e.g. "sn-gui 1/2"), its crate directory name (or "workspace" for files outside crates/), the changed line count and the repo-relative paths.`,
  { label: 'scope', phase: 'Scope', schema: SLICES },
)
const slices = (scope && scope.slices) || []
log(`${slices.length} slices: ${slices.map((s) => `${s.name} (${s.lines || '?'})`).join(', ')}`)
if (!slices.length) return { rounds: [], note: `nothing changed since ${BASE}` }

const seen = new Set()
const fixedAll = []
const wontfixAll = [...(A.wontfix || [])]
const rounds = []
let previous = [...(A.previous || [])]
let gate = null
let remaining = []

for (let round = START_ROUND; round <= MAX_ROUNDS; round++) {
  phase('Review')
  log(`Round ${round}: reviewing`)
  const tasks = []
  if (round === 1) {
    for (const s of slices) {
      const files = s.paths.map((p) => `- ${p}`).join('\n')
      const scopeText = `Review scope: the changes since git ref ${BASE} in these files (see \`git diff ${BASE} -- <file>\`; a file that is new since ${BASE} is in scope entirely). Read callers and surrounding code as needed.\n${files}`
      tasks.push({ role: 'rust', label: `rust:${s.name}`, scope: scopeText })
    }
    for (const role of ['dsp', 'ui']) {
      for (const crate of ROLES[role].crates) {
        const paths = slices.filter((s) => s.crate === crate).flatMap((s) => s.paths)
        if (!paths.length) continue
        const scopeText = `Review scope: the changes since git ref ${BASE} in crate ${crate} (see \`git diff ${BASE} -- <file>\`; new files are in scope entirely). Read the relevant sections of docs/SIGNAL_ANALYZER.md and docs/DEVELOPMENT_PLAN.md.\n${paths.map((p) => `- ${p}`).join('\n')}`
        tasks.push({ role, label: `${role}:${crate}`, scope: scopeText })
      }
    }
  } else {
    const fixedList = previous.map((f) => `- ${f.file}: ${f.title}`).join('\n')
    for (const role of ['rust', 'dsp', 'ui']) {
      const domain = ROLES[role].crates ? `in crates ${ROLES[role].crates.join(', ')}` : 'anywhere'
      const scopeText = `Review scope: the fixes made in the last review round, i.e. the uncommitted changes (\`git diff HEAD\`, plus untracked files from \`git status --short\`) ${domain}. Check that each fix below is correct and complete and that the changes introduced no new problems. Report an unfixed or wrongly fixed item again as a finding.\nItems fixed in round ${round - 1}:\n${fixedList || '- (none listed)'}`
      tasks.push({ role, label: `${role}:fixes-r${round}`, scope: scopeText })
    }
  }

  const reviews = await pipeline(tasks, (t) =>
    reviewAndVerify(t.role, `${t.label}`, t.scope, wontfixAll),
  )
  const all = reviews.filter(Boolean)
  const confirmed = all.flatMap((r) => r.confirmed)
  const fresh = []
  for (const f of confirmed) {
    const k = keyOf(f)
    // Round 1 dedups exact repeats; later rounds may legitimately re-report an
    // item whose fix was incomplete, so only drop repeats within the round.
    if (!seen.has(k) || round > 1) fresh.push(f)
    seen.add(k)
  }
  const unique = []
  const inRound = new Set()
  for (const f of fresh) {
    const k = keyOf(f)
    if (inRound.has(k)) continue
    inRound.add(k)
    unique.push(f)
  }
  const foundCount = all.reduce((n, r) => n + r.found, 0)
  log(`Round ${round}: ${foundCount} reported, ${unique.length} confirmed`)
  if (!unique.length) {
    rounds.push({ round, reported: foundCount, confirmed: 0 })
    remaining = []
    break
  }

  phase('Fix')
  const groups = {}
  for (const f of unique) (groups[crateOf(f.file)] = groups[crateOf(f.file)] || []).push(f)
  const crateNames = Object.keys(groups).filter((c) => c !== 'workspace')
  const fixResults = await parallel(
    crateNames.map((c) => () =>
      agent(fixPrompt(c, groups[c]), { label: `fix:${c}:r${round}`, phase: 'Fix', schema: FIXED }),
    ),
  )
  const results = fixResults.filter(Boolean)
  const lostCrates = crateNames.filter((_, i) => !fixResults[i])
  const deferred = results.flatMap((r) => r.deferred || [])
  const crossCrate = [
    ...(groups.workspace || []),
    ...deferred.map((d) => ({
      role: 'deferred',
      severity: 'major',
      file: d.file || 'workspace',
      title: d.title,
      problem: 'A fixer in another crate needs this change outside its crate.',
      fix: d.change,
    })),
    ...lostCrates.flatMap((c) => groups[c]),
  ]
  if (crossCrate.length) {
    const r = await agent(fixPrompt('workspace', crossCrate), {
      label: `fix:workspace:r${round}`,
      phase: 'Fix',
      schema: FIXED,
    })
    if (r) results.push({ ...r, deferred: [] })
  }
  const fixedNow = results.flatMap((r) => r.fixed || [])
  const wontNow = results.flatMap((r) => r.wontfix || [])
  fixedAll.push(...fixedNow.map((f) => ({ ...f, round })))
  wontfixAll.push(...wontNow.map((w) => ({ ...w, file: w.file || '', round })))
  previous = fixedNow.map((f) => ({ file: f.file || '', title: f.title }))

  phase('Gate')
  gate = await agent(
    `Run scripts/check.sh (format check, clippy with warnings as errors, all tests, doc tests). If anything fails, fix the cause properly (not by weakening tests or adding allow attributes unless the lint is wrong for that line, with a comment), then run it again until it passes. Also run \`cargo build -p sn-gui\`. Report whether it passes and what you changed.\n\n${RULES}`,
    { label: `gate:r${round}`, phase: 'Gate', schema: GATE },
  )
  rounds.push({
    round,
    reported: foundCount,
    confirmed: unique.length,
    fixed: fixedNow.length,
    wontfix: wontNow.length,
    deferred: deferred.length,
    gate: gate ? gate.passed : null,
  })
  remaining = unique
  log(`Round ${round}: fixed ${fixedNow.length}, won't fix ${wontNow.length}, gate ${gate && gate.passed ? 'passed' : 'FAILED'}`)
  if (round === MAX_ROUNDS) log(`Stopped after ${MAX_ROUNDS} rounds; the last round's fixes were not re-reviewed`)
}

return {
  base: BASE,
  rounds,
  fixed: fixedAll,
  wontfix: wontfixAll,
  lastRoundFindings: remaining.map((f) => ({ severity: f.severity, file: f.file, line: f.line, title: f.title, role: f.role })),
  gate,
}
