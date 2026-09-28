// Ported from the tests in crates/dsa-ai/src/parse.rs, plus the fence-label
// extension the web renders.
import { describe, expect, it } from 'vitest'
import { extractLastCodeBlock, extractThink, matchIndent, parseOptions, segments } from './parse'

describe('extractThink', () => {
  it('inline think tags are lifted out', () => {
    const { thinking, body } = extractThink('<think>hmm maybe</think>The answer is 4.')
    expect(thinking).toBe('hmm maybe')
    expect(body).toBe('The answer is 4.')
  })

  it('an unterminated think block is all reasoning', () => {
    const { thinking, body } = extractThink('<think>still going')
    expect(thinking).toBe('still going')
    expect(body).toBe('')
  })

  it('text without think tags is untouched', () => {
    const { thinking, body } = extractThink('just an answer')
    expect(thinking).toBe('')
    expect(body).toBe('just an answer')
  })
})

describe('segments', () => {
  it('segments alternate between prose and code', () => {
    const segs = segments('before\n```go\nx := 1\n```\nafter')
    expect(segs).toHaveLength(3)
    expect(segs[0].code).toBe(false)
    expect(segs[1].code).toBe(true)
    expect(segs[1].text).toBe('x := 1')
    expect(segs[2].code).toBe(false)
  })

  it('a streaming unterminated fence still shows its code', () => {
    const segs = segments('here:\n```go\nfunc f() {')
    expect(segs[1].code).toBe(true)
    expect(segs[1].text).toBe('func f() {')
  })

  it("a fence's language tag becomes the block's label", () => {
    const segs = segments('a\n```cpp title\nint x;\n```\n```\nraw\n```')
    expect(segs).toEqual([
      { code: false, text: 'a\n' },
      { code: true, text: 'int x;', lang: 'cpp' },
      { code: false, text: '\n' },
      { code: true, text: 'raw', lang: '' },
    ])
  })
})

describe('extractLastCodeBlock', () => {
  it('the last code block wins and prose is kept', () => {
    const reply = 'ISSUES:\n- off by one\n\nFIXED CODE:\n```go\nfunc f() {}\n```'
    const { code, rest } = extractLastCodeBlock(reply)
    expect(code).toBe('func f() {}')
    expect(rest).toContain('off by one')
    expect(rest).not.toContain('FIXED CODE')
  })

  it('two code blocks take the second', () => {
    const reply = '```go\nold\n```\nand the fix:\n```go\nnew\n```'
    expect(extractLastCodeBlock(reply).code).toBe('new')
  })

  it('a reply with no code reports none', () => {
    const { code, rest } = extractLastCodeBlock('- none found')
    expect(code).toBeNull()
    expect(rest).toBe('- none found')
  })
})

describe('parseOptions', () => {
  it('option lines become choices', () => {
    const { body, options } = parseOptions(
      'What would you like help with?\nOPTION: Explain the approach\n  OPTION:  Review my code ',
    )
    expect(body).toBe('What would you like help with?')
    expect(options).toEqual(['Explain the approach', 'Review my code'])
  })

  it('text without options keeps every line', () => {
    const { body, options } = parseOptions('line one\nline two')
    expect(body).toBe('line one\nline two')
    expect(options).toEqual([])
  })
})

describe('matchIndent', () => {
  it("indentation is converted to the original's style", () => {
    const original = 'func f() {\n\treturn 1\n}'
    const fixed = 'func f() {\n    return 2\n}'
    expect(matchIndent(fixed, original)).toContain('\treturn 2')
  })

  it('matching indentation is left alone', () => {
    const original = 'func f() {\n    return 1\n}'
    const fixed = 'func f() {\n    return 2\n}'
    expect(matchIndent(fixed, original)).toBe(fixed)
  })
})
