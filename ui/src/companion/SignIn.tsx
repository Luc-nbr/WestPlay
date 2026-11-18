import { type FormEvent, Fragment, type ReactNode, useState } from 'react'
import { createPortal } from 'react-dom'
import {
  acceptPrivacy,
  cancelSignInWith,
  openOmsiHub,
  privacyAccepted,
  type Provider,
  type SignInError,
  signIn,
  signInWith,
} from './omsihub'
import type { HubWords } from './pages/hubWords'
import privacyText from './privacy.md?raw'

const PROVIDERS: [Provider, string][] = [
  ['google', 'Google'],
  ['discord', 'Discord'],
]

/** The providers' marks (as Omsi-Hub's app draws them). */
const MARKS: Record<Provider, string> = {
  google:
    'M21.6 10.2H12v3.9h5.5c-.5 2.5-2.7 4.3-5.5 4.3a6.4 6.4 0 1 1 4.2-11.2l2.8-2.8A10.3 10.3 0 0 0 12 1.7a10.3 10.3 0 1 0 0 20.6c5.6 0 10-3.9 10-10.3 0-.6-.1-1.2-.4-1.8z',
  discord:
    'M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z',
}

/**
 * Signing in with an Omsi-Hub account: email and password, or Google or Discord in the browser; a new account on
 * the website. Before the first sign-in (and before the website opens to make an account), Euro-Hub's Privacy
 * Statement (privacy.md), to accept or not: nothing happens without it.
 */
export function SignInForm({ w }: { w: HubWords }) {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)
  const [waiting, setWaiting] = useState<Provider | null>(null)
  const [error, setError] = useState<SignInError | null>(null)
  // What waits on the privacy statement being accepted.
  const [asking, setAsking] = useState<(() => void) | null>(null)
  const [reading, setReading] = useState(false)

  const agreed = (then: () => void) => {
    if (privacyAccepted()) then()
    else setAsking(() => then)
  }
  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (busy || waiting || !email.trim() || !password) return
    agreed(() => {
      setBusy(true)
      setError(null)
      signIn(email, password)
        .catch((code: SignInError) => setError(code))
        .finally(() => setBusy(false))
    })
  }
  const withProvider = (provider: Provider) =>
    agreed(() => {
      setWaiting(provider)
      setError(null)
      signInWith(provider)
        .catch((code: SignInError | null) => code && setError(code))
        .finally(() => setWaiting(null))
    })
  const name = (provider: Provider) => PROVIDERS.find(([id]) => id === provider)?.[1] ?? ''

  return (
    <>
      {waiting ? (
        <div className="eh-wait">
          <span className="eh-spinner" aria-hidden />
          <p>{w.waiting.replace('{name}', name(waiting))}</p>
          <button type="button" className="eh-ghost" onClick={cancelSignInWith}>
            {w.cancel}
          </button>
        </div>
      ) : (
        <form className="eh-form" onSubmit={submit}>
          <div className="eh-providers">
            {PROVIDERS.map(([provider, label]) => (
              <button
                key={provider}
                type="button"
                className="eh-provider"
                data-provider={provider}
                disabled={busy}
                onClick={() => withProvider(provider)}
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path fill="currentColor" d={MARKS[provider]} />
                </svg>
                <span>{w.withProvider.replace('{name}', label)}</span>
              </button>
            ))}
          </div>
          <p className="eh-or">
            <span>{w.or}</span>
          </p>
          <label>
            <span>{w.email}</span>
            <input type="email" autoComplete="username" value={email} onChange={(event) => setEmail(event.target.value)} />
          </label>
          <label>
            <span>{w.password}</span>
            <input
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </label>
          {error && <p className="eh-form-error">{w.errors[error]}</p>}
          <button type="submit" className="eh-go" disabled={busy || !email.trim() || !password}>
            {busy ? w.signingIn : w.signIn}
          </button>
          <div className="eh-form-links">
            <button type="button" className="eh-link" onClick={() => agreed(() => openOmsiHub('signUp'))}>
              {w.create}
            </button>
            <button type="button" className="eh-link eh-link-quiet" onClick={() => setReading(true)}>
              {w.privacy}
            </button>
          </div>
        </form>
      )}
      {(asking || reading) && (
        <PrivacySheet
          w={w}
          ask={asking !== null}
          close={() => {
            setAsking(null)
            setReading(false)
          }}
          accept={() => {
            acceptPrivacy()
            const then = asking
            setAsking(null)
            then?.()
          }}
        />
      )}
    </>
  )
}

/** The privacy statement over everything: to read, or (`ask`) to accept before going on. */
export function PrivacySheet({
  w,
  ask,
  close,
  accept,
}: {
  w: HubWords
  ask: boolean
  close(): void
  accept(): void
}) {
  // Over the whole window: out of the glass card it's asked from (its blur would hold it inside), into the room,
  // whose colours it takes.
  return createPortal(
    <div className="eh-privacy" role="dialog" aria-modal="true" aria-label={w.privacyTitle}>
      <div className="eh-privacy-sheet">
        <h2>{w.privacyTitle}</h2>
        <div className="eh-privacy-text">{markdown(privacyText)}</div>
        {ask && <p className="eh-privacy-consent">{w.privacyConsent}</p>}
        <div className="eh-privacy-actions">
          <button type="button" className="eh-ghost" onClick={close}>
            {ask ? w.cancel : 'OK'}
          </button>
          {ask && (
            <button type="button" className="eh-go" onClick={accept}>
              {w.privacyAccept}
            </button>
          )}
        </div>
      </div>
    </div>,
    document.querySelector('.ob') ?? document.body,
  )
}

/** The statement's Markdown as it uses it: headings, paragraphs, lists, bold. */
function markdown(text: string): ReactNode[] {
  const blocks: ReactNode[] = []
  let list: string[] = []
  const flush = () => {
    if (list.length) {
      blocks.push(
        <ul key={blocks.length}>
          {list.map((item, at) => (
            <li key={at}>{bold(item)}</li>
          ))}
        </ul>,
      )
    }
    list = []
  }
  for (const line of text.split('\n')) {
    if (line.startsWith('- ')) {
      list.push(line.slice(2))
      continue
    }
    flush()
    if (!line.trim() || line.startsWith('# ')) continue
    if (line.startsWith('## ')) blocks.push(<h3 key={blocks.length}>{line.slice(3)}</h3>)
    else blocks.push(<p key={blocks.length}>{bold(line)}</p>)
  }
  flush()
  return blocks
}

function bold(text: string): ReactNode {
  return text.split(/\*\*(.+?)\*\*/g).map((part, at) => (at % 2 ? <b key={at}>{part}</b> : <Fragment key={at}>{part}</Fragment>))
}
