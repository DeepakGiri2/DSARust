import clsx from 'clsx'
import type { ReactNode } from 'react'

export interface SegOption<T extends string> {
  value: T
  label: ReactNode
  title?: string
  disabled?: boolean
}

/** `.seg` — the segmented control used for tiers, languages and tabs. */
export function Seg<T extends string>({
  value,
  options,
  onChange,
  small,
  className,
  'aria-label': ariaLabel,
}: {
  value: T
  options: SegOption<T>[]
  onChange: (v: T) => void
  small?: boolean
  className?: string
  'aria-label'?: string
}) {
  return (
    <div className={clsx('seg', small && 'small', className)} role="group" aria-label={ariaLabel}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          aria-pressed={o.value === value}
          title={o.title}
          disabled={o.disabled}
          onClick={() => o.value !== value && onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  )
}
