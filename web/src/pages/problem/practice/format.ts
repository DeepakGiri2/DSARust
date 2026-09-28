// Text the Practice tab shows or hands on: examples, stdin, the AI's view of
// the last run. Ported from crates/dsa-app/src/practice.rs and
// crates/dsa-core/src/problem.rs so the web reads exactly like the desktop.

import type { InputValue, Problem, RunResult, TestCaseView, TestStatus } from '@/api/types'

/** `InputValue::to_editable` — the text form the input editor and stdin use. */
export function toEditable(v: InputValue): string {
  if (v === null) return 'null'
  if (Array.isArray(v)) return v.map(toEditable).join(' ')
  return String(v)
}

/** `fmt_value` — LeetCode-style values for the examples: `[1,2]`, `"ab"`, `9`. */
export function fmtValue(v: InputValue): string {
  if (Array.isArray(v)) return `[${v.map(fmtValue).join(',')}]`
  if (typeof v === 'string') return `"${v}"`
  return toEditable(v)
}

/** One EXAMPLE block: `Input: field = value` per declared input, then `Output:`. */
export function exampleText(problem: Pick<Problem, 'inputs'>, test: TestCaseView): string {
  const lines = problem.inputs.map((f) => {
    const v = test.input[f.name]
    return `Input: ${f.name} = ${v === undefined ? '' : fmtValue(v)}`
  })
  lines.push(`Output: ${test.expected}`)
  return lines.join('\n')
}

/**
 * STDIN's starting text: the default input serialized the way the harness
 * reads it — one field per line, in declaration order (`serialize_input`).
 * That is what the desktop pre-fills and what the server assumes when a run
 * sends no stdin. A problem that declares no inputs falls back to its first
 * test case.
 */
export function defaultStdin(problem: Pick<Problem, 'inputs' | 'default_fields' | 'tests'>): string {
  if (problem.inputs.length === 0) return problem.tests[0]?.stdin ?? ''
  return problem.inputs.map((f) => `${problem.default_fields[f.name] ?? ''}\n`).join('')
}

/** A test's stdin on one line, for the TESTS list and the AI context. */
export function inlineStdin(stdin: string): string {
  return stdin.replace(/\r\n?/g, '\n').trim().replace(/\n/g, ' | ')
}

/** At most `max` characters, with an ellipsis when cut. */
export function truncate(s: string, max: number): string {
  const chars = Array.from(s)
  return chars.length <= max ? s : `${chars.slice(0, max).join('')}…`
}

/** The toolbar's reminder of how to print a debug line in each language. */
export function logHint(lang: string): string {
  switch (lang) {
    case 'cpp':
      return 'cout << "dbg: " << x << endl;'
    case 'java':
      return 'System.out.println("dbg: " + x);'
    case 'python':
      return 'print("dbg:", x)'
    default:
      return 'fmt.Println("dbg:", x)'
  }
}

/** What a failed platform run says about itself (`RunOutcome::error` on the desktop). */
export function runErrorText(r: RunResult): string {
  return (r.stderr || r.compile?.output || '').trim() || 'The runner failed before your program could start.'
}

/** True when the program compiled (or the language has no compile step). */
export const compiled = (r: RunResult) => r.compile === null || r.compile.ok

/**
 * Everything the AI's Fix mode should know about the last run and the last
 * test sweep — `Practice::run_context`. Passing tests add nothing: noise in
 * the prompt is context the model has to read past.
 */
export function runContext(run: RunResult | null, tests: RunResult | null): string {
  const parts: string[] = []
  if (run) {
    if (run.status === 'error') {
      parts.push(`run error: ${runErrorText(run)}`)
    } else if (!compiled(run)) {
      parts.push(`compile error:\n${truncate(run.compile?.output ?? '', 1200)}`)
    } else {
      if (run.stderr?.trim()) parts.push(`stderr:\n${truncate(run.stderr, 800)}`)
      if (run.stdout?.trim()) parts.push(`stdout:\n${truncate(run.stdout, 800)}`)
      if (run.exit_code != null && run.exit_code !== 0) parts.push(`exit code: ${run.exit_code}`)
    }
  }
  ;(tests?.tests ?? []).forEach((t, i) => {
    if (t.status === 'fail') {
      parts.push(
        `test ${i + 1} FAILED — input: ${inlineStdin(t.stdin)} · expected "${t.expected}" · got "${t.actual || '(nothing)'}"`,
      )
    } else if (t.status !== 'pass') {
      parts.push(`test ${i + 1} ${t.status} — ${truncate(t.detail, 400)}`)
    }
  })
  return parts.join('\n')
}

export type Tone = 'green' | 'red' | 'amber' | 'dim'

/** The TESTS badge per outcome — ✔, ✖, or ! for anything that is not a verdict on the answer. */
export const TEST_BADGE: Record<TestStatus, { glyph: string; tone: Tone; label: string }> = {
  pass: { glyph: '✔', tone: 'green', label: 'passed' },
  fail: { glyph: '✖', tone: 'red', label: 'wrong answer' },
  build: { glyph: '!', tone: 'amber', label: 'did not compile' },
  crash: { glyph: '!', tone: 'amber', label: 'crashed' },
  timeout: { glyph: '!', tone: 'amber', label: 'timed out' },
  error: { glyph: '!', tone: 'amber', label: 'runner error' },
}
