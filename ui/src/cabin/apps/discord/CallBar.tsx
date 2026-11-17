import { discordCall, useSafari } from '../safari/safari'
import './discord.css'

/**
 * Discord's call, as a car's own call screen: under Discord's page while a call is on, three big buttons (mute the
 * microphone, deafen, hang up), as CarPlay's call controls. They press Discord's own (its keys, its Disconnect:
 * discord.js); the wheel's buttons can too (the companion's Controls).
 */
export function CallBar() {
  const call = useSafari((state) => state.discordCall)
  if (!call?.inCall) return null
  return (
    <div className="dc-call" role="toolbar" aria-label="Discord">
      <span className="dc-call-state">
        <i />
        {call.deafened ? 'Geluid uit' : call.muted ? 'Gedempt' : 'In gesprek'}
      </span>
      <button
        type="button"
        className="dc-round"
        data-on={call.muted || undefined}
        aria-label="Dempen"
        onClick={() => discordCall('mute')}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <rect x="9" y="3" width="6" height="11" rx="3" />
          <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21" />
          {call.muted && <path className="dc-slash" d="M4 4l16 16" />}
        </svg>
      </button>
      <button
        type="button"
        className="dc-round"
        data-on={call.deafened || undefined}
        aria-label="Geluid uit"
        onClick={() => discordCall('deafen')}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 15v-3a8 8 0 0 1 16 0v3" />
          <rect x="3.5" y="14" width="4" height="6" rx="1.5" />
          <rect x="16.5" y="14" width="4" height="6" rx="1.5" />
          {call.deafened && <path className="dc-slash" d="M4 4l16 16" />}
        </svg>
      </button>
      <button type="button" className="dc-round dc-leave" aria-label="Ophangen" onClick={() => discordCall('leave')}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M3 13.5c5-4.5 13-4.5 18 0l-2.2 2.4-3.3-1.6v-2.4a11 11 0 0 0-7 0v2.4l-3.3 1.6z" />
        </svg>
      </button>
    </div>
  )
}
