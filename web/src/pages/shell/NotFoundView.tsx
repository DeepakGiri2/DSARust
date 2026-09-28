import { Link } from 'react-router'
import styles from './NotFound.module.css'

/**
 * "Nothing here". Also what a non-admin sees at /admin — a 404 rather than a
 * 403, the same answer the API gives, so the page does not advertise itself.
 */
export function NotFoundView() {
  return (
    <div className={styles.page}>
      <p className={styles.code} aria-hidden>
        404
      </p>
      <h1 className={styles.title}>
        Nothing <span className="grad-text">here</span>
      </h1>
      <p className="muted">That page doesn’t exist, or it moved. The problems are all still where you left them.</p>
      <Link className="btn btn-primary" to="/">
        ← back to the problem list
      </Link>
    </div>
  )
}
