// `finishFix` cases are ported from the tests in crates/dsa-app/src/assistant.rs,
// the mode checks from crates/dsa-ai/src/lib.rs.
import { describe, expect, it } from 'vitest'
import { finishFix, historyFor, type AssistantTurn, type ChatTurn, type UserTurn } from './conversation'
import { HISTORY_TURNS, MODE_ORDER, MODES } from './modes'

let seq = 0
const ask = (content: string): UserTurn => ({ id: `u${seq++}`, role: 'user', content, codeAttached: false })
const reply = (content: string, status: AssistantTurn['status'] = 'done'): AssistantTurn => ({
  id: `a${seq++}`,
  role: 'assistant',
  content,
  thinking: '',
  status,
  error: null,
})

describe('historyFor', () => {
  it('sends the last HISTORY_TURNS messages, ending with the new question', () => {
    const turns: ChatTurn[] = []
    for (let i = 0; i < 10; i++) turns.push(ask(`q${i}`), reply(`a${i}`))
    turns.push(ask('latest'))
    const history = historyFor(turns)
    expect(history.length).toBeLessThanOrEqual(HISTORY_TURNS)
    expect(history.at(-1)).toEqual({ role: 'user', content: 'latest' })
    // A full window of alternating turns would open on a reply; hosted chat
    // APIs want the user first.
    expect(history[0].role).toBe('user')
    history.forEach((m, i) => i > 0 && expect(m.role).not.toBe(history[i - 1].role))
  })

  it('leaves out replies that never said anything, and the questions they failed', () => {
    const history = historyFor([
      ask('rate limited'),
      reply('', 'error'),
      ask('stopped at once'),
      reply('', 'stopped'),
      ask('answered'),
      reply('partly', 'stopped'),
      ask('now'),
    ])
    expect(history).toEqual([
      { role: 'user', content: 'answered' },
      { role: 'assistant', content: 'partly' },
      { role: 'user', content: 'now' },
    ])
  })

  it('does not feed the model its own inline reasoning', () => {
    const history = historyFor([ask('q'), reply('<think>hmm</think>Try a set.'), ask('why?')])
    expect(history[1]).toEqual({ role: 'assistant', content: 'Try a set.' })
  })
})

describe('finishFix', () => {
  it('a fix reply becomes a proposal', () => {
    const out = finishFix(
      'ISSUES:\n- off by one\n\nFIXED CODE:\n```go\nfunc f() {\n\treturn 2\n}\n```',
      'func f() {\n\treturn 1\n}',
    )
    expect(out.kind).toBe('proposal')
    if (out.kind !== 'proposal') return
    expect(out.code).toContain('return 2')
    expect(out.analysis).toContain('off by one')
    expect(out.analysis).not.toContain('ISSUES:')
  })

  it('a proposal is re-indented to the code it was asked about', () => {
    const out = finishFix('```go\nfunc f() {\n    return 2\n}\n```', 'func f() {\n\treturn 1\n}')
    expect(out.kind === 'proposal' && out.code).toBe('func f() {\n\treturn 2\n}')
  })

  it('an identical fix is reported as no change', () => {
    const out = finishFix('ISSUES:\n- none found\n\n```go\nfunc f() {}\n```', 'func f() {}')
    expect(out).toEqual({ kind: 'unchanged', analysis: '- none found' })
  })

  it('a reply with no code block explains itself', () => {
    expect(finishFix('I could not read that.', 'x')).toEqual({ kind: 'text', analysis: 'I could not read that.' })
  })

  it('an empty model reply still says something useful', () => {
    const out = finishFix('', '')
    expect(out.kind).toBe('text')
    expect(out.analysis).toContain('nothing usable')
  })

  it('inline reasoning never reaches the analysis', () => {
    const out = finishFix('<think>the loop starts at 1</think>ISSUES:\n- skips index 0', 'x')
    expect(out).toEqual({ kind: 'text', analysis: '- skips index 0' })
  })
})

describe('modes', () => {
  it('every mode has a hint and a speaker', () => {
    for (const m of MODE_ORDER) {
      expect(MODES[m].emptyHint).not.toBe('')
      expect(MODES[m].speaker).not.toBe('')
      expect(MODES[m].label).not.toBe('')
      expect(MODES[m].placeholder).not.toBe('')
    }
    expect(MODES.interview.speaker).toBe('interviewer')
  })
})
