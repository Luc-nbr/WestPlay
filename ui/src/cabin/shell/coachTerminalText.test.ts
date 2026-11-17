import { describe, expect, it } from 'vitest'
import { clock, lateText, promptText, WORDS } from './coachTerminalText'

describe('the coach terminal', () => {
  it('shows game minutes as the clock', () => {
    expect(clock(0)).toBe('00:00')
    expect(clock(3 * 1440 + 8 * 60 + 5)).toBe('08:05')
  })

  it('says how far ahead or behind', () => {
    expect(lateText(WORDS.nl, null)).toBeNull()
    expect(lateText(WORDS.nl, 1.4)).toBe('op tijd')
    expect(lateText(WORDS.nl, 7.2)).toBe('+7 min te laat')
    expect(lateText(WORDS.en, -4)).toBe('4 min early')
  })

  it('words the announcement asked for', () => {
    const welcome = { kind: 'welcome', place: 'Paris', at: null }
    expect(promptText(WORDS.nl, welcome, 'Reizen de Vries')).toBe(
      'Welkom aan boord bij Reizen de Vries! Vandaag rijden we naar Paris. Goede reis!',
    )
    const next = { kind: 'nextPickup', place: 'Utrecht', at: 9 * 60 + 30 }
    expect(promptText(WORDS.en, next, '')).toBe('Next pick-up point: Utrecht, expected at 09:30.')
  })
})
