import { useEffect, useState } from 'react'
import { call, IN_APP } from './bridge'

/**
 * Signing in with an Omsi-Hub account (the team's own product, omsihub.com): the same accounts, on Omsi-Hub's
 * Supabase project, through its Auth routes (`/auth/v1`) and its database (`/rest/v1`), as Omsi-Hub's app does
 * (Omsi-Hub's docs/ONLINE.md). The URL and the publishable key are public by design; what a signed-in player may
 * read is decided by the project's row-level security. An account is made on the website.
 *
 * The session (its refresh token) is kept in this window's storage, so the player stays signed in; the access
 * token is refreshed as it runs out.
 */
const URL = 'https://mlatjtjjdfifbgddcxzu.supabase.co'
const KEY = 'sb_publishable_LF17d5vnctmpyH1h6dRprw_s3CGcD-P'
const STORE = 'eurohub.omsihub'

interface Session {
  access: string
  refresh: string
  /** When the access token runs out (ms). */
  expires: number
  userId: string
  email: string
  /** The name chosen on the website, for a profile not made yet. */
  metaName: string
}

export interface Profile {
  displayName: string
  friendCode: string
  email: string
  /** The photo's public address, if there is one. */
  avatar: string | null
  /** Since when the account exists (ms), if known. */
  since: number | null
}

export type OmsiHubState =
  | { kind: 'loading' }
  | { kind: 'signedOut' }
  | { kind: 'signedIn'; profile: Profile }

/** A sign-in that went wrong, as a code the words name. */
export type SignInError = 'wrong' | 'unconfirmed' | 'offline' | 'timeout' | 'other'

/** Who signs the player in, in the browser. */
export type Provider = 'google' | 'discord'

function load(): Session | null {
  try {
    const raw = localStorage.getItem(STORE)
    return raw ? (JSON.parse(raw) as Session) : null
  } catch {
    return null
  }
}

function save(session: Session | null) {
  try {
    if (session) localStorage.setItem(STORE, JSON.stringify(session))
    else localStorage.removeItem(STORE)
  } catch {
    // Not kept: signed in for this window only.
  }
}

interface TokenAnswer {
  access_token: string
  refresh_token: string
  expires_in: number
  user: { id: string; email?: string; created_at?: string; user_metadata?: { display_name?: string } }
}

function sessionOf(answer: TokenAnswer): Session {
  return {
    access: answer.access_token,
    refresh: answer.refresh_token,
    expires: Date.now() + answer.expires_in * 1000,
    userId: answer.user.id,
    email: answer.user.email ?? '',
    metaName: answer.user.user_metadata?.display_name ?? '',
  }
}

class AuthError extends Error {
  readonly code: SignInError
  constructor(code: SignInError) {
    super(code)
    this.code = code
  }
}

async function token(grant: 'password' | 'refresh_token' | 'pkce', body: object): Promise<Session> {
  let response: Response
  try {
    response = await fetch(`${URL}/auth/v1/token?grant_type=${grant}`, {
      method: 'POST',
      headers: { apikey: KEY, 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
  } catch {
    throw new AuthError('offline')
  }
  const answer = (await response.json().catch(() => ({}))) as Partial<TokenAnswer> & { error_code?: string }
  if (!response.ok || !answer.access_token) {
    if (answer.error_code === 'email_not_confirmed') throw new AuthError('unconfirmed')
    if (answer.error_code === 'invalid_credentials' || response.status === 400) throw new AuthError('wrong')
    throw new AuthError('other')
  }
  return sessionOf(answer as TokenAnswer)
}

/** A session whose access token is good for a while yet (refreshed if not); null once signed out. */
async function fresh(): Promise<Session | null> {
  const session = load()
  if (!session) return null
  if (session.expires - Date.now() > 60_000) return session
  try {
    const renewed = await token('refresh_token', { refresh_token: session.refresh })
    save(renewed)
    return renewed
  } catch (error) {
    // Refused: signed out. Offline: the old one, tried as it is.
    if (error instanceof AuthError && error.code !== 'offline') {
      save(null)
      return null
    }
    return session
  }
}

async function rest(session: Session, method: string, path: string, body?: object): Promise<unknown> {
  const response = await fetch(`${URL}/rest/v1${path}`, {
    method,
    headers: {
      apikey: KEY,
      Authorization: `Bearer ${session.access}`,
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: body ? JSON.stringify(body) : undefined,
  })
  if (!response.ok) throw new Error(`${response.status}`)
  return response.status === 204 ? null : response.json()
}

interface ProfileRow {
  display_name?: string
  friend_code?: string
  avatar?: string | null
  created_at?: string
}

async function readProfile(session: Session): Promise<Profile> {
  const query = `/profiles?select=display_name,friend_code,avatar&user_id=eq.${encodeURIComponent(session.userId)}&limit=1`
  let rows = (await rest(session, 'GET', query)) as ProfileRow[]
  // The first sign-in anywhere: the profile made from the name chosen on the website (as Omsi-Hub's app does).
  if (!rows.length) {
    const name = session.metaName.trim() || session.email.split('@')[0] || 'Driver'
    await rest(session, 'POST', '/rpc/create_profile', { p_name: name }).catch(() => {})
    rows = (await rest(session, 'GET', query)) as ProfileRow[]
  }
  const row = rows[0] ?? {}
  return {
    displayName: row.display_name || session.metaName || session.email.split('@')[0],
    friendCode: row.friend_code ?? '',
    email: session.email,
    avatar: row.avatar ? `${URL}/storage/v1/object/public/avatars/${row.avatar}` : null,
    since: row.created_at ? Date.parse(row.created_at) : null,
  }
}

// --- the state, shared by every part that shows it ---------------------------------------------------

let state: OmsiHubState = { kind: 'loading' }
const listeners = new Set<(next: OmsiHubState) => void>()
let started = false

function set(next: OmsiHubState) {
  state = next
  listeners.forEach((listener) => listener(next))
}

async function refresh() {
  const session = await fresh()
  if (!session) return set({ kind: 'signedOut' })
  try {
    set({ kind: 'signedIn', profile: await readProfile(session) })
  } catch (error) {
    // The token refused (signed out elsewhere, the account gone): signed out here too.
    if (error instanceof Error && error.message === '401') {
      save(null)
      return set({ kind: 'signedOut' })
    }
    // Offline: still signed in, with what the session knows.
    set({
      kind: 'signedIn',
      profile: {
        displayName: session.metaName || session.email.split('@')[0],
        friendCode: '',
        email: session.email,
        avatar: null,
        since: null,
      },
    })
  }
}

/** The Omsi-Hub account as it is now; read once as the companion opens. */
export function useOmsiHub(): OmsiHubState {
  const [now, setNow] = useState(state)
  useEffect(() => {
    listeners.add(setNow)
    if (!started) {
      started = true
      void refresh()
    }
    return () => {
      listeners.delete(setNow)
    }
  }, [])
  return now
}

/** Signs in with an Omsi-Hub account's email and password; throws a `SignInError` code if it can't. */
export async function signIn(email: string, password: string): Promise<void> {
  try {
    save(await token('password', { email: email.trim(), password }))
  } catch (error) {
    throw error instanceof AuthError ? error.code : 'other'
  }
  await refresh()
}

/** `bytes` random bytes from the system's generator, as base64url without padding. */
function random(bytes: number): string {
  return base64url(crypto.getRandomValues(new Uint8Array(bytes)))
}

function base64url(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '')
}

/**
 * Signs in with Google or Discord, in the player's browser (the app's `omsihub_oauth`, Omsi-Hub's PKCE flow): the
 * verifier made and kept here, its challenge and the flow's state to the app, which opens the browser and brings
 * back the code; the code exchanged here with the verifier. Throws a `SignInError` code, or `null` when the player
 * cancelled.
 */
export async function signInWith(provider: Provider): Promise<void> {
  const verifier = random(32)
  const state = random(16)
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier))
  const challenge = base64url(new Uint8Array(digest))
  let code: string
  try {
    code = await call<string>('omsihub_oauth', { provider, challenge, state })
  } catch (error) {
    throw error === 'cancelled' ? null : error === 'timeout' ? 'timeout' : 'other'
  }
  try {
    save(await token('pkce', { auth_code: code, code_verifier: verifier }))
  } catch (error) {
    throw error instanceof AuthError && error.code === 'offline' ? 'offline' : 'other'
  }
  await refresh()
}

/** Stops waiting for the browser's sign-in. */
export function cancelSignInWith(): void {
  void call('omsihub_oauth_cancel').catch(() => {})
}

// --- the privacy statement, agreed to before the first sign-in ----------------------------------------------

/** The privacy statement's version (privacy.md): a new one is asked again. */
export const PRIVACY_VERSION = 1
const PRIVACY_STORE = 'eurohub.privacy'

export function privacyAccepted(): boolean {
  try {
    return localStorage.getItem(PRIVACY_STORE) === String(PRIVACY_VERSION)
  } catch {
    return false
  }
}

export function acceptPrivacy(): void {
  try {
    localStorage.setItem(PRIVACY_STORE, String(PRIVACY_VERSION))
  } catch {
    // Not kept: asked again next time.
  }
}

/** Signs out here (Omsi-Hub's own sign-ins elsewhere stay). */
export async function signOut(): Promise<void> {
  const session = load()
  save(null)
  set({ kind: 'signedOut' })
  if (session) {
    await fetch(`${URL}/auth/v1/logout?scope=local`, {
      method: 'POST',
      headers: { apikey: KEY, Authorization: `Bearer ${session.access}` },
    }).catch(() => {})
  }
}

/** Opens Omsi-Hub's sign-up or account page in the player's browser. */
export function openOmsiHub(page: 'signUp' | 'account'): void {
  if (IN_APP) void call('open_omsihub', { page }).catch(() => {})
  else window.open(page === 'signUp' ? 'https://omsihub.com/account.html#sign-up' : 'https://omsihub.com/account.html')
}

/** Two letters for a name, for a profile without a photo. */
export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean)
  const letters = parts.length > 1 ? parts[0][0] + parts[parts.length - 1][0] : (parts[0] ?? '?').slice(0, 2)
  return letters.toUpperCase()
}
