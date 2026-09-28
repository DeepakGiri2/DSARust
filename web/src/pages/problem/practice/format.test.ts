// Ported from the tests in crates/dsa-app/src/practice.rs and
// crates/dsa-harness/src/lib.rs, plus the web's own run-error mapping.
import { describe, expect, it } from 'vitest'
import { ApiError } from '@/api/client'
import type { InputField, Problem, RunResult, TestCaseView, TestResult } from '@/api/types'
import { nextPlaylistName } from '../header/ProblemHeader'
import { defaultStdin, exampleText, fmtValue, inlineStdin, logHint, runContext, toEditable, truncate } from './format'
import { describeRunError } from './runNotice'

const field = (name: string, type: InputField['type']): InputField => ({
  name,
  label: name,
  type,
  min: null,
  max: null,
  min_len: null,
  max_len: null,
  charset: null,
  sorted: false,
  unique: false,
  help: null,
})

const problem: Pick<Problem, 'inputs' | 'default_fields' | 'tests'> = {
  inputs: [field('nums', 'int-array'), field('target', 'int')],
  default_fields: { nums: '2 7 11 15 3', target: '14' },
  tests: [],
}

const result = (patch: Partial<RunResult>): RunResult => ({
  id: 'r',
  slug: 'two-sum',
  lang: 'go',
  kind: 'run',
  mode: 'solution',
  status: 'ok',
  compile: null,
  passed: 0,
  total: 0,
  duration_ms: 0,
  progress: { status: 'attempted', favourite: false, attempts: 1, solved_at: null, updated_at: '' },
  created_at: '',
  ...patch,
})

const outcome = (patch: Partial<TestResult>): TestResult => ({
  name: 'case 1',
  status: 'pass',
  stdin: '',
  expected: '1',
  actual: '1',
  detail: '',
  duration_ms: 0,
  edge: false,
  ...patch,
})

describe('examples', () => {
  it('renders LeetCode-style values', () => {
    expect(fmtValue([1, 2])).toBe('[1,2]')
    expect(fmtValue('ab')).toBe('"ab"')
    expect(fmtValue(9)).toBe('9')
    expect(fmtValue([[1, 2], [3]])).toBe('[[1,2],[3]]')
    expect(fmtValue(true)).toBe('true')
    expect(fmtValue(null)).toBe('null')
  })

  it('lists every declared input, then the output', () => {
    const test: TestCaseView = {
      name: 'case 1',
      input: { target: 14, nums: [2, 7, 11, 15, 3] },
      stdin: '2 7 11 15 3\n14\n',
      expected: '2 4',
      edge: false,
    }
    expect(exampleText(problem, test)).toBe('Input: nums = [2,7,11,15,3]\nInput: target = 14\nOutput: 2 4')
  })
})

describe('stdin', () => {
  it('is one field per line in declaration order, like serialize_input', () => {
    expect(defaultStdin(problem)).toBe('2 7 11 15 3\n14\n')
  })

  it('still emits the line of a field missing from the default input', () => {
    // Otherwise every later field would shift up a line in the program's reader.
    expect(defaultStdin({ ...problem, default_fields: { target: '3' } })).toBe('\n3\n')
  })

  it('falls back to the first test when the problem declares no inputs', () => {
    const tests: TestCaseView[] = [{ name: '', input: {}, stdin: 'x\n', expected: '', edge: false }]
    expect(defaultStdin({ inputs: [], default_fields: {}, tests })).toBe('x\n')
  })

  it('is shown on one line in the tests list', () => {
    expect(inlineStdin('2 7 11 15 3\n14\n')).toBe('2 7 11 15 3 | 14')
    expect(inlineStdin('a\r\nb\r\n')).toBe('a | b')
  })

  it('uses the editable form of values', () => {
    expect(toEditable([1, 2, 3])).toBe('1 2 3')
    expect(toEditable('4 2 7')).toBe('4 2 7')
  })
})

describe('the AI run context', () => {
  it('summarises failures', () => {
    const run = result({ exit_code: 1, stderr: 'index out of range' })
    const tests = result({
      kind: 'test',
      status: 'failed',
      tests: [outcome({ status: 'fail', stdin: '2 7\n9\n', expected: '0 1', actual: '1 0' })],
    })
    const ctx = runContext(run, tests)
    expect(ctx).toContain('index out of range')
    expect(ctx).toContain('exit code: 1')
    expect(ctx).toContain('test 1 FAILED — input: 2 7 | 9')
    expect(ctx).toContain('expected "0 1"')
    expect(ctx).toContain('got "1 0"')
  })

  it('adds no noise for a passing test', () => {
    expect(runContext(null, result({ kind: 'test', tests: [outcome({})] }))).toBe('')
  })

  it('reports a compile error instead of the output', () => {
    const ctx = runContext(result({ status: 'compile_error', compile: { ok: false, output: 'undefined: x', duration_ms: 1 }, stdout: 'ignored' }), null)
    expect(ctx).toBe('compile error:\nundefined: x')
  })

  it('names non-verdict outcomes by their status', () => {
    const ctx = runContext(null, result({ kind: 'test', tests: [outcome({ status: 'timeout', detail: 'killed after 12s' })] }))
    expect(ctx).toBe('test 1 timeout — killed after 12s')
  })

  it('truncates long output for the prompt', () => {
    expect(Array.from(truncate('x'.repeat(2000), 100)).length).toBeLessThanOrEqual(101)
    expect(truncate('short', 100)).toBe('short')
  })
})

describe('toolbar', () => {
  it('shows the debug-print idiom of the language', () => {
    expect(logHint('go')).toContain('fmt.Println')
    expect(logHint('cpp')).toContain('cout')
    expect(logHint('java')).toContain('System.out')
    expect(logHint('python')).toContain('print')
  })
})

describe('playlist names', () => {
  it('takes the next free "playlist N", like the desktop', () => {
    expect(nextPlaylistName([])).toBe('playlist 1')
    expect(nextPlaylistName(['interview'])).toBe('playlist 2')
    expect(nextPlaylistName(['playlist 2', 'x'])).toBe('playlist 3')
  })
})

describe('run errors', () => {
  it('turns a 429 into a cooldown', () => {
    const n = describeRunError(new ApiError(429, 'rate_limited', 'slow down', { retry_after_secs: 12 }), 1000)
    expect(n).toMatchObject({ kind: 'error', retryAt: 13_000 })
  })

  it('asks a signed-out user to sign in', () => {
    expect(describeRunError(new ApiError(401, 'unauthorized', 'no session'))).toEqual({ kind: 'signin' })
  })

  it('points at the fix for plan and verification errors', () => {
    expect(describeRunError(new ApiError(402, 'payment_required', 'Pro only'))).toMatchObject({ link: { to: '/pricing' } })
    expect(describeRunError(new ApiError(403, 'email_unverified', 'verify'))).toMatchObject({ link: { to: '/account' } })
  })

  it('lists every validation message', () => {
    const n = describeRunError(new ApiError(422, 'validation', 'bad', { errors: ['code is too long'] }))
    expect(n).toMatchObject({ kind: 'error', details: ['code is too long'] })
  })
})
