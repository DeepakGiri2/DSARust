// Client-side checks for the account forms. They exist to answer instantly
// what the server would answer a round-trip later — the server still decides,
// and its field errors are shown the same way.

export const MIN_PASSWORD = 10

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

export function validateEmail(value: string): string | null {
  const v = value.trim()
  if (!v) return 'Enter your email address.'
  if (!EMAIL.test(v)) return 'That doesn’t look like an email address.'
  return null
}

/** For signing in: any non-empty password is worth sending. */
export function validateCurrentPassword(value: string): string | null {
  return value ? null : 'Enter your password.'
}

/** For choosing one: the server's minimum, checked before the round-trip. */
export function validateNewPassword(value: string): string | null {
  if (!value) return 'Choose a password.'
  if (value.length < MIN_PASSWORD) return `Use at least ${MIN_PASSWORD} characters.`
  return null
}

export function validateDisplayName(value: string): string | null {
  return value.trim() ? null : 'Tell us what to call you.'
}

export type StrengthScore = 0 | 1 | 2 | 3 | 4

export interface Strength {
  score: StrengthScore
  label: string
}

const LABELS: Record<StrengthScore, string> = {
  0: 'too short',
  1: 'weak',
  2: 'fair',
  3: 'good',
  4: 'strong',
}

// Fragments that make a long password guessable anyway.
const COMMON = ['password', 'passw0rd', 'qwerty', 'letmein', 'welcome', 'iloveyou', 'admin', 'abc123', '123456', '111111']

/** Runs like "abcdefghij" or "9876543210": long, and no harder than one character. */
function isSequence(pw: string): boolean {
  if (pw.length < 3) return false
  const step = pw.charCodeAt(1) - pw.charCodeAt(0)
  if (Math.abs(step) !== 1) return false
  for (let i = 2; i < pw.length; i++) {
    if (pw.charCodeAt(i) - pw.charCodeAt(i - 1) !== step) return false
  }
  return true
}

/**
 * A rough strength estimate for the meter. Length is most of it; variety of
 * character classes helps; a common word or a single repeated character caps
 * it at "weak" however long it is. It advises — only the minimum length is
 * enforced.
 */
export function passwordStrength(pw: string): Strength {
  if (pw.length < MIN_PASSWORD) return { score: 0, label: LABELS[0] }
  const lower = pw.toLowerCase()
  if (/^(.)\1*$/.test(pw) || isSequence(pw) || COMMON.some((w) => lower.includes(w))) {
    return { score: 1, label: LABELS[1] }
  }
  const classes = [/[a-z]/, /[A-Z]/, /\d/, /[^A-Za-z0-9]/].filter((r) => r.test(pw)).length
  let score = 1
  if (pw.length >= 14) score++
  if (classes >= 3) score++
  if (pw.length >= 20 || (classes === 4 && pw.length >= 12)) score++
  // Long runs of one character add length without adding guesses.
  if (/(.)\1{3,}/.test(pw)) score--
  const s = Math.min(4, Math.max(1, score)) as StrengthScore
  return { score: s, label: LABELS[s] }
}
