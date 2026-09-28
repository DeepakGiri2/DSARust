import { useEffect, useRef, useState, type ReactNode } from 'react'
import type React from 'react'
import clsx from 'clsx'
import styles from './Menu.module.css'

/**
 * A dropdown anchored under its trigger. Closes on outside click and Esc.
 * `children` may be a render function receiving `close`, for items that act
 * and then dismiss the menu.
 */
export function Menu({
  label,
  children,
  align = 'left',
  className,
  buttonClassName = 'mini-btn',
  title,
}: {
  label: ReactNode
  children: ReactNode | ((close: () => void) => ReactNode)
  align?: 'left' | 'right'
  className?: string
  buttonClassName?: string
  title?: string
}) {
  const [open, setOpen] = useState(false)
  const root = useRef<HTMLDivElement>(null)
  const trigger = useRef<HTMLButtonElement>(null)
  const popup = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    // Keyboard users land on the first item, as in a native menu.
    focusables(popup.current)[0]?.focus()
    const onDown = (e: MouseEvent) => {
      if (root.current && !root.current.contains(e.target as Node)) setOpen(false)
    }
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpen(false)
        trigger.current?.focus()
      }
    }
    document.addEventListener('mousedown', onDown)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('mousedown', onDown)
      document.removeEventListener('keydown', onKey)
    }
  }, [open])

  const onPopupKey = (e: React.KeyboardEvent) => {
    const list = focusables(popup.current)
    if (list.length === 0) return
    const i = list.indexOf(document.activeElement as HTMLElement)
    const to =
      e.key === 'ArrowDown' ? (i + 1) % list.length
      : e.key === 'ArrowUp' ? (i - 1 + list.length) % list.length
      : e.key === 'Home' ? 0
      : e.key === 'End' ? list.length - 1
      : -1
    if (to < 0) return
    e.preventDefault()
    list[to].focus()
  }

  const close = () => setOpen(false)
  return (
    <div ref={root} className={clsx(styles.root, className)}>
      <button
        ref={trigger}
        type="button"
        className={buttonClassName}
        aria-haspopup="menu"
        aria-expanded={open}
        title={title}
        onClick={() => setOpen((o) => !o)}
      >
        {label}
      </button>
      {open && (
        <div
          ref={popup}
          className={clsx(styles.popup, align === 'right' && styles.right)}
          role="menu"
          onKeyDown={onPopupKey}
        >
          {typeof children === 'function' ? children(close) : children}
        </div>
      )}
    </div>
  )
}

/** What the arrow keys walk: items, and any checkbox or link placed in a menu. */
const MENU_FOCUSABLE = '[role="menuitem"], [role="menuitemcheckbox"], button, input, a[href]'

function focusables(popup: HTMLElement | null): HTMLElement[] {
  return [...(popup?.querySelectorAll<HTMLElement>(MENU_FOCUSABLE) ?? [])].filter((el) => !el.hasAttribute('disabled'))
}

/** A clickable row inside a `Menu`. */
export function MenuItem({
  children,
  onSelect,
  disabled,
  danger,
}: {
  children: ReactNode
  onSelect: () => void
  disabled?: boolean
  danger?: boolean
}) {
  return (
    <button
      type="button"
      role="menuitem"
      className={clsx(styles.item, danger && styles.danger)}
      disabled={disabled}
      onClick={onSelect}
    >
      {children}
    </button>
  )
}

export function MenuSeparator() {
  return <div className={styles.sep} role="separator" />
}
