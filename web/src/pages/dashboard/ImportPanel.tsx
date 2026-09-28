// "Import desktop progress": choose the desktop app's JSON export, see what it
// holds, then merge it into the current profile. The server never downgrades,
// so importing an older file over newer progress is harmless.

import { useState } from 'react'
import clsx from 'clsx'
import { useImportProgress } from '@/api/hooks'
import type { Uuid } from '@/api/types'
import { describeError } from '@/pages/auth/errors'
import { plural } from '@/pages/shell/format'
import { MAX_IMPORT_BYTES, parseImport, type ParsedImport } from './importFile'
import styles from './dashboard.module.css'

export function ImportPanel({
  pid,
  profileName,
  knownSlugs,
}: {
  pid: Uuid
  profileName: string
  knownSlugs: ReadonlySet<string>
}) {
  const importProgress = useImportProgress(pid)
  const [picked, setPicked] = useState<{ name: string; parsed: ParsedImport } | null>(null)
  // Remounting the input is the reliable way to let the same file be chosen twice.
  const [inputKey, setInputKey] = useState(0)

  const choose = async (file: File | undefined) => {
    importProgress.reset()
    if (!file) return setPicked(null)
    const parsed: ParsedImport =
      file.size > MAX_IMPORT_BYTES
        ? { ok: false, error: 'That file is too large to be a progress export.' }
        : parseImport(await file.text(), knownSlugs)
    setPicked({ name: file.name, parsed })
  }

  const clear = () => {
    importProgress.reset()
    setPicked(null)
    setInputKey((k) => k + 1)
  }

  const parsed = picked?.parsed
  const skipped = parsed?.ok ? parsed.preview.skippedEntries + parsed.preview.skippedItems : 0

  return (
    <section className={clsx('card', styles.panel)} aria-labelledby="import-title">
      <h2 id="import-title" className={styles.panelTitle}>
        Import desktop progress
      </h2>
      <p className={styles.panelText}>
        Bring over what you did in the desktop app: choose the progress file it exported (JSON). It
        merges into <strong>{profileName}</strong> and never downgrades — a solved problem stays solved.
      </p>

      <div className={styles.fileRow}>
        <input
          key={inputKey}
          id="import-file"
          type="file"
          accept=".json,application/json"
          className={clsx('visually-hidden', styles.fileInput)}
          onChange={(e) => void choose(e.target.files?.[0])}
        />
        <label htmlFor="import-file" className="btn btn-ghost">
          Choose file…
        </label>
        {picked && <span className={styles.fileName}>{picked.name}</span>}
      </div>

      {parsed && !parsed.ok && (
        <p className="field-error" role="alert">
          {parsed.error}
        </p>
      )}

      {parsed?.ok && !importProgress.isSuccess && (
        <div className={styles.preview}>
          <ul className={styles.previewList}>
            <li>
              <span aria-hidden>✓</span> {parsed.preview.solved} solved
            </li>
            <li>
              <span aria-hidden>◐</span> {parsed.preview.attempted} attempted
            </li>
            <li>
              <span aria-hidden>★</span> {plural(parsed.preview.favourites, 'favourite')}
            </li>
            <li>
              <span aria-hidden>♪</span> {plural(parsed.preview.playlists, 'playlist')} (
              {plural(parsed.preview.playlistItems, 'problem')})
            </li>
          </ul>
          {skipped > 0 && (
            <p className={styles.panelText}>
              {plural(skipped, 'entry', 'entries')} will be skipped — those problems aren’t in the catalogue.
            </p>
          )}
          <div className={styles.previewActions}>
            <button
              type="button"
              className="btn btn-primary"
              disabled={importProgress.isPending}
              onClick={() => importProgress.mutate(parsed.request)}
            >
              {importProgress.isPending ? 'Importing…' : `Import into ${profileName}`}
            </button>
            <button type="button" className="mini-btn" onClick={clear}>
              cancel
            </button>
          </div>
        </div>
      )}

      {importProgress.isError && (
        <p className="field-error" role="alert">
          {describeError(importProgress.error).message}
        </p>
      )}
      {importProgress.isSuccess && (
        <p className={styles.done} role="status">
          ✓ Imported {plural(importProgress.data.progress_rows, 'progress row')} and{' '}
          {plural(importProgress.data.playlists, 'playlist')}.{' '}
          <button type="button" className="mini-btn" onClick={clear}>
            import another
          </button>
        </p>
      )}
    </section>
  )
}
