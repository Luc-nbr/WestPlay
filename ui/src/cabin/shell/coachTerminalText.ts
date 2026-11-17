import type { TouringStopKind, TouringTerminal } from '../../shared/host'

/** The terminal's words: Dutch for Dutch, English otherwise. */
export const WORDS = {
  nl: {
    kinds: { dayTrip: 'Dagtocht', tour: 'Rondreis', school: 'Schoolreis', charter: 'Charter' },
    stops: {
      depot: 'Remise',
      pickup: 'Opstapplaats',
      coffee: 'Koffiepauze',
      meal: 'Maaltijdpauze',
      destination: 'Bestemming',
      hotel: 'Hotel',
      excursion: 'Excursie',
      dropoff: 'Uitstapplaats',
    } satisfies Record<TouringStopKind, string>,
    phases: {
      toFirstPickup: 'Naar de eerste opstapplaats',
      boarding: 'Passagiers ophalen',
      outbound: 'Heenreis',
      stay: 'Verblijf',
      return: 'Terugreis',
      unloading: 'Passagiers afzetten',
      toDepot: 'Terug naar de remise',
      done: 'Klaar',
    },
    day: 'Dag',
    next: 'Volgende stop',
    late: 'min te laat',
    early: 'min te vroeg',
    onTime: 'op tijd',
    onBoard: 'aan boord',
    toCollect: 'op te halen',
    mood: 'Stemming',
    announce: 'Omroep',
    skip: 'Stop overslaan',
    prompts: {
      welcome: 'Welkom aan boord bij {company}! Vandaag rijden we naar {place}. Goede reis!',
      nextPickup: 'Volgende opstapplaats: {place}, verwachte aankomst {at}.',
      departure: 'Iedereen is aan boord. We vertrekken nu naar {place}.',
      coffee: 'We stoppen over 10 minuten voor een koffiepauze bij {place}.',
      meal: 'Over 10 minuten stoppen we voor de lunch bij {place}.',
      arrival: 'We zijn aangekomen in {place}. Veel plezier!',
      hotel: 'We zijn bij {place}. U kunt inchecken; morgen vertrekken we volgens het programma.',
      return: 'Over 10 minuten zijn we bij {place}. Vergeet uw spullen niet.',
      goodbye: 'Bedankt voor het reizen met {company}. Tot de volgende keer!',
    } as Record<string, string>,
  },
  en: {
    kinds: { dayTrip: 'Day trip', tour: 'Tour', school: 'School trip', charter: 'Charter' },
    stops: {
      depot: 'Depot',
      pickup: 'Pick-up point',
      coffee: 'Coffee stop',
      meal: 'Meal stop',
      destination: 'Destination',
      hotel: 'Hotel',
      excursion: 'Excursion',
      dropoff: 'Drop-off point',
    } satisfies Record<TouringStopKind, string>,
    phases: {
      toFirstPickup: 'To the first pick-up point',
      boarding: 'Picking up passengers',
      outbound: 'Outbound',
      stay: 'Stay',
      return: 'Return',
      unloading: 'Dropping passengers off',
      toDepot: 'Back to the depot',
      done: 'Done',
    },
    day: 'Day',
    next: 'Next stop',
    late: 'min late',
    early: 'min early',
    onTime: 'on time',
    onBoard: 'on board',
    toCollect: 'to collect',
    mood: 'Mood',
    announce: 'Announcement',
    skip: 'Skip stop',
    prompts: {
      welcome: 'Welcome aboard {company}! Today we are travelling to {place}. Enjoy the trip!',
      nextPickup: 'Next pick-up point: {place}, expected at {at}.',
      departure: 'Everyone is on board. We are now leaving for {place}.',
      coffee: 'In 10 minutes we will stop for a coffee break at {place}.',
      meal: 'In 10 minutes we will stop for lunch at {place}.',
      arrival: 'We have arrived in {place}. Enjoy!',
      hotel: 'We are at {place}. You can check in; tomorrow we leave as planned.',
      return: 'In 10 minutes we will be at {place}. Please take all your belongings.',
      goodbye: 'Thank you for travelling with {company}. See you next time!',
    } as Record<string, string>,
  },
}

export type Words = (typeof WORDS)['en']

/** Game minutes as the clock shows them (hh:mm). */
export function clock(minutes: number): string {
  const inDay = ((Math.round(minutes) % 1440) + 1440) % 1440
  return `${String(Math.floor(inDay / 60)).padStart(2, '0')}:${String(inDay % 60).padStart(2, '0')}`
}

/** What to say over the PA, in the cab's language. */
export function promptText(words: Words, prompt: NonNullable<TouringTerminal['prompt']>, company: string, shift = 0) {
  const line = words.prompts[prompt.kind] ?? ''
  return line
    .replace('{company}', company || 'Dashly')
    .replace('{place}', prompt.place)
    .replace('{at}', prompt.at === null ? '' : clock(prompt.at + shift))
}

/** How far ahead or behind the plan, as words. */
export function lateText(words: Words, late: number | null): string | null {
  if (late === null) return null
  if (Math.abs(late) <= 2) return words.onTime
  return late > 0 ? `+${Math.round(late)} ${words.late}` : `${Math.round(-late)} ${words.early}`
}
