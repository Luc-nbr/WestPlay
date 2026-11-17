import type { TachoLogEntry, TachoStatus } from '../useHostStatus'
import { hm, type TachoActivity, type TachoWords, when } from './tachoWords'

/** One line of the 24-hour printout. */
export type ReceiptLine =
  | { kind: 'rule' }
  | { kind: 'title'; text: string }
  | { kind: 'text'; text: string; right?: string; strong?: boolean }
  | { kind: 'activity'; activity: TachoActivity; from: string; length: string }
  | { kind: 'signature'; text: string }

/** Minutes as a tachograph prints them: 06h12. */
export function hours(minutes: number): string {
  const whole = Math.max(0, Math.floor(minutes))
  return `${String(Math.floor(whole / 60)).padStart(2, '0')}h${String(whole % 60).padStart(2, '0')}`
}

/** A card number from the driver's name, the same each time: NL and 14 digits, as a driver card's. */
export function cardNumber(name: string): string {
  let hash = 2166136261
  for (const char of name || 'driver') hash = Math.imul(hash ^ char.charCodeAt(0), 16777619) >>> 0
  const digits = String(hash).padStart(10, '0') + String((hash * 7919) % 10000).padStart(4, '0')
  return `NL ${digits.slice(0, 14)}`
}

/** The driver's name: the one typed, else the game profile's, else "Driver". */
export function driverName(driver: string, tacho: TachoStatus, t: TachoWords): string {
  return driver.trim() || tacho.profile?.name || t.receiptDriver
}

/** The tachograph's time now, as the display and the printout show it: `HH:MM`. */
export function clockTime(tacho: TachoStatus): string {
  if (tacho.clock === 'real') {
    const date = new Date()
    return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`
  }
  const minutes = Math.floor(tacho.gameTime ?? tacho.now) % 1440
  return `${String(Math.floor(minutes / 60)).padStart(2, '0')}:${String(minutes % 60).padStart(2, '0')}`
}

/** The day's printout (the last 24 hours), as a tachograph prints it on its thermal paper: when and what, the
 * driver and their card, the truck, every activity, the totals, the violations, and a line to sign. */
export function receipt(tacho: TachoStatus, driver: string, card: boolean, t: TachoWords): ReceiptLine[] {
  const s = tacho.summary
  const from = tacho.now - 1440
  // As a tachograph prints it: the date and time in UTC (the game's own clock with game time).
  const printed =
    tacho.clock === 'real'
      ? `${new Date().toLocaleString('nl-NL', { dateStyle: 'short', timeStyle: 'short', timeZone: 'UTC' })} (UTC)`
      : `${when(tacho.now, 'game', t.days)} (${t.gameClock})`
  const activities = tacho.log
    .filter((entry): entry is Extract<TachoLogEntry, { type: 'activity' }> => entry.type === 'activity')
    .filter((entry) => entry.end > from)
    .reverse()
  const violations = tacho.log
    .filter((entry): entry is Extract<TachoLogEntry, { type: 'violation' }> => entry.type === 'violation')
    .filter((entry) => entry.at > from)
  const name = driverName(driver, tacho, t)
  const company = tacho.profile?.company
  return [
    { kind: 'text', text: printed, strong: true },
    { kind: 'title', text: `24h ▼ ${t.receiptTitle}` },
    { kind: 'rule' },
    { kind: 'text', text: t.receiptDriver },
    ...(card
      ? ([
          { kind: 'text', text: name, strong: true },
          ...(company ? [{ kind: 'text', text: company } as ReceiptLine] : []),
          { kind: 'text', text: tacho.profile?.cardNumber ?? cardNumber(name) },
        ] as ReceiptLine[])
      : ([{ kind: 'text', text: t.receiptNoCard, strong: true }] as ReceiptLine[])),
    { kind: 'rule' },
    { kind: 'text', text: t.receiptVehicle },
    { kind: 'text', text: tacho.truck ?? t.receiptNoTruck, strong: true },
    { kind: 'text', text: t.receiptDistance, right: `${tacho.distance.toFixed(1)} km` },
    { kind: 'rule' },
    { kind: 'title', text: t.receiptActivities },
    ...activities.map((entry): ReceiptLine => ({
      kind: 'activity',
      activity: entry.activity,
      from: when(Math.max(entry.start, from), tacho.clock, t.days),
      length: hours(entry.end - Math.max(entry.start, from)),
    })),
    { kind: 'rule' },
    { kind: 'title', text: t.receiptTotals },
    { kind: 'text', text: `Σ ${t.day}`, right: hours(s.dayDriving) },
    { kind: 'text', text: `Σ ${t.week}`, right: hours(s.weekDriving) },
    { kind: 'text', text: `Σ ${t.fortnight}`, right: hours(s.fortnightDriving) },
    { kind: 'text', text: t.untilBreak, right: hm(s.nextLeft) },
    { kind: 'rule' },
    { kind: 'title', text: `! ${t.receiptViolations}` },
    ...(violations.length
      ? violations.map((entry): ReceiptLine => ({
          kind: 'text',
          // Events and violations marked "!", as on a tachograph's printout.
          text: `! ${when(entry.at, tacho.clock, t.days)} ${t.violations[entry.kind]}`,
          right: `+${hours(entry.over)}`,
        }))
      : ([{ kind: 'text', text: t.receiptNoViolations }] as ReceiptLine[])),
    { kind: 'rule' },
    { kind: 'signature', text: t.receiptSignature },
  ]
}
