import type { TachoAlert } from '../../shared/host'
import { useLive } from '../state/store'
import { useLanguage } from '../state/text'
import './tachoBanner.css'

/** The banner's words: Dutch for Dutch, English otherwise. */
const WORDS = {
  nl: {
    soon: {
      break: 'Pauze nodig over {n} min',
      day: 'Dagelijkse rijtijd op over {n} min',
      week: 'Wekelijkse rijtijd op over {n} min',
      fortnight: 'Rijtijd van twee weken op over {n} min',
      rest: 'Dagrust nodig over {n} min',
      card: 'Steek je bestuurderskaart in',
    },
    over: {
      break: 'Neem nu een pauze van 45 min',
      day: 'Dagelijkse rijtijd overschreden',
      week: 'Wekelijkse rijtijd overschreden',
      fortnight: 'Rijtijd van twee weken overschreden',
      rest: 'Dagrust te laat: stop en rust',
      card: 'Rijden zonder bestuurderskaart',
    },
    violation: 'Overtreding vastgelegd',
    title: 'Tachograaf',
  },
  en: {
    soon: {
      break: 'Break needed in {n} min',
      day: 'Daily driving time up in {n} min',
      week: 'Weekly driving time up in {n} min',
      fortnight: 'Two-week driving time up in {n} min',
      rest: 'Daily rest due in {n} min',
      card: 'Insert your driver card',
    },
    over: {
      break: 'Take a 45-minute break now',
      day: 'Daily driving time exceeded',
      week: 'Weekly driving time exceeded',
      fortnight: 'Two-week driving time exceeded',
      rest: 'Daily rest overdue: stop and rest',
      card: 'Driving without a driver card',
    },
    violation: 'Violation recorded',
    title: 'Tachograph',
  },
}

function text(words: (typeof WORDS)['en'], alert: TachoAlert): string {
  return words[alert.tone][alert.limit].replace('{n}', String(Math.max(1, alert.minutes)))
}

/** The tachograph's warning over the screen (the truck's and the iPad's): a break due, a limit passed, a rule just
 * broken. It pulses as it comes, and again with each warning given (15 and 5 minutes ahead). */
export function TachoBanner() {
  const tacho = useLive((state) => state.tacho)
  const words = useLanguage() === 'nl_nl' ? WORDS.nl : WORDS.en
  const alert = tacho?.alert ?? null
  const line = alert ? text(words, alert) : tacho?.violation ? words.violation : null
  if (!line) return null
  const tone = alert?.tone ?? 'over'
  return (
    // Mounted anew with each warning given: it pulses again.
    <div key={tacho?.ping ?? 0} className="tacho-banner" data-tone={tone} role="status">
      <svg viewBox="0 0 24 24" aria-hidden>
        <circle cx="12" cy="12" r="8.5" />
        <circle cx="12" cy="12" r="2.2" />
        <path d="M3.8 10.5h6M14.2 10.5h6M12 14.2v6" />
      </svg>
      <span>
        <small>{words.title}</small>
        <b>{line}</b>
      </span>
    </div>
  )
}
