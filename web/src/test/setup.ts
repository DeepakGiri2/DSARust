import '@testing-library/jest-dom/vitest'

// React Router builds a `Request` per navigation around jsdom's AbortSignal,
// which Node's own `Request` rejects — so under jsdom every data-router
// navigation would throw. Nothing in the tests aborts a navigation, so the
// signal can simply be dropped.
class NavigationRequest extends Request {
  constructor(input: RequestInfo | URL, init?: RequestInit) {
    if (!init) {
      super(input)
      return
    }
    const { signal: _signal, ...rest } = init
    super(input, rest)
  }
}
globalThis.Request = NavigationRequest

// jsdom has no canvas; renderer tests that need one mock getContext themselves.
// It also has no matchMedia, which some components read for reduced motion.
if (!window.matchMedia) {
  window.matchMedia = (query: string) =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }) as MediaQueryList
}
