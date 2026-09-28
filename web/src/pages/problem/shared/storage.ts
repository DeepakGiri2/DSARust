// Browser storage that never throws. Private mode, a full quota or a blocked
// origin must degrade to "this lasts for the tab", not break the workspace.

type Area = 'local' | 'session'

function area(which: Area): Storage | null {
  try {
    return which === 'local' ? window.localStorage : window.sessionStorage
  } catch {
    return null
  }
}

export function readStore(which: Area, key: string): string | null {
  try {
    return area(which)?.getItem(key) ?? null
  } catch {
    return null
  }
}

/** `null` removes the key. */
export function writeStore(which: Area, key: string, value: string | null): void {
  try {
    const s = area(which)
    if (!s) return
    if (value === null) s.removeItem(key)
    else s.setItem(key, value)
  } catch {
    // Quota or policy: nothing sensible to do but carry on.
  }
}

export function readJson<T>(which: Area, key: string, valid: (v: unknown) => v is T): T | null {
  const raw = readStore(which, key)
  if (raw === null) return null
  try {
    const parsed: unknown = JSON.parse(raw)
    return valid(parsed) ? parsed : null
  } catch {
    return null
  }
}
