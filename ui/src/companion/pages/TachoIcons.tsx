import type { TachoActivity } from './tachoWords'

/** The four activities' pictograms, as tachographs draw them. */
export function ActivityIcon({ activity }: { activity: TachoActivity | null }) {
  return (
    <svg className="tacho-icon" viewBox="0 0 24 24" aria-hidden>
      {activity === 'driving' && (
        <>
          <circle cx="12" cy="12" r="8.5" />
          <circle cx="12" cy="12" r="2.2" />
          <path d="M3.8 10.5h6M14.2 10.5h6M12 14.2v6" />
        </>
      )}
      {activity === 'work' && (
        <>
          <path d="M5 19L15.5 8.5M19 19L8.5 8.5" />
          <path d="M13.5 6.5l4-2.5 2.5 2.5-2.5 4zM10.5 6.5l-4-2.5-2.5 2.5 2.5 4z" />
        </>
      )}
      {activity === 'available' && (
        <>
          <rect x="4" y="4" width="16" height="16" rx="1" />
          <path d="M4 20L20 4" />
        </>
      )}
      {activity === 'rest' && <path d="M3 18V7M3 14h18v4M21 14v-2.5a2.5 2.5 0 0 0-2.5-2.5H11v5M6.5 11.5h2" />}
      {activity === null && <path d="M6 12h12" />}
    </svg>
  )
}

/** A card in its slot (`in`), or the slot empty (crossed). */
export function CardIcon({ in: inside }: { in: boolean }) {
  return (
    <svg className="tacho-icon" viewBox="0 0 24 24" aria-hidden>
      <rect x="5" y="3" width="14" height="18" rx="1.5" />
      <path d="M8.5 7h4v4h-4z" />
      {!inside && <path d="M3 21L21 3" />}
    </svg>
  )
}

/** The clock's pictogram on the display. */
export function ClockIcon() {
  return (
    <svg className="tacho-icon" viewBox="0 0 24 24" aria-hidden>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 7.5V12l3 2" />
    </svg>
  )
}

/** A driver on an eject button: 1 or 2. */
export function DriverIcon({ n }: { n: 1 | 2 }) {
  return (
    <svg className="tacho-icon" viewBox="0 0 24 24" aria-hidden>
      <circle cx="12" cy="7" r="3.5" />
      <path d="M5.5 21v-3.5a6.5 6.5 0 0 1 13 0V21z" />
      <text x="12" y="19.5" textAnchor="middle" fontSize="7" fontWeight="800" stroke="none" fill="#1a1d22">
        {n}
      </text>
    </svg>
  )
}
