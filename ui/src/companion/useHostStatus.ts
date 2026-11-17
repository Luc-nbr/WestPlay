import { call, on } from './bridge'
import type { GameKind } from './setup'
import { useEffect, useState } from 'react'
import { followRound } from './mapRound'

/** Emitted by the Tauri backend about once per second (mirrors `westplay_host::Status`). */
export interface HostStatus {
  webviewVersion: string
  adapter: string | null
  /** The graphics card WestPlay's screen is drawn on, and the game's (`null` until the game runs). */
  cabAdapter: string | null
  gameAdapter: string | null
  /** The PC's graphics cards, the high-performance ones first (mirrors `westplay_host::graphics::Adapter`). */
  adapters: Adapter[]
  framesPerSecond: number
  droppedTotal: number
  gameConnected: boolean
  /** Which game is connected (`null`: none). */
  game: GameKind | null
  screenDrawing: boolean
  /** Whether the truck the player is in is one WestPlay shows in, as the plugin last saw (`null`: not known yet). */
  truckSupported: boolean | null
  /** The coach the player drives: how many speakers its passengers' set plays through (`null`: a truck, no game,
   * or the coach's parts not read yet). */
  coachSpeakers: number | null
  /** Whether the game runs as administrator: WestPlay can't read its route, guidance or weather then. */
  gameElevated: boolean
  /** Whether WestPlay is open on the truck's screen (otherwise the truck's original screen shows), as the
   * cabin page last said; `null` until it has. */
  westplayOpen: boolean | null
  /** Whether YouTube is signed in (`null`: not known yet: YouTube hasn't run), and whether Google's sign-in
   * window is open. Nothing else of the account. */
  youtubeSignedIn: boolean | null
  youtubeSigningIn: boolean
  /** The same of Spotify: signed in or not (`null`: not known yet), and its sign-in window open. */
  spotifySignedIn: boolean | null
  spotifySigningIn: boolean
  discordSignedIn: boolean | null
  discordSigningIn: boolean
  /** The same of Apple Music: signed in or not (`null`: not known yet), and Apple's sign-in window open. */
  appleMusicSignedIn: boolean | null
  appleMusicSigningIn: boolean
  /** The map of the profile played (mirrors `westplay_host::maps::MapStatus`). */
  map: MapStatus
  /** The map the cab shows: its folder in WestPlay's maps, and when it was built (seconds since 1970;
   * `null`: none yet). */
  mapFolder: string | null
  mapBuilt: number | null
  /** Every profile's map (mirrors `westplay_host::maps::ProfileMap`): WestPlay makes one for each, one by
   * one, the profile played first. */
  profileMaps: ProfileMap[]
  /** The iPad (Truck Screen › iPad), while switched on: where to open WestPlay on it, its pairing code. */
  ipad: { address: string; code: string; link: string; connected: boolean; fps: number | null } | null
  /** The controller button last pressed (`count` new with each press), for binding one. */
  controllerPress: { device: string; name: string; button: number; count: number } | null
  /** The announcements' slots: each one's length in seconds (null: empty), and what's recording or playing. */
  announce: {
    slots: (number | null)[]
    recording: number | null
    playing: number | null
    /** The microphone heard now (always on, or push-to-talk held). */
    talking: boolean
    /** Push-to-record held: recording now. */
    quickRecording: boolean
    inputs: { id: string; name: string }[]
    outputs: { id: string; name: string }[]
    asioDrivers: string[]
    asioInputs: string[]
    asioOutputs: string[]
    engine: { driver: string; rate: number; buffer: number; inputMs: number; outputMs: number } | null
    error: string | null
  }
  /** The tachograph (mirrors `westplay_host::tacho::TachoStatus`; `null`: switched off). */
  tacho: TachoStatus | null
  /** The touring-coach company (mirrors `westplay_host::touring::TouringStatus`; `null`: switched off). */
  touring?: TouringStatus | null
  error: string | null
}

export type TouringStopKind = 'depot' | 'pickup' | 'coffee' | 'meal' | 'destination' | 'hotel' | 'excursion' | 'dropoff'
export type TripKind = 'dayTrip' | 'tour' | 'school' | 'charter'

/** A place: its name, where in the game's world (metres), how close counts. */
export interface TouringPoint {
  name: string
  x: number
  z: number
  radius: number
  node: string | null
}

export interface TouringStop {
  kind: TouringStopKind
  leg: 'boarding' | 'outbound' | 'stay' | 'return' | 'unloading' | 'toDepot'
  point: TouringPoint
  /** Planned (game minutes), the trip's day, passengers on or off here, when reached and left. */
  at: number
  day: number
  passengers: number
  reached: number | null
  left: number | null
  missed: boolean
}

/** A trip of the catalogue (or driven): the seat's price (a school's or charter's: the contract), game km. */
export interface TouringTrip {
  id: number
  kind: TripKind
  title: string
  destination: string
  days: number
  departure: number
  listed: number
  price: number
  seats: number
  booked: number
  distance: number
  stops: TouringStop[]
}

export interface TouringMood {
  punctuality: number
  comfort: number
  breaks: number
  bonus: number
  total: number
}

export interface TouringScore {
  pickupsLate: number[]
  harshBrakes: number
  harshAccelerations: number
  harshCorners: number
  breaks: number
  overruns: number
  goodStops: number
  shortStops: number
  missedStops: number
  announcements: number
}

export interface TouringActive {
  trip: TouringTrip
  next: number
  phase: 'toFirstPickup' | 'boarding' | 'outbound' | 'stay' | 'return' | 'unloading' | 'toDepot' | 'done'
  onBoard: number
  toCollect: number
  mood: TouringMood
  score: TouringScore
  meter: { litres: number; km: number; ferries: number }
  distance: number | null
  eta: number | null
  late: number | null
  prompt: { kind: string; place: string; at: number | null } | null
}

export interface TouringFinished {
  id: number
  kind: TripKind
  destination: string
  days: number
  departure: number
  ended: number
  passengers: number
  income: number
  costs: { fuel: number; tolls: number; ferries: number; hotel: number; wage: number }
  profit: number
  mood: number
  reputation: number
  cancelled: boolean
}

export interface TouringStatus {
  company: string
  depot: string | null
  balance: number
  reputation: number
  catalogue: TouringTrip[]
  active: TouringActive | null
  history: TouringFinished[]
  recorded: { kind: TouringStopKind; point: TouringPoint }[]
  /** The game's clock (game minutes; `null`: no game). */
  now: number | null
  mapReady: boolean
}

/** The tachograph: its clock (minutes: game minutes, or minutes since 1970), where the driver stands, its log
 * (newest first). Every duration in minutes. */
export interface TachoStatus {
  enabled: boolean
  clock: 'game' | 'real'
  now: number
  summary: {
    activity: 'driving' | 'work' | 'available' | 'rest' | null
    activityFor: number
    drivingSinceBreak: number
    breakLeft: number
    breakNeeded: number
    breakTaken: number
    dayDriving: number
    dayLimit: number
    dayLeft: number
    extendedDaysUsed: number
    reducedRestsUsed: number
    restDueIn: number
    restTaken: number
    weekDriving: number
    weekLeft: number
    fortnightDriving: number
    fortnightLeft: number
    nextLimit: 'break' | 'day' | 'week' | 'fortnight'
    nextLeft: number
  }
  log: TachoLogEntry[]
  /** Live from the game: speed (km/h), kilometres recorded, the truck, the game's clock (game minutes; `null`: no
   * game), the driver's card in its slot. */
  speed: number
  distance: number
  truck: string | null
  gameTime: number | null
  card: boolean
  /** The game's profile played (its save): the card's driver and company, the card's number (the profile's own),
   * and its kilometres, deliveries, experience and money; `null` until read. */
  profile: TachoProfile | null
}

export interface TachoProfile {
  name: string
  company: string
  cardNumber: string
  distance: number
  deliveries: number
  experience: number
  money: number
}

export type TachoLogEntry =
  | { type: 'activity'; activity: 'driving' | 'work' | 'available' | 'rest'; start: number; end: number }
  | {
      type: 'violation'
      kind:
        | 'continuousDriving'
        | 'dailyDriving'
        | 'dailyRestLate'
        | 'reducedRests'
        | 'weeklyDriving'
        | 'fortnightDriving'
        | 'overspeed'
      at: number
      over: number
    }

export interface Adapter {
  /** What the player's choice is kept as (`Settings.graphics.gpu`). */
  id: string
  name: string
}

export interface ProfileMap {
  /** Its game. */
  game: GameKind
  /** The profile's folder, as the setup's check of the game names it (`ProfileCheck.dir`). */
  dir: string
  /** Its map's folder, by name: profiles sharing a map share it (what `prepareMapsAgain` asks by). */
  folder: string
  status: MapStatus
  /** The map it plays (`europe`, or a standalone map mod's), the mods that change it as the game names them, and
   * how many of those can't be read: as the host worked them out from the profile now (setup's check of the game
   * may be older: a mod turned off since). */
  map?: string
  mapMods?: string[]
  unreadable?: number
  /** The map mods left out of its map, as the Mod Manager names them: their files couldn't be read (a damaged
   * or protected archive), so the map was made from the rest. */
  leftOut?: string[]
  /** TruckersMP's map (`dir`: TruckersMP's installation), one for every profile: not a profile's. */
  truckersmp?: boolean
  /** TruckersMP's ProMods servers' map, beside its other servers'. */
  promods?: boolean
}

/** What a map being built is doing (mirrors `westplay_host::maps::MapStage`). */
export type MapStage = 'reading' | 'drawing' | 'tiles' | 'saving'

export type MapStatus =
  | { state: 'unknown' }
  /** WestPlay isn't set up in a game yet. */
  | { state: 'noGame' }
  | { state: 'ready' }
  /** How far, 0 to 1. */
  | { state: 'building'; done: number; stage: MapStage }
  /** To build once the game is closed; `stopped`: its preparation was under way when the game started, and was
   * stopped. */
  | { state: 'waitingForGame'; stopped?: boolean }
  /** To build once the PC has `needed` bytes of memory free (it has `free` now). */
  | { state: 'waitingForMemory'; needed: number; free: number }
  /** Road to Heavy: no map for it. */
  | { state: 'notAllowed' }
  | { state: 'failed'; error: string }
  /** Another profile's map, to make after the maps before it. */
  | { state: 'queued' }

/** Latest host status, or `null` until the first update arrives. */
export function useHostStatus(): HostStatus | null {
  const [status, setStatus] = useState<HostStatus | null>(null)
  useEffect(() => {
    return on<HostStatus>('host-status', (next) => {
      followRound(next)
      setStatus(next)
    })
  }, [])
  return status
}

/** Opens Google's sign-in for YouTube in a window on the desktop. */
export function youtubeSignIn(): Promise<void> {
  return call('youtube_sign_in')
}

/** Signs YouTube out. */
export function youtubeSignOut(): Promise<void> {
  return call('youtube_sign_out')
}

/** Runs YouTube (hidden), so the status says whether it's signed in. */
export function youtubeCheck(): Promise<void> {
  return call('youtube_check')
}

/** Opens Spotify's sign-in in a window on the desktop. */
export function spotifySignIn(): Promise<void> {
  return call('spotify_sign_in')
}

/** Signs Spotify out. */
export function spotifySignOut(): Promise<void> {
  return call('spotify_sign_out')
}

/** Runs Spotify's web player (hidden), so the status says whether it's signed in. */
export function spotifyCheck(): Promise<void> {
  return call('spotify_check')
}

/** Opens Discord's sign-in in a window on the desktop (the cab's Discord app's profile). */
export function discordSignIn(): Promise<void> {
  return call('discord_sign_in')
}

/** Signs Discord out. */
export function discordSignOut(): Promise<void> {
  return call('discord_sign_out')
}

/** Finds out (hidden) whether Discord is signed in. */
export function discordCheck(): Promise<void> {
  return call('discord_check')
}

/** Opens Apple's sign-in for Apple Music in a window on the desktop. */
export function appleMusicSignIn(): Promise<void> {
  return call('apple_music_sign_in')
}

/** Signs Apple Music out. */
export function appleMusicSignOut(): Promise<void> {
  return call('apple_music_sign_out')
}

/** Opens the support page (Kreosus) in the player's browser. */
export function openSupportPage(): Promise<void> {
  return call('open_support_page')
}

/** WestPlay's pages elsewhere, at the foot of the list. */
export type Social = 'youtube' | 'discord' | 'patreon'

/** Opens one of WestPlay's pages elsewhere in the player's browser (the app holds their addresses). */
export function openSocial(site: Social): Promise<void> {
  return call('open_social', { site })
}

/** Where this run's app.log is: Documents\WestPlay\logs, or WestPlay's own folder where Documents refused it. */
export function appLogPlace(): Promise<string | null> {
  return call('app_log_place')
}

/** Runs Apple Music's web player (hidden), so the status says whether it's signed in. */
export function appleMusicCheck(): Promise<void> {
  return call('apple_music_check')
}

/** Opens or closes WestPlay on the truck's screen (the status then reports it, once the cabin has). */
export function setWestplayOpen(open: boolean): Promise<void> {
  return call('set_westplay_open', { open })
}
