// Turning a model's free-form reply into something the panel can render — a
// port of `crates/dsa-ai/src/parse.rs`.
//
// These are the fiddly bits of the assistant and the ones most likely to break
// silently on a model that formats slightly differently, so they are pure
// functions with tests rather than inline string poking in the components.
// Keep them in step with the Rust: the desktop and the web must read the same
// reply the same way.

import { lines, trimEndMatches, trimStartMatches } from './str'

const THINK_OPEN = '<think>'
const THINK_CLOSE = '</think>'
const FENCE = '```'

/**
 * Some models emit reasoning inline as `<think>…</think>` instead of on the
 * separate thinking channel. Pull it out so it renders collapsed either way.
 * `thinking` is kept as sent; `body` is trimmed.
 */
export function extractThink(content: string): { thinking: string; body: string } {
  let thinking = ''
  let out = ''
  let rest = content

  for (let start = rest.indexOf(THINK_OPEN); start >= 0; start = rest.indexOf(THINK_OPEN)) {
    out += rest.slice(0, start)
    const after = rest.slice(start + THINK_OPEN.length)
    const end = after.indexOf(THINK_CLOSE)
    if (end >= 0) {
      thinking += after.slice(0, end)
      rest = after.slice(end + THINK_CLOSE.length)
    } else {
      // Unterminated: the model is still streaming its reasoning.
      thinking += after
      rest = ''
    }
  }
  out += rest
  return { thinking, body: out.trim() }
}

/**
 * A fenced code block, or the text around one.
 *
 * `lang` is the fence's info string (` ```go `), which the Rust skips; the web
 * shows it as the block's label. It is empty when the fence has none.
 */
export type Segment = { code: false; text: string } | { code: true; text: string; lang: string }

/**
 * Split a reply into plain-text and fenced-code segments, in order.
 * An unterminated final fence still yields its (partial) code, which is what
 * makes streaming look right rather than hiding the block until it closes.
 */
export function segments(text: string): Segment[] {
  const out: Segment[] = []
  let rest = text

  for (let open = rest.indexOf(FENCE); open >= 0; open = rest.indexOf(FENCE)) {
    if (open > 0) out.push({ code: false, text: rest.slice(0, open) })
    const after = rest.slice(open + FENCE.length)
    // Skip the optional language tag on the fence line.
    const newline = after.indexOf('\n')
    const bodyStart = newline >= 0 ? newline + 1 : after.length
    const lang = after.slice(0, bodyStart).trim().split(/\s+/)[0]
    const body = after.slice(bodyStart)
    const close = body.indexOf(FENCE)
    if (close >= 0) {
      out.push({ code: true, text: body.slice(0, close).trimEnd(), lang })
      rest = body.slice(close + FENCE.length)
    } else {
      out.push({ code: true, text: body, lang })
      rest = ''
    }
  }
  if (rest !== '') out.push({ code: false, text: rest })
  return out
}

/**
 * The last fenced code block plus everything else, which is how the fix mode
 * separates "issues" prose from the corrected function. Earlier code blocks
 * are dropped from `rest`: in a Fix reply they are quotes of the old code.
 */
export function extractLastCodeBlock(text: string): { code: string | null; rest: string } {
  const segs = segments(text)
  const last = segs.findLastIndex((s) => s.code)
  if (last < 0) return { code: null, rest: text }

  const prose = segs
    .filter((s) => !s.code)
    .map((s) => s.text)
    .join('')
  const rest = trimEndMatches(trimEndMatches(prose.trimEnd(), 'FIXED CODE:'), 'Fixed code:').trim()
  return { code: segs[last].text, rest }
}

/**
 * Guide mode asks clarifying questions by emitting `OPTION: …` lines; those
 * become clickable choices instead of text.
 */
export function parseOptions(content: string): { body: string; options: string[] } {
  const options: string[] = []
  const body: string[] = []
  for (const line of lines(content)) {
    const trimmed = line.trimStart()
    if (trimmed.startsWith('OPTION:')) options.push(trimmed.slice('OPTION:'.length).trim())
    else body.push(line)
  }
  return { body: body.join('\n').trim(), options }
}

/**
 * Small models often reformat indentation (tabs vs spaces), which would make
 * the diff flag every single line. Convert the fix's leading whitespace back
 * to the original's style.
 */
export function matchIndent(fixed: string, original: string): string {
  const origTabs = lines(original).some((l) => l.startsWith('\t'))
  const fixedTabs = lines(fixed).some((l) => l.startsWith('\t'))
  if (origTabs === fixedTabs) return fixed

  return lines(fixed)
    .map((line) => {
      if (fixedTabs) {
        const rest = trimStartMatches(line, '\t')
        return '    '.repeat(line.length - rest.length) + rest
      }
      const spaces = line.length - trimStartMatches(line, ' ').length
      // Whole groups of four become tabs; a ragged remainder stays as spaces.
      return '\t'.repeat(Math.floor(spaces / 4)) + line.slice(spaces - (spaces % 4))
    })
    .join('\n')
}
