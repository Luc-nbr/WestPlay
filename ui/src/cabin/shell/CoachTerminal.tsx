import { useState } from 'react'
import { postToHost } from '../../shared/host'
import { clock, lateText, promptText, WORDS } from './coachTerminalText'
import { useLive } from '../state/store'
import { useLanguage } from '../state/text'
import './coachTerminal.css'

/** The touring coach's driver's terminal over the screen (the truck's and the iPad's), while a trip is driven:
 * the trip, the next stop with its time and how far ahead or behind, the passengers, their mood, and the
 * announcement to make. Tapped, it folds to a pill and back. */
export function CoachTerminal() {
  const touring = useLive((state) => state.touring)
  const shift = useLive((state) => state.clockShift)
  const words = useLanguage() === 'nl_nl' ? WORDS.nl : WORDS.en
  const [folded, setFolded] = useState(false)
  const trip = touring?.trip
  if (!trip) return null
  const next = trip.next
  const late = next ? lateText(words, next.late) : null
  const tone = next?.late != null && next.late > 5 ? 'late' : 'ok'
  const mood = trip.mood >= 75 ? 'good' : trip.mood >= 50 ? 'fair' : 'poor'
  if (folded) {
    return (
      <button type="button" className="coach-terminal coach-folded" onClick={() => setFolded(false)}>
        <b>{next ? words.stops[next.kind] : words.phases[trip.phase]}</b>
        {next && <span>{clock(next.at + shift)}</span>}
        {late && <span data-tone={tone}>{late}</span>}
      </button>
    )
  }
  return (
    <section className="coach-terminal" aria-label={touring.company || words.kinds[trip.kind]}>
      <header onClick={() => setFolded(true)}>
        <small>{touring.company || 'Dashly'}</small>
        <b>
          {words.kinds[trip.kind]} · {trip.destination}
        </b>
        <span>
          {words.day} {trip.day}/{trip.days} · {words.phases[trip.phase]}
        </span>
      </header>
      {next && (
        <div className="coach-next" data-tone={tone}>
          <small>
            {words.next}: {words.stops[next.kind]}
          </small>
          <b>{next.name}</b>
          <span>
            {clock(next.at + shift)}
            {next.distance !== null && ` · ${Math.round(next.distance)} km`}
            {late && <em> · {late}</em>}
          </span>
        </div>
      )}
      <div className="coach-people">
        <span>
          <b>{trip.onBoard}</b> {words.onBoard}
        </span>
        {trip.toCollect > 0 && (
          <span>
            <b>{trip.toCollect}</b> {words.toCollect}
          </span>
        )}
        <span className="coach-mood" data-mood={mood}>
          {words.mood} <b>{trip.mood}%</b>
        </span>
      </div>
      {trip.prompt && (
        <div className="coach-prompt">
          <small>{words.announce}</small>
          <p>{promptText(words, trip.prompt, touring.company, shift)}</p>
        </div>
      )}
      {next && (
        <button type="button" className="coach-skip" onClick={() => postToHost({ type: 'touring', action: 'skip' })}>
          {words.skip}
        </button>
      )}
    </section>
  )
}
