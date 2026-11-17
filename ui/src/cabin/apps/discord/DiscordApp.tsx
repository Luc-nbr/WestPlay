import { useEffect } from 'react'
import { SafariApp } from '../safari/SafariApp'
import { load, newTab, selectTab, useSafari } from '../safari/safari'

/** Where Discord opens: its web app (sign in, servers, calls, streams). */
const DISCORD = 'https://discord.com/app'
/** Discord's desktop layout (its server list, channels, chat and members side by side) fitted to the cab's screen:
 * its pages drawn this much smaller, unless the player chose a zoom of their own for it (Safari's). */
const FIT = 0.75

/** Whether `url` is Discord's own (discord.com and its subdomains). */
function isDiscord(url: string): boolean {
  try {
    const host = new URL(url).hostname
    return host === 'discord.com' || host.endsWith('.discord.com')
  } catch {
    return false
  }
}

/**
 * Discord in the cab: its web app in Safari's engine (its own tab, kept with Safari's), shown at once. Calls hear
 * the player's microphone (the host lets discord.com have it, safari.rs) and play through the truck's speakers;
 * a friend's stream plays in the page as any video does.
 */
export function DiscordApp() {
  useEffect(() => {
    const { zooms } = useSafari.getState()
    if (zooms['discord.com'] === undefined) useSafari.setState({ zooms: { ...zooms, 'discord.com': FIT } })
    const found = useSafari.getState().tabs.find((tab) => isDiscord(tab.url))
    if (found) {
      selectTab(found.id)
    } else {
      newTab()
      load(DISCORD)
    }
  }, [])
  return <SafariApp />
}
