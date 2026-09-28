import { Link } from 'react-router'
import { Modal } from '@/ui'
import styles from './CatalogPage.module.css'

/** What a guest's click on a star opens: progress needs somewhere to live. */
export function SignInPrompt({ open, onClose, next }: { open: boolean; onClose: () => void; next: string }) {
  return (
    <Modal open={open} onClose={onClose} title="☆ Keep track of it" labelledBy="signin-prompt-title" width={440}>
      <p className={styles.modalNote}>
        Sign in to star problems, tick off what you have solved and build playlists. Accounts are
        free, and your progress follows you to every device.
      </p>
      <div className={styles.promptActions}>
        <Link className="btn btn-primary" to={`/signup?next=${next}`}>
          Create a free account
        </Link>
        <Link className="btn btn-ghost" to={`/login?next=${next}`}>
          Sign in
        </Link>
      </div>
    </Modal>
  )
}
