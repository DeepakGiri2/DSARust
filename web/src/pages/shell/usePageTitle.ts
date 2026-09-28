import { useEffect } from 'react'

const SITE = 'DSA Visualized'

/** The bare site title, for the home page and for whatever has no title of its own. */
export const SITE_TITLE = `${SITE} — step-through algorithm debugger`

/**
 * `document.title` for a screen: "Dashboard · DSA Visualized", or the site
 * title for `null`. The previous title comes back on unmount, so a screen that
 * sets none (a modal route, an error boundary) never inherits a stale one.
 */
export function usePageTitle(title: string | null) {
  useEffect(() => {
    const previous = document.title
    document.title = title ? `${title} · ${SITE}` : SITE_TITLE
    return () => {
      document.title = previous
    }
  }, [title])
}
