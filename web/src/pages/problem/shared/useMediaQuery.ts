import { useCallback, useSyncExternalStore } from 'react'

/** Breakpoints of the workspace: side columns become tabs, then everything does. */
export const COMPACT_QUERY = '(max-width: 1100px)'
export const PHONE_QUERY = '(max-width: 700px)'

export function useMediaQuery(query: string): boolean {
  const subscribe = useCallback(
    (onChange: () => void) => {
      const list = window.matchMedia(query)
      list.addEventListener('change', onChange)
      return () => list.removeEventListener('change', onChange)
    },
    [query],
  )
  return useSyncExternalStore(
    subscribe,
    () => window.matchMedia(query).matches,
    () => false,
  )
}
