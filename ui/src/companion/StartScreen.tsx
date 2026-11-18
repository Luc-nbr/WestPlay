import { useEffect, useState } from 'react'
import { EuroHubLogo } from '../shared/EuroHubMark'
import { useOmsiHub } from './omsihub'
import { Avatar } from './pages/Profile'
import { SignInForm } from './SignIn'
import type { HubWords } from './pages/hubWords'

/**
 * The start screen, as the companion opens, in Omsi-Hub's look: the night-blue ground with the road running
 * through it, the logo, and a card of glass. Signed in with an Omsi-Hub account, it welcomes the player back by
 * name; signed out, it offers the sign-in (an account is made on omsihub.com), or Euro-Hub without one. Start
 * (or Enter) goes on into the companion.
 */
export function StartScreen({ w, version, done }: { w: HubWords; version: string; done(): void }) {
  const account = useOmsiHub()
  const [leaving, setLeaving] = useState(false)
  const go = () => {
    if (leaving) return
    setLeaving(true)
    window.setTimeout(done, 420)
  }
  // Enter starts, once signed in (the form has its own Enter).
  useEffect(() => {
    if (account.kind !== 'signedIn') return
    const key = (event: KeyboardEvent) => event.key === 'Enter' && go()
    window.addEventListener('keydown', key)
    return () => window.removeEventListener('keydown', key)
  })

  return (
    <div className="eh-start" data-leaving={leaving || undefined}>
      <svg className="eh-start-roads" viewBox="0 0 1200 800" preserveAspectRatio="xMidYMid slice" aria-hidden>
        <path d="M-50 640 C 260 560 380 700 640 600 S 1040 420 1260 470" />
        <path d="M-50 210 C 200 260 420 120 700 190 S 1100 330 1260 250" />
        <path d="M180 -40 C 230 200 120 420 260 860" />
        <path d="M960 -40 C 900 240 1060 520 930 860" />
      </svg>
      <div className="eh-start-in">
        <div className="eh-start-logo">
          <EuroHubLogo height={84} road />
        </div>
        <p className="eh-start-tagline">{w.tagline}</p>
        <div className="eh-start-card eh-glass">
          {account.kind === 'loading' && <span className="eh-spinner" aria-hidden />}
          {account.kind === 'signedIn' && (
            <div className="eh-start-welcome">
              <Avatar name={account.profile.displayName} src={account.profile.avatar} size={64} />
              <b>{w.welcomeBack.replace('{name}', account.profile.displayName)}</b>
              <span>{account.profile.email}</span>
              <button type="button" className="eh-go" onClick={go} autoFocus>
                {w.start}
              </button>
            </div>
          )}
          {account.kind === 'signedOut' && (
            <>
              <h2>{w.signInTitle}</h2>
              <p>{w.signInSub}</p>
              <SignInForm w={w} />
              <button type="button" className="eh-ghost" onClick={go}>
                {w.without}
              </button>
            </>
          )}
        </div>
      </div>
      <footer className="eh-start-foot">
        <span>Euro-Hub {version}</span>
        <span>
          powered by <b>MOF Technologies</b> · Omsi-Hub
        </span>
      </footer>
    </div>
  )
}
