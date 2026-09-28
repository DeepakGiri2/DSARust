// Adapted from the tests in crates/dsa-app/src/highlight.rs: the roles are
// the desktop's, the tokenizer is Lezer's.
import { describe, expect, it } from 'vitest'
import { minimalChange } from './CodeEditor'
import { HL, highlightLines, type HlSpan } from './highlight'

const cls = (lines: HlSpan[][], text: string) => lines.flat().find((s) => s.text === text)?.cls ?? null
const joined = (line: HlSpan[]) => line.map((s) => s.text).join('')

describe('highlightLines', () => {
  it('classifies keywords, numbers and punctuation', () => {
    const lines = highlightLines('func f() {\n    for i := 0; i < 10; i++ {\n    }\n}\n', 'go')
    expect(cls(lines, 'func')).toBe(HL.keyword)
    expect(cls(lines, 'for')).toBe(HL.keyword)
    expect(cls(lines, '10')).toBe(HL.number)
    expect(cls(lines, '{')).toBe(HL.punct)
  })

  it('lets a comment swallow the rest of the line', () => {
    const lines = highlightLines('func f() {\n    x := 1 // set x to one\n}\n', 'go')
    expect(cls(lines, '// set x to one')).toBe(HL.comment)
  })

  it('uses # comments for Python, not slashes', () => {
    expect(cls(highlightLines('seen = {}  # a map\n', 'python'), '# a map')).toBe(HL.comment)
  })

  it('keeps strings whole across escapes and embedded comment markers', () => {
    const lines = highlightLines('func f() {\n    s := "a//b\\" c" + t\n}\n', 'go')
    expect(cls(lines, '"a//b\\" c"')).toBe(HL.string)
    expect(lines.flat().some((s) => s.cls === HL.comment)).toBe(false)
  })

  it('reads type names as types and builtin C-family types as keywords', () => {
    const java = highlightLines('class A {\n    Map<Integer, Integer> seen = new HashMap<>();\n    int n;\n}\n', 'java')
    expect(cls(java, 'Map')).toBe(HL.type)
    expect(cls(java, 'new')).toBe(HL.keyword)
    expect(cls(java, 'int')).toBe(HL.keyword)
  })

  it('accounts for every character, line by line', () => {
    const src = 'int f(int i) {\n    if (seen.count(need)) { return {seen[need], i}; }\n    // π ≈ 3.14 — done\n}\n'
    const lines = highlightLines(src, 'cpp')
    expect(lines.map(joined)).toEqual(src.split('\n').slice(0, -1))
  })

  it("counts lines like Rust's str::lines, so tag_lines line up", () => {
    expect(highlightLines('a\nb\n', 'go')).toHaveLength(2)
    expect(highlightLines('a\nb', 'go')).toHaveLength(2)
    expect(highlightLines('a\n\n', 'go')).toHaveLength(2)
    expect(highlightLines('', 'go')).toHaveLength(0)
  })

  it('shows an unknown syntax as plain text', () => {
    expect(highlightLines('x := 1', 'brainfuck')).toEqual([[{ text: 'x := 1', cls: null }]])
  })
})

describe('minimalChange', () => {
  it('replaces only the part that differs', () => {
    expect(minimalChange('return 1\n', 'return 2\n')).toEqual({ from: 7, to: 8, insert: '2' })
    expect(minimalChange('abc', 'abc')).toEqual({ from: 3, to: 3, insert: '' })
    expect(minimalChange('', 'new')).toEqual({ from: 0, to: 0, insert: 'new' })
    expect(minimalChange('aaa', 'aa')).toEqual({ from: 2, to: 3, insert: '' })
  })
})
