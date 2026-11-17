import type { TachoStatus } from '../useHostStatus'
import { type CompanionWords, companionWords } from './words'

/** The Tachograph page's words: Dutch for Dutch, English for every other language. */
export interface TachoWords {
  title: string
  sub: string
  off: string
  activity: Record<TachoActivity, string>
  none: string
  since: string
  untilBreak: string
  breakTaking: string
  breakDone: string
  day: string
  week: string
  fortnight: string
  restDue: string
  restLate: string
  restTaking: string
  extended: string
  reduced: string
  gameClock: string
  realClock: string
  limits: Record<TachoLimit, string>
  log: string
  logEmpty: string
  violations: Record<TachoViolation, string>
  over: string
  settings: string
  enabled: string
  enabledDetail: string
  clockGame: string
  clockGameDetail: string
  clockReal: string
  clockRealDetail: string
  clockNote: string
  cabWarnings: string
  cabWarningsDetail: string
  clear: string
  clearConfirm: string
  days: string[]
  profileTitle: string
  profileDriver: string
  profileCompany: string
  profileCard: string
  profileDistance: string
  profileDeliveries: string
  profileExperience: string
  profileMoney: string
  profileWaiting: string
  driverAuto: string
  lcdNoCard: string
  lcdLeft: string
  lcdBreak: string
  lcdDay: string
  lcdWeek: string
  lcdFortnight: string
  lcdRestIn: string
  lcdViolations: string
  lcdNone: string
  lcdPrint: string
  lcdPrintOk: string
  lcdPrinting: string
  cardTitle: string
  cardHint: string
  cardIn: string
  buttonsHint: string
  receiptTitle: string
  receiptDriver: string
  receiptVehicle: string
  receiptActivities: string
  receiptTotals: string
  receiptDistance: string
  receiptViolations: string
  receiptNoViolations: string
  receiptSignature: string
  receiptPrint: string
  receiptTear: string
  receiptNoCard: string
  receiptNoTruck: string
  driverName: string
  driverPlaceholder: string
  paper: string
  paperDetail: string
}

export type TachoActivity = 'driving' | 'work' | 'available' | 'rest'
export type TachoLimit = 'break' | 'day' | 'week' | 'fortnight'
export type TachoViolation =
  | 'continuousDriving'
  | 'dailyDriving'
  | 'dailyRestLate'
  | 'reducedRests'
  | 'weeklyDriving'
  | 'fortnightDriving'
  | 'overspeed'

const EN: TachoWords = {
  title: 'Tachograph',
  sub: 'Your driving and rest times, recorded live from the game and checked against the EU rules.',
  off: 'The tachograph is off.',
  activity: { driving: 'Driving', work: 'Other work', available: 'Available', rest: 'Break / rest' },
  none: 'Waiting for the game',
  since: 'for',
  untilBreak: 'Driving left until break',
  breakTaking: 'Break',
  breakDone: 'Break taken: a full 4.5 hours ahead',
  day: 'Day',
  week: 'Week',
  fortnight: '2 weeks',
  restDue: 'Daily rest due in',
  restLate: 'Daily rest overdue',
  restTaking: 'Daily rest',
  extended: '10-hour days used',
  reduced: 'Reduced rests used',
  gameClock: 'Game time',
  realClock: 'Real time',
  limits: { break: 'break', day: 'daily limit', week: 'weekly limit', fortnight: 'two-week limit' },
  log: 'Log',
  logEmpty: 'Nothing recorded yet. Drive, and it fills up.',
  violations: {
    continuousDriving: 'Over 4.5 hours driving without a break',
    dailyDriving: 'Daily driving time exceeded',
    dailyRestLate: 'Daily rest not taken in time',
    reducedRests: 'More than 3 reduced daily rests this week',
    weeklyDriving: 'Over 56 hours driving this week',
    fortnightDriving: 'Over 90 hours driving in two weeks',
    overspeed: 'Speeding: over 90 km/h for more than a minute',
  },
  over: '{n} over',
  settings: 'Settings',
  enabled: 'Tachograph',
  enabledDetail: 'Records driving, other work and rest as you play.',
  clockGame: 'Game time',
  clockGameDetail: 'The game’s clock. Sleeping counts as daily rest, as the game’s own fatigue does.',
  clockReal: 'Real time',
  clockRealDetail: 'Your PC’s clock: 4.5 hours means 4.5 real hours. Time away from the game counts as rest.',
  clockNote:
    'The game’s clock runs about 15–20 times faster than real time, so Game time is the default: a full working day fits in one evening and the game’s sleep is your daily rest. Each clock keeps its own log.',
  cabWarnings: 'Warnings on the screen',
  cabWarningsDetail: 'A banner on the truck’s screen and the iPad when a break is due or a limit is passed.',
  clear: 'Clear log',
  clearConfirm: 'Clear for sure?',
  days: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
  profileTitle: 'Profile (from your save)',
  profileDriver: 'Driver',
  profileCompany: 'Company',
  profileCard: 'Card number',
  profileDistance: 'Total driven',
  profileDeliveries: 'Deliveries',
  profileExperience: 'Experience',
  profileMoney: 'Balance',
  profileWaiting: 'Read once the game has saved your profile.',
  driverAuto: 'From your profile; you can change it.',
  lcdNoCard: 'no card',
  lcdLeft: 'left',
  lcdBreak: 'break',
  lcdDay: 'day',
  lcdWeek: 'week',
  lcdFortnight: '2wk',
  lcdRestIn: 'rest in',
  lcdViolations: 'violations',
  lcdNone: 'none',
  lcdPrint: 'printout 24h',
  lcdPrintOk: 'OK = print',
  lcdPrinting: 'printing...',
  cardTitle: 'DRIVER CARD',
  cardHint: 'Drag your card into slot 1 (or click it).',
  cardIn: 'Card in slot 1. Press eject button 1 to take it out.',
  buttonsHint: '▲ ▼ browse · OK printout · ↵ back',
  receiptTitle: 'Daily printout, driver',
  receiptDriver: 'Driver',
  receiptVehicle: 'Vehicle',
  receiptActivities: 'Activities',
  receiptTotals: 'Totals',
  receiptDistance: 'Distance',
  receiptViolations: 'Violations',
  receiptNoViolations: 'No violations',
  receiptSignature: 'Driver signature',
  receiptPrint: 'Print',
  receiptTear: 'Tear off',
  receiptNoCard: 'No card in slot 1',
  receiptNoTruck: 'Unknown',
  driverName: 'Driver name',
  driverPlaceholder: 'Your name on the card',
  paper: 'Receipt printer',
  paperDetail:
    'Paper width of your thermal printer. Print opens Windows’ print dialog: pick your receipt printer there.',
}

const NL: TachoWords = {
  title: 'Tachograaf',
  sub: 'Je rij- en rusttijden, live bijgehouden uit het spel en getoetst aan de EU-regels.',
  off: 'De tachograaf staat uit.',
  activity: { driving: 'Rijden', work: 'Andere werkzaamheden', available: 'Beschikbaarheid', rest: 'Pauze / rust' },
  none: 'Wachten op het spel',
  since: 'sinds',
  untilBreak: 'Rijtijd tot pauze',
  breakTaking: 'Pauze',
  breakDone: 'Pauze genomen: weer 4,5 uur rijden',
  day: 'Dag',
  week: 'Week',
  fortnight: '2 weken',
  restDue: 'Dagrust uiterlijk over',
  restLate: 'Dagrust te laat',
  restTaking: 'Dagrust',
  extended: '10-uursdagen gebruikt',
  reduced: 'Verkorte rusten gebruikt',
  gameClock: 'Speltijd',
  realClock: 'Echte tijd',
  limits: { break: 'pauze', day: 'daglimiet', week: 'weeklimiet', fortnight: 'tweewekenlimiet' },
  log: 'Logboek',
  logEmpty: 'Nog niets vastgelegd. Ga rijden en het vult zich.',
  violations: {
    continuousDriving: 'Meer dan 4,5 uur gereden zonder pauze',
    dailyDriving: 'Dagelijkse rijtijd overschreden',
    dailyRestLate: 'Dagrust niet op tijd genomen',
    reducedRests: 'Meer dan 3 verkorte dagrusten deze week',
    weeklyDriving: 'Meer dan 56 uur gereden deze week',
    fortnightDriving: 'Meer dan 90 uur gereden in twee weken',
    overspeed: 'Snelheidsoverschrijding: langer dan een minuut boven 90 km/u',
  },
  over: '{n} te veel',
  settings: 'Instellingen',
  enabled: 'Tachograaf',
  enabledDetail: 'Legt rijden, andere werkzaamheden en rust vast terwijl je speelt.',
  clockGame: 'Speltijd',
  clockGameDetail: 'De klok van het spel. Slapen telt als dagrust, net als bij de vermoeidheid in het spel.',
  clockReal: 'Echte tijd',
  clockRealDetail: 'De klok van je pc: 4,5 uur is echt 4,5 uur. Tijd buiten het spel telt als rust.',
  clockNote:
    'De klok van het spel loopt zo’n 15 à 20 keer sneller dan echte tijd. Daarom is Speltijd de standaard: een volle werkdag past in één avond en slapen in het spel is je dagrust. Elke klok heeft een eigen logboek.',
  cabWarnings: 'Waarschuwingen op het scherm',
  cabWarningsDetail: 'Een melding op het truckscherm en de iPad als een pauze nodig is of een limiet is overschreden.',
  clear: 'Logboek wissen',
  clearConfirm: 'Zeker wissen?',
  days: ['ma', 'di', 'wo', 'do', 'vr', 'za', 'zo'],
  profileTitle: 'Profiel (uit je save)',
  profileDriver: 'Bestuurder',
  profileCompany: 'Bedrijf',
  profileCard: 'Kaartnummer',
  profileDistance: 'Totaal gereden',
  profileDeliveries: 'Leveringen',
  profileExperience: 'Ervaring',
  profileMoney: 'Saldo',
  profileWaiting: 'Wordt gelezen zodra het spel je profiel heeft opgeslagen.',
  driverAuto: 'Uit je profiel, je kunt hem aanpassen.',
  lcdNoCard: 'geen kaart',
  lcdLeft: 'rest',
  lcdBreak: 'pauze',
  lcdDay: 'dag',
  lcdWeek: 'week',
  lcdFortnight: '2wk',
  lcdRestIn: 'rust over',
  lcdViolations: 'overtreding',
  lcdNone: 'geen',
  lcdPrint: 'afdruk 24h',
  lcdPrintOk: 'OK = printen',
  lcdPrinting: 'printen...',
  cardTitle: 'BESTUURDERSKAART',
  cardHint: 'Sleep je kaart in sleuf 1 (of klik erop).',
  cardIn: 'Kaart zit in sleuf 1. Druk op de uitwerpknop 1 om hem eruit te halen.',
  buttonsHint: '▲ ▼ bladeren · OK afdruk · ↵ terug',
  receiptTitle: 'Dagafdruk bestuurder',
  receiptDriver: 'Bestuurder',
  receiptVehicle: 'Voertuig',
  receiptActivities: 'Activiteiten',
  receiptTotals: 'Totalen',
  receiptDistance: 'Afstand',
  receiptViolations: 'Overtredingen',
  receiptNoViolations: 'Geen overtredingen',
  receiptSignature: 'Handtekening bestuurder',
  receiptPrint: 'Afdrukken',
  receiptTear: 'Afscheuren',
  receiptNoCard: 'Geen kaart in sleuf 1',
  receiptNoTruck: 'Onbekend',
  driverName: 'Naam bestuurder',
  driverPlaceholder: 'Je naam op de kaart',
  paper: 'Bonprinter',
  paperDetail:
    'Papierbreedte van je thermische printer. Afdrukken opent het printvenster van Windows: kies daar je bonprinter.',
}

export function tachoWords(cw: CompanionWords): TachoWords {
  return cw === companionWords('nl_nl') ? NL : EN
}

/** Minutes as h:mm ("-0:12" when over). */
export function hm(minutes: number): string {
  const sign = minutes < 0 ? '-' : ''
  const whole = Math.floor(Math.abs(minutes))
  return `${sign}${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, '0')}`
}

/** A moment on the tachograph's clock: the game's weekday and time, or the PC's. */
export function when(minutes: number, clock: TachoStatus['clock'], days: string[]): string {
  if (clock === 'real') {
    const date = new Date(minutes * 60_000)
    const day = days[(date.getDay() + 6) % 7]
    return `${day} ${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`
  }
  const whole = Math.floor(minutes)
  const day = days[Math.floor(whole / 1440) % 7]
  const time = whole % 1440
  return `${day} ${String(Math.floor(time / 60)).padStart(2, '0')}:${String(time % 60).padStart(2, '0')}`
}
