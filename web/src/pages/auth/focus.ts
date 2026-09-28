import type { RefObject } from 'react'

/**
 * Move focus to the first field (in form order) that has an error, so a
 * keyboard or screen-reader user lands on the thing to fix.
 */
export function focusFirstError(
  fields: Record<string, string>,
  order: readonly (readonly [name: string, ref: RefObject<HTMLInputElement | null>])[],
): void {
  for (const [name, ref] of order) {
    if (fields[name]) {
      ref.current?.focus()
      return
    }
  }
}
