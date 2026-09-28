// The input editor under the code (`Visualize::input_panel`): one text field
// per declared input, "re-visualize" to record a new trace. The server parses
// and validates the text with the desktop's own rules and answers 422 with
// every violation, which are listed here.

import { memo, useState } from 'react'
import clsx from 'clsx'
import type { InputField } from '@/api/types'
import ui from '../shared/ui.module.css'
import styles from './Visualize.module.css'

export const InputPanel = memo(function InputPanel({
  fields,
  defaults,
  errors,
  busy,
  onApply,
}: {
  fields: InputField[]
  /** `Problem.default_fields` — the default input as editable text. */
  defaults: Record<string, string>
  errors: string[]
  busy: boolean
  onApply: (fields: Record<string, string>) => void
}) {
  const [texts, setTexts] = useState<Record<string, string>>(() =>
    Object.fromEntries(fields.map((f) => [f.name, defaults[f.name] ?? ''])),
  )

  return (
    <section className={styles.inputs} aria-label="Input">
      <h3 className={clsx('section-label', ui.head, ui.asIs)}>input</h3>
      <form
        className={styles.inputRow}
        onSubmit={(e) => {
          e.preventDefault()
          onApply(texts)
        }}
      >
        {fields.map((f) => {
          const id = `viz-input-${f.name}`
          const text = texts[f.name] ?? ''
          return (
            <div key={f.name} className={styles.field}>
              <label htmlFor={id} className={styles.fieldLabel}>
                {f.label || f.name}
              </label>
              <input
                id={id}
                className={clsx('input mono', styles.fieldInput, text.length > 20 && styles.wide)}
                value={text}
                onChange={(e) => setTexts((t) => ({ ...t, [f.name]: e.target.value }))}
                spellCheck={false}
                autoComplete="off"
                title={f.help ?? undefined}
                aria-describedby={f.help ? `${id}-help` : undefined}
                aria-invalid={errors.length > 0 || undefined}
              />
              {f.help && (
                <span id={`${id}-help`} className={styles.help}>
                  {f.help}
                </span>
              )}
            </div>
          )
        })}
        <button type="submit" className={ui.apply} disabled={busy}>
          {busy ? '⏳ tracing…' : 're-visualize'}
        </button>
      </form>
      {errors.length > 0 && (
        <ul className={styles.inputErrors} role="alert">
          {errors.map((e) => (
            <li key={e}>{e}</li>
          ))}
        </ul>
      )}
    </section>
  )
})
