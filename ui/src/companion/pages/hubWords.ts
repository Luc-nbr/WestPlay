import type { SignInError } from '../omsihub'
import { type CompanionWords, companionWords } from './words'

/** The start screen's and the Profile page's words: Dutch for Dutch, English for every other language. */
export interface HubWords {
  tagline: string
  start: string
  welcome: string
  welcomeBack: string
  signInTitle: string
  signInSub: string
  email: string
  password: string
  signIn: string
  signingIn: string
  create: string
  without: string
  errors: Record<SignInError, string>
  profileTitle: string
  profileSub: string
  signedOutTitle: string
  signedOutSub: string
  account: string
  name: string
  friendCode: string
  emailRow: string
  manage: string
  signOut: string
  copied: string
  copy: string
  driver: string
  or: string
  withProvider: string
  waiting: string
  cancel: string
  privacy: string
  privacyTitle: string
  privacyConsent: string
  privacyAccept: string
}

const EN: HubWords = {
  tagline: 'Built for realism. Made for the road.',
  start: 'Start',
  welcome: 'Welcome to Euro-Hub',
  welcomeBack: 'Welcome back, {name}',
  signInTitle: 'Sign in with Omsi-Hub',
  signInSub: 'One account for Omsi-Hub and Euro-Hub: your profile, your name and your friend code.',
  email: 'Email address',
  password: 'Password',
  signIn: 'Sign in',
  signingIn: 'Signing in…',
  create: 'Create an account',
  without: 'Continue without an account',
  errors: {
    wrong: 'That email address and password don’t match an Omsi-Hub account.',
    unconfirmed: 'Confirm your email address first: open the link in the mail Omsi-Hub sent you.',
    offline: 'Omsi-Hub can’t be reached. Check your internet connection.',
    timeout: 'The sign-in in the browser took too long. Try again.',
    other: 'Signing in didn’t work. Try again in a moment.',
  },
  profileTitle: 'Your profile',
  profileSub: 'Signed in with your Omsi-Hub account.',
  signedOutTitle: 'Profile',
  signedOutSub: 'Sign in with your Omsi-Hub account to have your profile in Euro-Hub too.',
  account: 'Omsi-Hub account',
  name: 'Name',
  friendCode: 'Friend code',
  emailRow: 'Email',
  manage: 'Manage on omsihub.com',
  signOut: 'Sign out',
  copied: 'Copied',
  copy: 'Copy',
  driver: 'Driver',
  or: 'or',
  withProvider: 'Sign in with {name}',
  waiting: 'Sign in with {name} in the browser window that has opened. Euro-Hub continues by itself afterwards.',
  cancel: 'Cancel',
  privacy: 'Privacy Statement',
  privacyTitle: 'Euro-Hub’s Privacy Statement',
  privacyConsent:
    'By continuing, you agree that Euro-Hub signs you in to your Omsi-Hub account and shows your profile, as this Privacy Statement describes.',
  privacyAccept: 'Accept and continue',
}

const NL: HubWords = {
  tagline: 'Gebouwd voor realisme. Gemaakt voor de weg.',
  start: 'Starten',
  welcome: 'Welkom bij Euro-Hub',
  welcomeBack: 'Welkom terug, {name}',
  signInTitle: 'Inloggen met Omsi-Hub',
  signInSub: 'Eén account voor Omsi-Hub en Euro-Hub: je profiel, je naam en je vriendcode.',
  email: 'E-mailadres',
  password: 'Wachtwoord',
  signIn: 'Inloggen',
  signingIn: 'Bezig met inloggen…',
  create: 'Account maken',
  without: 'Doorgaan zonder account',
  errors: {
    wrong: 'Dit e-mailadres en wachtwoord horen niet bij een Omsi-Hub-account.',
    unconfirmed: 'Bevestig eerst je e-mailadres: open de link in de mail van Omsi-Hub.',
    offline: 'Omsi-Hub is niet bereikbaar. Controleer je internetverbinding.',
    timeout: 'Inloggen in de browser duurde te lang. Probeer het nog eens.',
    other: 'Inloggen lukte niet. Probeer het zo nog eens.',
  },
  profileTitle: 'Je profiel',
  profileSub: 'Ingelogd met je Omsi-Hub-account.',
  signedOutTitle: 'Profiel',
  signedOutSub: 'Log in met je Omsi-Hub-account om je profiel ook in Euro-Hub te hebben.',
  account: 'Omsi-Hub-account',
  name: 'Naam',
  friendCode: 'Vriendcode',
  emailRow: 'E-mail',
  manage: 'Beheren op omsihub.com',
  signOut: 'Uitloggen',
  copied: 'Gekopieerd',
  copy: 'Kopiëren',
  driver: 'Chauffeur',
  or: 'of',
  withProvider: 'Inloggen met {name}',
  waiting: 'Log in met {name} in het browservenster dat is geopend. Euro-Hub gaat daarna vanzelf verder.',
  cancel: 'Annuleren',
  privacy: 'Privacyverklaring',
  privacyTitle: 'Privacyverklaring van Euro-Hub',
  privacyConsent:
    'Door verder te gaan, ga je ermee akkoord dat Euro-Hub je inlogt met je Omsi-Hub-account en je profiel toont, zoals deze privacyverklaring beschrijft.',
  privacyAccept: 'Akkoord en verder',
}

export function hubWords(cw: CompanionWords): HubWords {
  return cw === companionWords('nl_nl') ? NL : EN
}
