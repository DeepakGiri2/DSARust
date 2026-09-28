import { describe, expect, it } from 'vitest'
import { lines, trimEndMatches, trimStartMatches } from './str'

describe('lines — Rust str::lines()', () => {
  it('drops the final line ending but keeps interior blank lines', () => {
    expect(lines('a\nb\n')).toEqual(['a', 'b'])
    expect(lines('a\n\nb')).toEqual(['a', '', 'b'])
    expect(lines('a\n\n')).toEqual(['a', ''])
  })

  it('has no lines for empty text and one empty line for a lone newline', () => {
    expect(lines('')).toEqual([])
    expect(lines('\n')).toEqual([''])
  })

  it('strips the carriage return of a CRLF ending, and only that one', () => {
    expect(lines('a\r\nb\r\n')).toEqual(['a', 'b'])
    expect(lines('a\rb')).toEqual(['a\rb'])
    expect(lines('a\r')).toEqual(['a\r'])
  })
})

describe('trim_start_matches / trim_end_matches', () => {
  it('strips every repetition at that end and nothing else', () => {
    expect(trimStartMatches('\t\t\tx\t', '\t')).toBe('x\t')
    expect(trimStartMatches('ISSUES:ISSUES: a', 'ISSUES:')).toBe(' a')
    expect(trimEndMatches('abcFIXED CODE:FIXED CODE:', 'FIXED CODE:')).toBe('abc')
    expect(trimEndMatches('x  ', ' ')).toBe('x')
  })

  it('leaves text without the pattern untouched', () => {
    expect(trimStartMatches('abc', 'z')).toBe('abc')
    expect(trimEndMatches('abc', 'z')).toBe('abc')
    expect(trimEndMatches('', 'z')).toBe('')
  })
})
