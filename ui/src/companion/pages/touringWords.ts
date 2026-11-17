import type { TouringActive, TouringStopKind, TripKind } from '../useHostStatus'
import { type CompanionWords, companionWords } from './words'

/** The Touringcar page's words: Dutch for Dutch, English for every other language. */
export interface TouringWords {
  title: string
  sub: string
  off: string
  noGame: string
  noMap: string
  company: string
  companyPlaceholder: string
  home: string
  homeDetail: string
  homePlaceholder: string
  depot: string
  balance: string
  reputation: string
  seats: string
  enabled: string
  enabledDetail: string
  navigate: string
  navigateDetail: string
  announcements: string
  announcementsDetail: string
  active: string
  noActive: string
  phases: Record<TouringActive['phase'], string>
  next: string
  late: string
  early: string
  onTime: string
  onBoard: string
  toCollect: string
  mood: string
  moodParts: { punctuality: string; comfort: string; breaks: string; bonus: string }
  meter: string
  skip: string
  cancel: string
  cancelConfirm: string
  stops: string
  stopKinds: Record<TouringStopKind, string>
  reached: string
  missed: string
  catalogue: string
  catalogueEmpty: string
  refresh: string
  start: string
  busy: string
  kinds: Record<TripKind, string>
  day: string
  daysN: string
  perSeat: string
  contract: string
  booked: string
  points: string
  pointsDetail: string
  record: string
  pointName: string
  forget: string
  history: string
  historyEmpty: string
  cancelled: string
  profit: string
  costs: string
  reset: string
  resetConfirm: string
  days: string[]
}

const NL: TouringWords = {
  title: 'Touringcar',
  sub: 'Je eigen touringcarbedrijf: dagtochten, meerdaagse reizen, schoolreizen en charters door heel Europa. Rijd ze met elk voertuig; een busmod is niet nodig.',
  off: 'Het touringcarbedrijf staat uit.',
  noGame: 'Start de game om reizen te plannen.',
  noMap: 'De kaart is nog niet klaar: zodra die er is, worden de reizen gepland.',
  company: 'Bedrijfsnaam',
  companyPlaceholder: 'Bijv. Reizen de Vries',
  home: 'Thuisbasis',
  homeDetail: 'Een stad op de kaart. Leeg: de eerste Nederlandse stad die de kaart heeft.',
  homePlaceholder: 'amsterdam',
  depot: 'Remise',
  balance: 'Saldo',
  reputation: 'Reputatie',
  seats: 'Zitplaatsen',
  enabled: 'Touringcarbedrijf',
  enabledDetail: 'Reizen plannen en rijden naast de vrachtwagenfuncties.',
  navigate: 'Navigatie naar de volgende stop',
  navigateDetail: 'Zet de route van de game op de volgende stop (niet tijdens een opdracht van de game).',
  announcements: 'Omroepen',
  announcementsDetail: 'Vraag om omroepen via de buspeakers: "Welkom aan boord…". Omroepen geven een bonus.',
  active: 'Rit',
  noActive: 'Geen rit bezig. Kies een reis uit de catalogus.',
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
  next: 'Volgende stop',
  late: 'min te laat',
  early: 'min te vroeg',
  onTime: 'op tijd',
  onBoard: 'Aan boord',
  toCollect: 'Nog op te halen',
  mood: 'Stemming',
  moodParts: { punctuality: 'Op tijd', comfort: 'Comfort', breaks: 'Pauzes', bonus: 'Omroepen' },
  meter: 'Gereden',
  skip: 'Volgende stop overslaan',
  cancel: 'Rit annuleren',
  cancelConfirm: 'Zeker? Passagiers krijgen hun geld terug en je reputatie daalt.',
  stops: 'Stops',
  stopKinds: {
    depot: 'Remise',
    pickup: 'Opstapplaats',
    coffee: 'Koffiepauze',
    meal: 'Maaltijdpauze',
    destination: 'Bestemming',
    hotel: 'Hotel',
    excursion: 'Excursie',
    dropoff: 'Uitstapplaats',
  },
  reached: 'bereikt',
  missed: 'overgeslagen',
  catalogue: 'Reiscatalogus',
  catalogueEmpty: 'Nog geen reizen.',
  refresh: 'Nieuwe reizen (zonder boekingen)',
  start: 'Rijden',
  busy: 'Eerst de huidige rit afmaken',
  kinds: { dayTrip: 'Dagtocht', tour: 'Rondreis', school: 'Schoolreis', charter: 'Charter' },
  day: 'dag',
  daysN: '{n} dagen',
  perSeat: 'p.p.',
  contract: 'contract',
  booked: 'geboekt',
  points: 'Eigen plekken',
  pointsDetail:
    'Sta stil op een plek in de game en leg hem hier vast als opstapplaats, koffiestop, hotel of remise. Nieuwe reizen gebruiken ze.',
  record: 'Hier vastleggen',
  pointName: 'Naam',
  forget: 'Vergeten',
  history: 'Geschiedenis',
  historyEmpty: 'Nog geen ritten gereden.',
  cancelled: 'geannuleerd',
  profit: 'Winst',
  costs: 'brandstof € {fuel} · tol € {tolls} · veer € {ferries} · hotel € {hotel} · loon € {wage}',
  reset: 'Bedrijf opnieuw beginnen',
  resetConfirm: 'Zeker? Saldo, reputatie en geschiedenis worden gewist.',
  days: ['ma', 'di', 'wo', 'do', 'vr', 'za', 'zo'],
}

const EN: TouringWords = {
  title: 'Coach company',
  sub: 'Your own coach-travel company: day trips, multi-day tours, school trips and charters across Europe. Drive them in any vehicle; a coach mod is optional.',
  off: 'The coach company is switched off.',
  noGame: 'Start the game to plan trips.',
  noMap: 'The map is not ready yet: trips are planned as soon as it is.',
  company: 'Company name',
  companyPlaceholder: 'E.g. De Vries Travel',
  home: 'Home base',
  homeDetail: 'A city on the map. Empty: the first Dutch city the map has.',
  homePlaceholder: 'amsterdam',
  depot: 'Depot',
  balance: 'Balance',
  reputation: 'Reputation',
  seats: 'Seats',
  enabled: 'Coach company',
  enabledDetail: 'Plan and drive trips alongside the truck features.',
  navigate: 'Navigate to the next stop',
  navigateDetail: "Sets the game's route to the next stop (not during a job of the game's).",
  announcements: 'Announcements',
  announcementsDetail: 'Asks for announcements over the coach speakers: "Welcome aboard…". Announcements give a bonus.',
  active: 'Trip',
  noActive: 'No trip under way. Pick one from the catalogue.',
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
  next: 'Next stop',
  late: 'min late',
  early: 'min early',
  onTime: 'on time',
  onBoard: 'On board',
  toCollect: 'Still to collect',
  mood: 'Mood',
  moodParts: { punctuality: 'On time', comfort: 'Comfort', breaks: 'Breaks', bonus: 'Announcements' },
  meter: 'Driven',
  skip: 'Skip the next stop',
  cancel: 'Cancel the trip',
  cancelConfirm: 'Sure? Passengers get their money back and your reputation drops.',
  stops: 'Stops',
  stopKinds: {
    depot: 'Depot',
    pickup: 'Pick-up point',
    coffee: 'Coffee stop',
    meal: 'Meal stop',
    destination: 'Destination',
    hotel: 'Hotel',
    excursion: 'Excursion',
    dropoff: 'Drop-off point',
  },
  reached: 'reached',
  missed: 'skipped',
  catalogue: 'Trip catalogue',
  catalogueEmpty: 'No trips yet.',
  refresh: 'New trips (without bookings)',
  start: 'Drive',
  busy: 'Finish the current trip first',
  kinds: { dayTrip: 'Day trip', tour: 'Tour', school: 'School trip', charter: 'Charter' },
  day: 'day',
  daysN: '{n} days',
  perSeat: 'per seat',
  contract: 'contract',
  booked: 'booked',
  points: 'Your own places',
  pointsDetail:
    'Stand still somewhere in the game and record it here as a pick-up point, coffee stop, hotel or depot. New trips use them.',
  record: 'Record here',
  pointName: 'Name',
  forget: 'Forget',
  history: 'History',
  historyEmpty: 'No trips driven yet.',
  cancelled: 'cancelled',
  profit: 'Profit',
  costs: 'fuel € {fuel} · tolls € {tolls} · ferry € {ferries} · hotel € {hotel} · wage € {wage}',
  reset: 'Start the company over',
  resetConfirm: 'Sure? Balance, reputation and history are wiped.',
  days: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
}

export function touringWords(cw: CompanionWords): TouringWords {
  return cw === companionWords('nl_nl') ? NL : EN
}

/** Game minutes as a day and a time ("wo 08:30"): game minute 0 is a Monday, 00:00. */
export function gameWhen(minutes: number, days: string[]): string {
  const whole = Math.round(minutes)
  const day = days[Math.floor(whole / 1440) % 7] ?? ''
  const inDay = ((whole % 1440) + 1440) % 1440
  return `${day} ${String(Math.floor(inDay / 60)).padStart(2, '0')}:${String(inDay % 60).padStart(2, '0')}`
}

/** Euros, whole. */
export function euro(amount: number): string {
  return `€ ${Math.round(amount).toLocaleString('nl-NL')}`
}

/** How far ahead or behind the plan. */
export function lateWords(t: TouringWords, late: number | null): string {
  if (late === null) return ''
  if (Math.abs(late) <= 2) return t.onTime
  return late > 0 ? `+${Math.round(late)} ${t.late}` : `${Math.round(-late)} ${t.early}`
}
