/**
 * The cabin page on an iPad (`westplay_host::remote`): served by WestPlay on the local network, it talks to WestPlay
 * over a WebSocket as the truck's page does through WebView2. `installIpad` puts a stand-in for WebView2's
 * `chrome.webview` in place before the page's own code runs, so the page works as it does in the cab; what's about
 * the truck's screen itself (its layout, cursor, keys) stays the truck page's and isn't passed on.
 *
 * The iPad must be paired once: the code the companion shows (Truck Screen › iPad), after which WestPlay knows it.
 */

declare global {
  interface Window {
    /** Set by WestPlay's server on the page it serves the iPad. */
    westplayIpad?: boolean
  }
}

export const IS_IPAD = typeof window !== 'undefined' && window.westplayIpad === true

/** Where WestPlay serves the player's own files to the page: WebView2's virtual hosts in the cab, the server's paths
 * on the iPad (`maps`, `customize`, `music-covers`, `safari-tabs`). No `/` at the end. */
export function hostFiles(name: 'maps' | 'customize' | 'music-covers' | 'safari-tabs'): string {
  return IS_IPAD ? `${location.origin}/${name}` : `https://westplay.${name}`
}

/** What WestPlay says to the truck's page only. */
const TRUCK_ONLY = new Set(['screen', 'cursor', 'westplay', 'keybind', 'typed', 'typingState', 'headUnit'])

type Listener = (event: { data: unknown }) => void

/** Binary messages (a player's picture), for whoever shows them. */
export const binaryListeners = new Set<(data: ArrayBuffer) => void>()

let socket: WebSocket | null = null

/** What the truck's screen shows while the iPad is the head unit: the Dashboard, Maps, or the truck's own screen. */
export type TruckView = 'dashboard' | 'map' | 'game'
const TRUCK_VIEW_KEY = 'westplay.ipad.truckView'

export function truckView(): TruckView {
  try {
    const view = localStorage.getItem(TRUCK_VIEW_KEY)
    return view === 'map' || view === 'game' ? view : 'dashboard'
  } catch {
    return 'dashboard'
  }
}

export function setTruckView(view: TruckView): void {
  try {
    localStorage.setItem(TRUCK_VIEW_KEY, view)
  } catch {
    // Not kept: chosen for now.
  }
  sendToWestplay({ type: 'truckView', view })
}

/** Sends a message to WestPlay from the iPad (nothing while it's not connected). */
export function sendToWestplay(message: unknown): void {
  if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify(message))
}

export function installIpad(): void {
  if (!IS_IPAD) return
  const style = document.createElement('style')
  // A touchscreen: the page's own drags scroll it (as with the cab's mouse), nothing selected or zoomed by a press.
  style.textContent =
    'html,body{touch-action:none;-webkit-user-select:none;user-select:none;-webkit-touch-callout:none;overscroll-behavior:none}' +
    // The players' pictures show under the page, where it leaves room for them (as on the truck's frame).
    'html{background:#000}body{background:transparent}#root{position:relative;z-index:1}' +
    '.ipad-picture{position:fixed;z-index:0;object-fit:fill;pointer-events:none;display:none}' +
    '*{-webkit-tap-highlight-color:transparent}' +
    // The cab follows every drag itself (Home's pages, its lists: dragScroll.ts): Safari taking one over as its own
    // pan cancels it halfway (a swipe that went nowhere).
    '*{touch-action:none!important}'
  document.head.append(style)
  const pictures = installPictures()
  // Safari shows a press (`:active`) on a touchscreen only on pages listening for touches.
  document.addEventListener('touchstart', () => {}, { passive: true })
  reportFrames()
  // Gone to the background, or back (iPadOS stops a page there): told to Euro-Hub's log.
  document.addEventListener('visibilitychange', () => {
    ;(window as unknown as { westplayTell?: (text: string) => void }).westplayTell?.(`page ${document.visibilityState}`)
  })

  const listeners = new Set<Listener>()
  const queued: string[] = []
  // Connected before: Euro-Hub may have started again since, a newer build of it (a new app, a fix). The page asks
  // for its own address again, and loads it anew if its scripts aren't these any more.
  let connectedBefore = false
  const ourScripts = () =>
    [...document.querySelectorAll<HTMLScriptElement>('script[type="module"][src]')].map(
      (script) => new URL(script.src, location.href).pathname,
    )
  const reloadIfNewer = async () => {
    try {
      const page = await (await fetch('/', { cache: 'no-store' })).text()
      const theirs = [...page.matchAll(/<script[^>]+type="module"[^>]+src="([^"]+)"/g)].map(
        (match) => new URL(match[1], location.href).pathname,
      )
      const ours = ourScripts()
      if (theirs.length > 0 && theirs.some((src) => !ours.includes(src))) location.reload()
    } catch {
      // Not reachable this moment: the next connection asks again.
    }
  }
  const connect = () => {
    const ws = new WebSocket(`${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/ws`)
    ws.binaryType = 'arraybuffer'
    ws.onopen = () => {
      socket = ws
      if (connectedBefore) void reloadIfNewer()
      connectedBefore = true
      // WestPlay sends its whole state again (it went on without this page).
      if (listeners.size > 0) ws.send(JSON.stringify({ type: 'ready' }))
      for (const message of queued.splice(0)) ws.send(message)
      ws.send(JSON.stringify({ type: 'truckView', view: truckView() }))
    }
    ws.onmessage = (event: MessageEvent) => {
      if (typeof event.data !== 'string') {
        for (const listener of binaryListeners) listener(event.data as ArrayBuffer)
        return
      }
      let data: { type?: string }
      try {
        data = JSON.parse(event.data) as { type?: string }
      } catch {
        return
      }
      if (data.type && TRUCK_ONLY.has(data.type)) return
      for (const listener of listeners) listener({ data })
    }
    ws.onclose = () => {
      if (socket === ws) socket = null
      setTimeout(() => void start(), 1000)
    }
  }
  const start = async () => {
    try {
      const paired = await fetch('/paired', { cache: 'no-store' })
      if (paired.ok) connect()
      else showPairing()
    } catch {
      setTimeout(() => void start(), 2000)
    }
  }

  ;(window as unknown as { chrome: unknown }).chrome = {
    webview: {
      addEventListener: (_: 'message', listener: Listener) => listeners.add(listener),
      removeEventListener: (_: 'message', listener: Listener) => listeners.delete(listener),
      postMessage: (message: unknown) => {
        pictures.placed(message)
        const text = JSON.stringify(message)
        if (socket?.readyState === WebSocket.OPEN) socket.send(text)
        else if ((message as { type?: string }).type !== 'ready') queued.push(text)
      },
    },
  }
  void start()
}

interface Rect {
  x: number
  y: number
  width: number
  height: number
}

/** Each player's message tag (`westplay_host::mirror`) by the page's message type. */
const PICTURE_TAGS: Record<string, number> = { youtube: 1, youtubeMusic: 2, safari: 3 }

/**
 * The players' pictures from WestPlay (their browsers run on the PC), each shown where the page placed that player
 * (its `show` commands); a touch on a Safari page passed on to it.
 */
function installPictures(): { placed: (message: unknown) => void } {
  const shown = new Map<number, { rect: Rect; covers: Rect[]; img: HTMLImageElement }>()
  const image = (tag: number) => {
    let entry = shown.get(tag)
    if (!entry) {
      const img = document.createElement('img')
      img.className = 'ipad-picture'
      document.body.prepend(img)
      entry = { rect: { x: 0, y: 0, width: 0, height: 0 }, covers: [], img }
      shown.set(tag, entry)
    }
    return entry
  }
  binaryListeners.add((data) => {
    const tag = new Uint8Array(data, 0, 1)[0]
    const entry = shown.get(tag)
    if (!entry || entry.img.style.display !== 'block') return
    const url = URL.createObjectURL(new Blob([data.slice(1)], { type: 'image/jpeg' }))
    const before = entry.img.src
    entry.img.onload = () => {
      if (before.startsWith('blob:')) URL.revokeObjectURL(before)
    }
    entry.img.src = url
  })
  const within = (rect: Rect, x: number, y: number) =>
    x >= rect.x && y >= rect.y && x < rect.x + rect.width && y < rect.y + rect.height
  let touching: number | null = null
  const touch = (kind: 'down' | 'move' | 'up', event: PointerEvent) => {
    const web = shown.get(PICTURE_TAGS.safari)
    if (kind === 'down') {
      const on =
        web &&
        web.img.style.display === 'block' &&
        within(web.rect, event.clientX, event.clientY) &&
        !web.covers.some((cover) => within(cover, event.clientX, event.clientY))
      if (!on) return
      touching = event.pointerId
    } else if (touching !== event.pointerId) {
      return
    }
    if (kind === 'up') touching = null
    // In the screen's pixels, as Safari places its page (WebPage.tsx): two to the iPad's point.
    const scale = window.devicePixelRatio || 1
    sendToWestplay({ type: 'ipadWeb', kind, x: event.clientX * scale, y: event.clientY * scale })
  }
  window.addEventListener('pointerdown', (event) => touch('down', event), true)
  window.addEventListener('pointermove', (event) => touch('move', event), true)
  window.addEventListener('pointerup', (event) => touch('up', event), true)
  window.addEventListener('pointercancel', (event) => touch('up', event), true)
  return {
    placed: (message) => {
      const { type, command } = (message ?? {}) as { type?: string; command?: Record<string, unknown> }
      const tag = type ? PICTURE_TAGS[type] : undefined
      if (!tag || !command) return
      const entry = image(tag)
      if (command.action === 'hide') {
        entry.img.style.display = 'none'
      } else if (command.action === 'show') {
        // Safari places its page in the screen's pixels (WebPage.tsx): the iPad's points are each two of them.
        const scale = tag === PICTURE_TAGS.safari ? window.devicePixelRatio || 1 : 1
        const points = (r: Rect): Rect => ({
          x: (Number(r.x) || 0) / scale,
          y: (Number(r.y) || 0) / scale,
          width: (Number(r.width) || 0) / scale,
          height: (Number(r.height) || 0) / scale,
        })
        const rect = points(command as unknown as Rect)
        entry.rect = rect
        entry.covers = Array.isArray(command.covers) ? (command.covers as Rect[]).map(points) : []
        Object.assign(entry.img.style, {
          display: 'block',
          left: `${rect.x}px`,
          top: `${rect.y}px`,
          width: `${rect.width}px`,
          height: `${rect.height}px`,
        })
      }
    },
  }
}

/** The page's frames a second, told to WestPlay every 2 s (the companion shows them). */
function reportFrames(): void {
  let frames = 0
  let slow = 0
  let last = performance.now()
  let since = last
  const frame = (now: number) => {
    frames += 1
    if (now - last > 25) slow += 1
    last = now
    if (now - since >= 2000) {
      sendToWestplay({ type: 'ipadFps', fps: Math.round((frames * 1000) / (now - since)), slow })
      frames = 0
      slow = 0
      since = now
    }
    requestAnimationFrame(frame)
  }
  requestAnimationFrame(frame)
}

/** Asks for the companion's code, once; the page starts again paired. */
function showPairing(): void {
  const dutch = navigator.language.toLowerCase().startsWith('nl')
  const say = dutch
    ? {
        title: 'Koppel deze iPad met Euro-Hub',
        text: 'Voer de code in die Euro-Hub op je pc toont (Vrachtwagenscherm › iPad).',
        button: 'Koppelen',
        wrong: 'Die code klopt niet.',
        tip: 'Tip: zet Instellingen › Scherm en helderheid › Automatisch slot op Nooit, en voeg Euro-Hub toe aan je beginscherm (Deel › Zet op beginscherm).',
      }
    : {
        title: 'Pair this iPad with Euro-Hub',
        text: 'Enter the code Euro-Hub shows on your PC (Truck Screen › iPad).',
        button: 'Pair',
        wrong: 'That code isn’t right.',
        tip: 'Tip: set Settings › Display & Brightness › Auto-Lock to Never, and add Euro-Hub to your Home Screen (Share › Add to Home Screen).',
      }
  const box = document.createElement('div')
  box.style.cssText =
    'position:fixed;inset:0;z-index:99999;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:24px;background:#000;color:#fff;font:400 28px/1.3 -apple-system,system-ui,sans-serif;text-align:center;padding:48px;touch-action:auto;-webkit-user-select:text'
  box.innerHTML = `<div style="font-weight:700;font-size:44px"></div><div></div>
    <input inputmode="numeric" maxlength="6" autocomplete="one-time-code" style="font:600 56px monospace;letter-spacing:12px;width:330px;text-align:center;border-radius:16px;border:0;padding:16px;color:#000" />
    <button style="font:600 32px -apple-system,system-ui;padding:16px 56px;border-radius:999px;border:0;background:#0a84ff;color:#fff"></button>
    <div style="color:#ff453a;min-height:36px"></div><div style="color:#8e8e93;font-size:22px;max-width:760px"></div>`
  const [title, text, input, button, wrong, tip] = [...box.children] as HTMLElement[]
  title.textContent = say.title
  text.textContent = say.text
  button.textContent = say.button
  tip.textContent = say.tip
  const pair = async () => {
    const code = (input as HTMLInputElement).value.trim()
    const answer = await fetch(`/pair?code=${encodeURIComponent(code)}`, { cache: 'no-store' }).catch(() => null)
    if (answer?.ok) location.reload()
    else wrong.textContent = say.wrong
  }
  button.addEventListener('click', () => void pair())
  input.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') void pair()
  })
  document.body.append(box)
}
