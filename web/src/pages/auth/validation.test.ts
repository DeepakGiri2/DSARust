import { describe, expect, it } from 'vitest'
import { ApiError } from '@/api/client'
import { describeError } from './errors'
import { afterAuth } from './redirect'
import { MIN_PASSWORD, passwordStrength, validateEmail, validateNewPassword } from './validation'

describe('form validation', () => {
  it('checks email addresses loosely', () => {
    expect(validateEmail('')).toBe('Enter your email address.')
    expect(validateEmail('alex')).toMatch(/doesn’t look like/)
    expect(validateEmail(' alex@example.com ')).toBeNull()
  })

  it('enforces the minimum password length before the round-trip', () => {
    expect(validateNewPassword('')).toBe('Choose a password.')
    expect(validateNewPassword('a'.repeat(MIN_PASSWORD - 1))).toMatch(/at least 10/)
    expect(validateNewPassword('correct horse')).toBeNull()
  })

  it('rates strength by length and variety, and distrusts the obvious', () => {
    expect(passwordStrength('short').score).toBe(0)
    expect(passwordStrength('aaaaaaaaaaaaaaaaaaaa').score).toBe(1)
    expect(passwordStrength('abcdefghijkl').score).toBe(1)
    expect(passwordStrength('MyPassword2024!').score).toBe(1)
    expect(passwordStrength('lowercaseonly').score).toBe(1)
    expect(passwordStrength('lowercase and spaces').score).toBeGreaterThanOrEqual(2)
    expect(passwordStrength('Tr0ub4dor&3xtra').score).toBe(4)
  })
})

describe('describeError', () => {
  const err = (status: number, code: ConstructorParameters<typeof ApiError>[1], details?: Record<string, unknown>) =>
    new ApiError(status, code, 'server says no', details)

  it('puts field errors next to their fields', () => {
    const d = describeError(err(422, 'validation', { fields: { email: 'taken' } }))
    expect(d).toEqual({ message: null, fields: { email: 'taken' } })
  })

  it('joins a set of messages when no field is named', () => {
    expect(describeError(err(422, 'validation', { errors: ['a', 'b'] })).message).toBe('a b')
  })

  it('says how long to wait when throttled or locked', () => {
    expect(describeError(err(429, 'rate_limited', { retry_after_secs: 90 })).message).toBe(
      'Too many attempts. Try again in 2 minutes.',
    )
    expect(describeError(err(423, 'account_locked', { retry_after_secs: 900 })).message).toMatch(
      /locked for 15 minutes/,
    )
  })

  it('lets the form word a code better than the server', () => {
    expect(describeError(err(409, 'conflict'), { conflict: 'That name is taken.' }).message).toBe('That name is taken.')
    expect(describeError(err(500, 'internal')).message).toBe('server says no')
    expect(describeError(new Error('offline')).message).toBe('offline')
  })
})

describe('afterAuth', () => {
  it('goes to ?next, through the picker when a profile must be chosen', () => {
    expect(afterAuth('/dashboard', false)).toBe('/dashboard')
    expect(afterAuth(encodeURIComponent('/problems/two-sum'), true)).toBe(
      `/profiles?next=${encodeURIComponent('/problems/two-sum')}`,
    )
  })

  it('never bounces back to a sign-in page or off the site', () => {
    expect(afterAuth('/login?next=/x', false)).toBe('/')
    expect(afterAuth('/signup', false)).toBe('/')
    expect(afterAuth('https://evil.example', false)).toBe('/')
    expect(afterAuth('//evil.example', false)).toBe('/')
    expect(afterAuth(null, false)).toBe('/')
    expect(afterAuth('/verify-email', false)).toBe('/verify-email')
  })
})
