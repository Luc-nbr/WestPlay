import { type HostMessage, isInHost, postToHost } from '../../shared/host'

/**
 * What the page keeps between runs, kept by WestPlay (`westplay_host::page_state`) so every page it shows (the
 * truck's screen, the iPad) has the same: the settings, recent destinations, Safari's tabs, the players' kept state.
 * The page's own `localStorage` stays a copy, so the page starts with what it had at once (and works outside
 * WestPlay); every change goes to WestPlay, and WestPlay's changes come back into the copy and the stores using it.
 *
 * What's about one screen only (its brightness, the app it shows, the map's view) isn't shared: keys not listed
 * here stay the page's own, and a shared key's `localFields` stay with each screen.
 */
export const SHARED_KEYS: readonly string[] = [
  'westplay.cabin',
  'westplay.recents',
  'westplay.safari',
  'westplay.youtube',
  'westplay.youtubeMusic',
  'westplay.spotify.kept',
  'westplay.spotify.volume',
  'westplay.youtubeMusic.volume',
]

const locals = new Map<string, string[]>()
const rehydrators = new Map<string, () => unknown>()

function local(): Storage | null {
  try {
    return globalThis.localStorage ?? null
  } catch {
    return null
  }
}

function read(key: string): string | null {
  try {
    return local()?.getItem(key) ?? null
  } catch {
    return null
  }
}

function write(key: string, value: string | null): void {
  try {
    if (value === null) local()?.removeItem(key)
    else local()?.setItem(key, value)
  } catch {
    // Not kept here: WestPlay still has it.
  }
}

/** A zustand persisted value (`{state, version}`) with `fields` of its state taken out, or as it is if it's not one. */
function withoutLocal(key: string, value: string | null): string | null {
  const fields = locals.get(key)
  if (!fields || value === null) return value
  try {
    const parsed = JSON.parse(value) as { state?: Record<string, unknown> }
    if (!parsed || typeof parsed.state !== 'object' || parsed.state === null) return value
    const state = { ...parsed.state }
    for (const field of fields) delete state[field]
    return JSON.stringify({ ...parsed, state })
  } catch {
    return value
  }
}

/** WestPlay's `value` with this screen's own fields kept from `mine`. */
function withLocal(key: string, value: string | null, mine: string | null): string | null {
  const fields = locals.get(key)
  if (!fields || value === null || mine === null) return value
  try {
    const parsed = JSON.parse(value) as { state?: Record<string, unknown> }
    const own = (JSON.parse(mine) as { state?: Record<string, unknown> })?.state
    if (!parsed || typeof parsed.state !== 'object' || !own) return value
    const state = { ...parsed.state }
    for (const field of fields) if (field in own) state[field] = own[field]
    return JSON.stringify({ ...parsed, state })
  } catch {
    return value
  }
}

function shared(key: string): boolean {
  return SHARED_KEYS.includes(key)
}

/** The storage the shared stores keep in: `localStorage`'s shape, its changes to shared keys told to WestPlay. */
export const sharedStorage: Pick<Storage, 'getItem' | 'setItem' | 'removeItem'> = {
  getItem: read,
  setItem: (key, value) => {
    const before = read(key)
    write(key, value)
    if (shared(key) && isInHost() && withoutLocal(key, before) !== withoutLocal(key, value)) {
      postToHost({ type: 'pageState', key, value: withoutLocal(key, value) })
    }
  },
  removeItem: (key) => {
    const before = read(key)
    write(key, null)
    if (shared(key) && isInHost() && before !== null) postToHost({ type: 'pageState', key, value: null })
  },
}

/** These fields of a shared store's state are this screen's own: never sent, kept when WestPlay's value comes. */
export function localFields(key: string, fields: string[]): void {
  locals.set(key, fields)
}

/** The store keeping in `key` loads again when WestPlay's value for it changes. */
export function onSharedChange(key: string, rehydrate: () => unknown): void {
  rehydrators.set(key, rehydrate)
}

function take(key: string, value: string | null): void {
  if (!shared(key)) return
  const mine = read(key)
  const next = withLocal(key, value, mine)
  if (next === mine) return
  write(key, next)
  void rehydrators.get(key)?.()
}

/** WestPlay's kept state (all of it as the page connects, or one part as it changes). */
export function receivePageState(message: Extract<HostMessage, { type: 'pageState' | 'pageStateChanged' }>): void {
  if (message.type === 'pageStateChanged') {
    take(message.key, message.value)
    return
  }
  if (message.values === null) {
    // WestPlay keeps nothing yet (the first run with it keeping this): it takes what this page kept.
    const values: Record<string, string> = {}
    for (const key of SHARED_KEYS) {
      const value = withoutLocal(key, read(key))
      if (value !== null) values[key] = value
    }
    if (isInHost()) postToHost({ type: 'pageStateImport', values })
    return
  }
  for (const key of SHARED_KEYS) take(key, message.values[key] ?? null)
}
