import { useState } from 'react'
import { initials, openOmsiHub, signOut, useOmsiHub } from '../omsihub'
import { n } from '../room/room'
import { Button, Group, Panel, Row } from '../room/parts'
import { GLOW, useScreenAt } from '../room/screen'
import { SignInForm } from '../SignIn'
import { hubWords } from './hubWords'
import type { PageProps } from './page'

/**
 * Profile: the player's Omsi-Hub account in Euro-Hub. Signed in: their photo, name and friend code (as on
 * omsihub.com), the account managed on the website, signing out. Signed out: the sign-in. On the room's screen,
 * Home.
 */
export function Profile({ cw }: PageProps) {
  const w = hubWords(cw)
  const account = useOmsiHub()
  const [copied, setCopied] = useState(false)
  useScreenAt('home', GLOW.home, null)

  if (account.kind !== 'signedIn') {
    return (
      <Panel place="page" title={w.signedOutTitle} sub={w.signedOutSub}>
        {account.kind === 'signedOut' && (
          <div className="eh-profile-signin ob-in" style={n(4)}>
            <SignInForm w={w} />
          </div>
        )}
      </Panel>
    )
  }

  const { profile } = account
  const copy = () => {
    void navigator.clipboard?.writeText(profile.friendCode).then(() => {
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1600)
    })
  }
  return (
    <Panel
      place="page"
      title={w.profileTitle}
      sub={w.profileSub}
      action={
        <>
          <Button glass onClick={() => openOmsiHub('account')}>
            {w.manage}
          </Button>
          <Button glass onClick={() => void signOut()}>
            {w.signOut}
          </Button>
        </>
      }
    >
      <div className="eh-profile-card ob-in" style={n(4)}>
        <Avatar name={profile.displayName} src={profile.avatar} size={72} />
        <div>
          <b>{profile.displayName}</b>
          <span>{w.driver} · Omsi-Hub &amp; Euro-Hub</span>
        </div>
      </div>
      <Group title={w.account} place={5}>
        <Row title={w.name} value={profile.displayName} />
        {profile.friendCode && (
          <Row
            title={w.friendCode}
            value={profile.friendCode}
            accessory={
              <button type="button" className="eh-copy" onClick={copy}>
                {copied ? w.copied : w.copy}
              </button>
            }
          />
        )}
        <Row title={w.emailRow} value={profile.email} />
      </Group>
    </Panel>
  )
}

/** A player's photo, or their initials on the accent's blue. */
export function Avatar({ name, src, size }: { name: string; src: string | null; size: number }) {
  const [broken, setBroken] = useState(false)
  return (
    <span className="eh-avatar" style={{ width: size, height: size, fontSize: size * 0.38 }}>
      {src && !broken ? <img src={src} alt="" onError={() => setBroken(true)} /> : initials(name)}
    </span>
  )
}
