// Euro-Hub: Discord's pages in Safari (safari.rs, before safari.js, which takes `window.chrome.webview` from the
// page): Discord in Euro-Hub's look, made for a car's screen, and its call told to the cab (whether there is one, and
// whether the microphone is muted and the sound deafened) for the cab's own call bar. What it looks like only: the
// account, the messages and the call are Discord's own, untouched.
;(() => {
  if (window !== window.top || !/(^|\.)discord\.com$/.test(location.hostname)) return
  const host = window.chrome?.webview
  if (!host) return

  // ---------- Euro-Hub's look ----------
  const SKIN = `
    :root, .theme-dark, .theme-darker, .theme-midnight, .theme-light {
      --background-primary: #0e1a33 !important;
      --background-secondary: #0a1530 !important;
      --background-secondary-alt: #08122a !important;
      --background-tertiary: #060e22 !important;
      --background-floating: #0b1735 !important;
      --background-base-lowest: #060e22 !important;
      --background-base-lower: #0a1530 !important;
      --background-base-low: #0e1a33 !important;
      --background-surface-high: #13234a !important;
      --background-surface-higher: #172a57 !important;
      --background-surface-highest: #1b3163 !important;
      --chat-background-default: #0e1a33 !important;
      --modal-background: #0b1735 !important;
      --brand-500: #3b6bff !important;
      --brand-560: #3159d6 !important;
      --brand-experiment: #3b6bff !important;
      --button-filled-brand-background: #3b6bff !important;
      --control-brand-foreground: #3b6bff !important;
    }
    /* Not for a car: the member list, Nitro, the Shop, Quests, and getting the desktop app. */
    [class*="membersWrap"], a[href="/store"], a[href="/shop"], a[href^="/quest"],
    [class*="listItem"]:has([class*="download"]) { display: none !important; }
    /* Big enough to press on a screen, the text read at a glance. */
    [class*="channel_"] [class*="link_"], [class*="interactive_"] { min-height: 44px !important; }
    [class*="channel_"] { margin-block: 2px !important; }
    [class*="name_"], [class*="channelName"] { font-size: 17px !important; }
    [class*="messageContent"] { font-size: 18px !important; line-height: 1.45 !important; }
    /* Rounded, as Euro-Hub's cards; no scrollbars (the screen is dragged). */
    [class*="chatContent"] { border-top-left-radius: 18px !important; }
    [class*="guilds_"] [class*="listItem"] [class*="wrapper_"] { border-radius: 16px !important; }
    ::-webkit-scrollbar { width: 0 !important; height: 0 !important; }
  `
  const style = document.createElement('style')
  style.textContent = SKIN
  const place = () => {
    if (!style.isConnected) (document.head || document.documentElement).append(style)
  }
  place()
  document.addEventListener('DOMContentLoaded', place)
  new MutationObserver(place).observe(document.documentElement, { childList: true })

  // ---------- The call, for the cab's call bar ----------
  const LEAVE = /disconnect|verbinding verbreken|trennen|déconnecter|desconectar|scollegati|rozłącz|отключ|bağlantıyı kes/i
  const panel = () => document.querySelector('[class*="rtcConnectionStatus"]')
  const call = () => {
    const switches = [...document.querySelectorAll('section[class*="panels"] [role="switch"]')]
    return {
      inCall: !!panel(),
      muted: switches[0]?.getAttribute('aria-checked') === 'true',
      deafened: switches[1]?.getAttribute('aria-checked') === 'true',
    }
  }
  let said = ''
  setInterval(() => {
    const now = JSON.stringify(call())
    if (now === said) return
    said = now
    host.postMessage({ discord: JSON.parse(now) })
  }, 400)
  // The cab's Hang Up: Discord's own Disconnect pressed.
  host.addEventListener('message', ({ data }) => {
    if (data?.action !== 'discordLeave') return
    const box = panel()?.closest('[class*="connection"]') ?? panel()?.parentElement?.parentElement
    const buttons = [...(box?.querySelectorAll('button, [role="button"]') ?? [])]
    const leave = buttons.find((button) => LEAVE.test(button.getAttribute('aria-label') ?? '')) ?? buttons.at(-1)
    leave?.click()
  })
})()
