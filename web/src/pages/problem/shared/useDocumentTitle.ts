import { useEffect } from 'react'

/** Set the tab title while mounted; put the previous one back on the way out. */
export function useDocumentTitle(title: string): void {
  useEffect(() => {
    const previous = document.title
    document.title = title
    return () => {
      document.title = previous
    }
  }, [title])
}
