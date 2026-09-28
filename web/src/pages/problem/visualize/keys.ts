// Debugger keyboard shortcuts — `Visualize::keys` on the desktop, plus the
// VS Code keys (F5, F10, F11) the transport hover texts already name.

export type DebugAction = 'back' | 'over' | 'in' | 'out' | 'play' | 'restart' | 'continue' | 'end'

type KeyLike = Pick<KeyboardEvent, 'key' | 'shiftKey' | 'ctrlKey' | 'metaKey' | 'altKey' | 'target'>

/** Somewhere keys mean text: form fields, rich text, the code editor. */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  if (target.closest('input, textarea, select, [contenteditable=""], [contenteditable="true"], .cm-editor')) return true
  return target instanceof HTMLElement && target.isContentEditable === true
}

/** Space already activates a focused control; stealing it would do two things at once. */
function activatesOnSpace(target: EventTarget | null): boolean {
  return (
    target instanceof Element &&
    target.closest('button, a[href], summary, [role="button"], [role="menuitem"], [role="menuitemcheckbox"], [role="tab"]') !==
      null
  )
}

/**
 * The action a key press asks of the debugger, or null to leave the event
 * alone. Browser and OS shortcuts (anything with Ctrl, Cmd or Alt) always
 * pass through — Ctrl+R must still reload the page.
 */
export function keyAction(e: KeyLike): DebugAction | null {
  if (e.ctrlKey || e.metaKey || e.altKey) return null
  if (isTypingTarget(e.target)) return null
  switch (e.key) {
    case 'ArrowLeft':
      return 'back'
    case 'ArrowRight':
    case 'F10':
      return 'over'
    case 'ArrowDown':
      return 'in'
    case 'ArrowUp':
      return 'out'
    case 'F11':
      return e.shiftKey ? 'out' : 'in'
    case ' ':
      return activatesOnSpace(e.target) ? null : 'play'
    case 'r':
    case 'R':
    case 'Home':
      return 'restart'
    case 'c':
    case 'C':
    case 'F5':
      return 'continue'
    case 'End':
      return 'end'
    default:
      return null
  }
}
