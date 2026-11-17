import { useCallback, useEffect, useRef, useState } from 'react'
import { call } from '../bridge'
import { useKeyName } from '../keys'
import { Group, KeyChooser, Note, Panel, Row, Slider, Switch } from '../room/parts'
import { n, useWords } from '../room/room'
import { GLOW, openAndClose, useScreenAt, useTour } from '../room/screen'
import {
  type Binding,
  type ControllerAction,
  DEFAULT_ANNOUNCE,
  DEFAULT_KEY,
  KEYBIND_IDS,
  type KeybindId,
  SENSITIVITY,
  type Settings,
} from '../useSettings'
import { EqCurve, type Tone } from './EqCurve'
import { KeyButton } from './KeyButton'
import type { PageProps } from './page'

/**
 * How the player uses the truck's screen: the key they hold (the onboarding's keycaps), how fast the pointer
 * goes, and whether the truck's own strip stays beside WestPlay; and every keybind (owner 2026-10-21): the
 * screen's key, any key, held or pressed once, and the music's volume and play/pause. On the room's screen
 * the pointer uses it, opening an app and coming back Home, again and again.
 */
export function Controls({ settings: state, status, cw }: PageProps) {
  const w = cw.controls
  const { settings, update, error } = state
  useScreenAt('home', GLOW.home, null)
  useTour([...openAndClose('music', GLOW.music), ...openAndClose('maps', GLOW.maps)])
  return (
    <Panel place="page" title={w.title} sub={w.sub}>
      <p className="ob-group-title ob-in" style={n(4)}>
        {settings?.interaction.mode === 'toggle' ? w.keyToggle : w.key}
      </p>
      <KeyChooser
        value={settings?.interaction.key ?? DEFAULT_KEY}
        disabled={!settings}
        place={4}
        choose={(key) => settings && update({ ...settings, interaction: { ...settings.interaction, key } })}
      />
      {settings && (
        <Group title={w.pointer} place={5}>
          <SpeedRow
            saved={settings.interaction.sensitivity}
            label={w.speed}
            detail={w.speedDetail}
            save={(sensitivity) => update({ ...settings, interaction: { ...settings.interaction, sensitivity } })}
          />
          <Row
            title={w.strip}
            detail={w.stripDetail}
            accessory={
              <Switch
                on={settings.screen.truckStrip === 'auto'}
                label={w.strip}
                change={(on) =>
                  update({ ...settings, screen: { ...settings.screen, truckStrip: on ? 'auto' : 'off' } })
                }
              />
            }
          />
        </Group>
      )}
      {settings && <Keybinds settings={settings} update={update} status={status} w={w} />}
      {error && <Note tone="bad">{error}</Note>}
    </Panel>
  )
}

/** The coach's PA: the microphone live, push-to-talk, and the recorded announcements. */
export function AnnouncePage({ settings: state, status, cw }: PageProps) {
  const w = cw.controls
  const { settings, update, error } = state
  useScreenAt('home', GLOW.home, null)
  return (
    <Panel place="page" title={w.announce} sub={w.announceSub}>
      {settings && <Announce settings={settings} update={update} status={status} w={w} />}
      {error && <Note tone="bad">{error}</Note>}
    </Panel>
  )
}

/** The sound: the driver, devices and buffer (as a DAW's), and how worn the music's speakers sound. */
export function AudioPage({ settings: state, status, cw }: PageProps) {
  const w = cw.controls
  const { settings, update, error } = state
  useScreenAt('home', GLOW.home, null)
  return (
    <Panel place="page" title={w.audio} sub={w.audioSub}>
      {settings && <AudioSetup settings={settings} update={update} status={status} w={w} />}
      {settings && <MusicSound settings={settings} update={update} w={w} />}
      {error && <Note tone="bad">{error}</Note>}
    </Panel>
  )
}

/**
 * Every keybind: the screen's key (any key, not only the keycaps above), held or pressed once, and the music's
 * volume up and down and play/pause, each alone or with the screen's key. One listens for its key at a time;
 * a key taken already (the screen's, or another keybind's the same way) is refused, saying what it's for.
 */
function Keybinds({
  settings,
  update,
  status,
  w,
}: {
  settings: Settings
  update(settings: Settings): void
  status: PageProps['status']
  w: PageProps['cw']['controls']
}) {
  const wheel = { settings, update, status, w }
  const words = useWords()
  const [listening, setListening] = useState<KeybindId | 'screen' | null>(null)
  const [refused, setRefused] = useState<string | null>(null)
  const screenKey = settings.interaction.key
  const keyName = useKeyName(screenKey, words)
  const names: Record<KeybindId, string> = {
    volumeUp: w.volumeUp,
    volumeDown: w.volumeDown,
    playPause: w.playPause,
    brightnessDown: w.brightnessDown,
    brightnessUp: w.brightnessUp,
  }
  const listen = useCallback(
    (which: KeybindId | 'screen') => (on: boolean) => {
      setListening(on ? which : null)
      if (on) setRefused(null)
    },
    [],
  )
  // What `binding` would clash with, if anything: the screen's key, or another keybind pressed the same way.
  const clash = (binding: Binding, except: KeybindId | 'screen'): string | null => {
    if (except !== 'screen' && binding.key === screenKey) return w.useScreen
    for (const id of KEYBIND_IDS) {
      if (id === except) continue
      const other = settings.keybinds[id]
      if (except === 'screen' ? other.key === binding.key : sameKeys(other, binding)) return names[id]
    }
    return null
  }
  const choose = (which: KeybindId | 'screen') => (binding: Binding) => {
    const taken = clash(binding, which)
    if (taken) {
      setRefused(w.inUse.replaceAll('{action}', taken))
      return
    }
    setRefused(null)
    if (which === 'screen') update({ ...settings, interaction: { ...settings.interaction, key: binding.key } })
    else update({ ...settings, keybinds: { ...settings.keybinds, [which]: binding } })
  }
  return (
    <>
      <Group title={w.keybinds} place={6}>
        <Row
          title={w.useScreen}
          accessory={
            <KeyButton
              binding={{ key: screenKey, withInteractionKey: false }}
              label={w.useScreen}
              listening={listening === 'screen'}
              listen={listen('screen')}
              choose={choose('screen')}
              words={words}
              pressKey={w.pressKey}
            />
          }
        />
        <Row
          title={w.toggle}
          detail={w.toggleDetail.replaceAll('{key}', keyName)}
          accessory={
            <Switch
              on={settings.interaction.mode === 'toggle'}
              label={w.toggle}
              change={(on) =>
                update({ ...settings, interaction: { ...settings.interaction, mode: on ? 'toggle' : 'hold' } })
              }
            />
          }
        />
        {KEYBIND_IDS.map((id) => (
          <Row
            key={id}
            title={names[id]}
            accessory={
              <span className="dashly-binds">
                <KeyButton
                  binding={settings.keybinds[id]}
                  with={screenKey}
                  label={names[id]}
                  listening={listening === id}
                  listen={listen(id)}
                  choose={choose(id)}
                  words={words}
                  pressKey={w.pressKey}
                />
                <WheelBind action={id} {...wheel} />
              </span>
            }
          />
        ))}
        <Row title={w.nextTrack} accessory={<WheelBind action="nextTrack" {...wheel} />} />
        <Row title={w.previousTrack} accessory={<WheelBind action="previousTrack" {...wheel} />} />
        <Row title={w.discordMute} accessory={<WheelBind action="discordMute" {...wheel} />} />
        <Row title={w.discordDeafen} accessory={<WheelBind action="discordDeafen" {...wheel} />} />
        <Row title={w.discordLeave} accessory={<WheelBind action="discordLeave" {...wheel} />} />
      </Group>
      <Note tone={refused ? 'bad' : 'calm'}>{refused ?? w.keybindsNote.replaceAll('{key}', keyName)}</Note>
    </>
  )
}

/** Two bindings pressed the same way: the same key, alone or with the screen's key both. */
function sameKeys(a: Binding, b: Binding): boolean {
  return a.key === b.key && a.withInteractionKey === b.withInteractionKey
}

/** The pointer's speed: its slider follows the thumb, and it's saved once it's let go. A value saved
 * elsewhere (another window) resets it. */
function SpeedRow({
  saved,
  label,
  detail,
  save,
}: {
  saved: number
  label: string
  detail: string
  save(value: number): void
}) {
  const [dragging, setDragging] = useState(saved)
  const [lastSaved, setLastSaved] = useState(saved)
  if (saved !== lastSaved) {
    setLastSaved(saved)
    setDragging(saved)
  }
  return (
    <div className="ob-row cp-slider-row">
      <span className="ob-row-text">
        <b>{label}</b>
        <span>{detail}</span>
      </span>
      <span className="ob-row-value">{`${dragging.toFixed(1)}×`}</span>
      <Slider
        value={dragging}
        min={SENSITIVITY.min}
        max={SENSITIVITY.max}
        step={SENSITIVITY.step}
        label={label}
        change={setDragging}
        commit={save}
      />
    </div>
  )
}

/**
 * A wheel button (a Moza's rim, a button box: any controller Windows has) for `action`, beside its key wherever a key
 * is bound: a round button with a wheel. Click it, then press the button on the wheel; right-click clears it. A
 * button does one thing: taken from whatever had it (WestPlay reads the controllers: westplay_host::controller).
 */
function WheelBind({
  action,
  settings,
  update,
  status,
  w,
}: {
  action: ControllerAction
  settings: Settings
  update(settings: Settings): void
  status: PageProps['status']
  w: PageProps['cw']['controls']
}) {
  const [learning, setLearning] = useState<number | null>(null)
  const press = status?.controllerPress ?? null
  const bound = settings.controller?.[action]
  useEffect(() => {
    if (learning === null || !press || press.count <= learning) return
    const button = { device: press.device, name: press.name, button: press.button }
    const controller = Object.fromEntries(
      Object.entries(settings.controller ?? {}).filter(
        ([, other]) => !(other && other.device === button.device && other.button === button.button),
      ),
    )
    update({ ...settings, controller: { ...controller, [action]: button } })
    setLearning(null)
  }, [learning, press, settings, update, action])
  const title =
    learning !== null
      ? w.wheelPress
      : bound
        ? `${w.wheelButton.replaceAll('{name}', bound.name).replaceAll('{n}', String(bound.button))} \u2014 ${w.wheelClearHint}`
        : w.wheelChooseHint
  return (
    <button
      type="button"
      className="dashly-wheel"
      data-bound={bound ? '' : undefined}
      data-learning={learning !== null ? '' : undefined}
      title={title}
      aria-label={title}
      onClick={() => setLearning(learning !== null ? null : (press?.count ?? 0))}
      onContextMenu={(event) => {
        event.preventDefault()
        if (!bound) return
        const controller = { ...settings.controller }
        delete controller[action]
        update({ ...settings, controller })
      }}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="12" r="8.5" />
        <circle cx="12" cy="12" r="2.2" />
        <path d="M3.8 10.5h6M14.2 10.5h6M12 14.2v6.3" />
      </svg>
      {bound && <b>{bound.button}</b>}
    </button>
  )
}

/** The microphone live through the coach's speakers: on, and how loud (westplay_host::announce). */
function Announce({
  settings,
  update,
  status,
  w,
}: {
  settings: Settings
  update(settings: Settings): void
  status: PageProps['status']
  w: PageProps['cw']['controls']
}) {
  const slots = status?.announce ?? { slots: [null, null, null, null, null, null], recording: null, playing: null }
  const words = useWords()
  const [listening, setListening] = useState(false)
  const [listeningRecord, setListeningRecord] = useState(false)
  const press = (slot: number, action: string) => void call('announce_slot', { slot, action })
  const announce: Settings['announce'] = { ...DEFAULT_ANNOUNCE, ...settings.announce }
  return (
    <>
      <Group title={w.announce} place={6}>
        <Row
          title={w.announceOn}
          detail={w.announceDetail}
          accessory={
            <span className="dashly-binds">
              <WheelBind action="announce" settings={settings} update={update} status={status} w={w} />
              <Switch
                on={announce.on}
                label={w.announceOn}
                change={(on) => update({ ...settings, announce: { ...announce, on } })}
              />
            </span>
          }
        />
        <Row
          title={w.duck}
          detail={w.duckDetail}
          accessory={
            <Switch
              on={announce.duck}
              label={w.duck}
              change={(duck) => update({ ...settings, announce: { ...announce, duck } })}
            />
          }
        />
        <Row
          title={w.pushToTalk}
          detail={status?.announce?.talking ? w.talkingNow : w.pushToTalkDetail}
          accessory={
            <span style={{ display: 'flex', gap: 8 }}>
              {announce.pttKey !== null && !listening && (
                <button
                  type="button"
                  className="ob-button cp-keybind"
                  onClick={() => update({ ...settings, announce: { ...announce, pttKey: null } })}
                >
                  {w.wheelClear}
                </button>
              )}
              {announce.pttKey === null && !listening ? (
                <button type="button" className="ob-button cp-keybind" onClick={() => setListening(true)}>
                  {w.wheelChoose}
                </button>
              ) : (
                <KeyButton
                  binding={{ key: announce.pttKey ?? 0, withInteractionKey: false }}
                  label={w.pushToTalk}
                  listening={listening}
                  listen={setListening}
                  choose={(binding) => update({ ...settings, announce: { ...announce, pttKey: binding.key } })}
                  words={words}
                  pressKey={w.pressKey}
                />
              )}
              <WheelBind action="pushToTalk" settings={settings} update={update} status={status} w={w} />
            </span>
          }
        />
        <Row
          title={w.pushToRecord}
          detail={status?.announce?.quickRecording ? w.recordingNow : w.pushToRecordDetail}
          accessory={
            <span style={{ display: 'flex', gap: 8 }}>
              {announce.ptrKey !== null && !listeningRecord && (
                <button
                  type="button"
                  className="ob-button cp-keybind"
                  onClick={() => update({ ...settings, announce: { ...announce, ptrKey: null } })}
                >
                  {w.wheelClear}
                </button>
              )}
              {announce.ptrKey === null && !listeningRecord ? (
                <button type="button" className="ob-button cp-keybind" onClick={() => setListeningRecord(true)}>
                  {w.wheelChoose}
                </button>
              ) : (
                <KeyButton
                  binding={{ key: announce.ptrKey ?? 0, withInteractionKey: false }}
                  label={w.pushToRecord}
                  listening={listeningRecord}
                  listen={setListeningRecord}
                  choose={(binding) => update({ ...settings, announce: { ...announce, ptrKey: binding.key } })}
                  words={words}
                  pressKey={w.pressKey}
                />
              )}
              <WheelBind action="pushToRecord" settings={settings} update={update} status={status} w={w} />
            </span>
          }
        />
        <LiveSlider
          label={w.announceVolume}
          saved={announce.volume}
          format={(value) => `${value}%`}
          save={(volume) => update({ ...settings, announce: { ...announce, volume } })}
        />
        <LiveSlider
          label={w.announceQuality}
          detail={w.announceQualityDetail}
          saved={announce.quality}
          format={(value) => (value < 34 ? w.qualityOld : value < 67 ? w.qualityUsed : w.qualityNew)}
          save={(quality) => update({ ...settings, announce: { ...announce, quality } })}
        />
        <LiveSlider
          label={w.distortion}
          detail={w.distortionDetail}
          saved={announce.drive}
          format={(value) => `${value}%`}
          save={(drive) => update({ ...settings, announce: { ...announce, drive } })}
        />
      </Group>
      <Group title={w.slots} place={6}>
        {slots.slots.map((length, slot) => {
          const recording = slots.recording === slot
          const playing = slots.playing === slot
          return (
            <Row
              key={slot}
              title={w.slot.replace('{n}', String(slot + 1))}
              detail={recording ? w.recordingNow : length === null ? w.slotEmpty : `${length.toFixed(1)} s`}
              accessory={
                <span className="dashly-slot">
                  <WheelBind
                    action={`slot${slot + 1}` as ControllerAction}
                    settings={settings}
                    update={update}
                    status={status}
                    w={w}
                  />
                  {length !== null && !recording && (
                    <button
                      type="button"
                      className="dashly-round"
                      data-kind="clear"
                      title={w.wheelClear}
                      aria-label={w.wheelClear}
                      onClick={() => press(slot, 'clear')}
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="M9 4h6M5 7h14M7 7l1 12h8l1-12" />
                      </svg>
                    </button>
                  )}
                  {length !== null && !recording && (
                    <button
                      type="button"
                      className="dashly-round"
                      data-kind="play"
                      data-active={playing || undefined}
                      title={playing ? w.stopPlaying : w.play}
                      aria-label={playing ? w.stopPlaying : w.play}
                      onClick={() => press(slot, 'play')}
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        {playing ? <rect x="7" y="7" width="10" height="10" rx="2" /> : <path d="M8 5.5v13l11-6.5z" />}
                      </svg>
                    </button>
                  )}
                  <button
                    type="button"
                    className="dashly-round"
                    data-kind="record"
                    data-active={recording || undefined}
                    title={recording ? w.stopRecording : w.record}
                    aria-label={recording ? w.stopRecording : w.record}
                    onClick={() => press(slot, 'record')}
                  >
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                      {recording ? (
                        <rect x="7" y="7" width="10" height="10" rx="2" />
                      ) : (
                        <circle cx="12" cy="12" r="6" />
                      )}
                    </svg>
                  </button>
                </span>
              }
            />
          )
        })}
      </Group>
      <Note tone="calm">{w.announceNote}</Note>
    </>
  )
}

/**
 * A slider that changes the sound as it moves: saved while it's dragged (at most every 120 ms) and once let go, so
 * what's heard follows the thumb. A value saved elsewhere (the cab, another window) moves it.
 */
function LiveSlider({
  label,
  detail,
  saved,
  min = 0,
  max = 100,
  step = 5,
  format,
  save,
}: {
  label: string
  detail?: string
  saved: number
  min?: number
  max?: number
  step?: number
  format(value: number): string
  save(value: number): void
}) {
  const [value, setValue] = useState(saved)
  const [lastSaved, setLastSaved] = useState(saved)
  const sent = useRef(0)
  if (saved !== lastSaved) {
    setLastSaved(saved)
    setValue(saved)
  }
  return (
    <div className="ob-row cp-slider-row">
      <span className="ob-row-text">
        <b>{label}</b>
        {detail && <span>{detail}</span>}
      </span>
      <span className="ob-row-value">{format(value)}</span>
      <Slider
        value={value}
        min={min}
        max={max}
        step={step}
        label={label}
        change={(next) => {
          setValue(next)
          const now = performance.now()
          if (now - sent.current >= 120) {
            sent.current = now
            save(next)
          }
        }}
        commit={save}
      />
    </div>
  )
}

/** A sound preset: the tone (dB each), the bass boost and the distortion (0 to 100). */
interface Preset {
  id: string
  bass: number
  mid: number
  treble: number
  boost: number
  drive: number
}

const PRESETS: Preset[] = [
  { id: 'flat', bass: 0, mid: 0, treble: 0, boost: 0, drive: 0 },
  { id: 'bassBoost', bass: 4, mid: 0, treble: 1, boost: 60, drive: 0 },
  { id: 'superBass', bass: 6, mid: -1, treble: 0, boost: 100, drive: 15 },
  { id: 'vocal', bass: -2, mid: 4, treble: 2, boost: 0, drive: 0 },
  { id: 'bright', bass: -1, mid: 1, treble: 5, boost: 0, drive: 0 },
  { id: 'loudness', bass: 4, mid: -1, treble: 4, boost: 20, drive: 0 },
  { id: 'rock', bass: 3, mid: -1, treble: 3, boost: 20, drive: 20 },
  { id: 'radio', bass: -4, mid: 5, treble: -4, boost: 0, drive: 35 },
  { id: 'distorted', bass: 2, mid: 1, treble: 1, boost: 40, drive: 80 },
]

/**
 * The music's sound on the truck's speakers (every music app's): a preset, or each part by hand: bass, mid and treble
 * (the cab's own Equalizer's), a bass boost under them, distortion, and how worn the speakers are. Live as moved.
 */
function MusicSound({
  settings,
  update,
  w,
}: {
  settings: Settings
  update(settings: Settings): void
  w: PageProps['cw']['controls']
}) {
  const sound = settings.sound
  const curve = sound.custom ? sound.equalizer : { bass: 0, mid: 0, treble: 0 }
  const set = (change: Partial<Settings['sound']>) => update({ ...settings, sound: { ...sound, ...change } })
  const chosen = PRESETS.find(
    (preset) =>
      preset.bass === curve.bass &&
      preset.mid === curve.mid &&
      preset.treble === curve.treble &&
      preset.boost === (sound.boost ?? 0) &&
      preset.drive === (sound.drive ?? 0),
  )
  return (
    <>
      <Group title={w.musicSound} place={8}>
        <div className="ob-row dashly-presets">
          {PRESETS.map((preset) => (
            <button
              key={preset.id}
              type="button"
              className="dashly-preset"
              data-chosen={chosen?.id === preset.id || undefined}
              onClick={() =>
                set({
                  custom: preset.id !== 'flat',
                  equalizer: { bass: preset.bass, mid: preset.mid, treble: preset.treble },
                  boost: preset.boost,
                  drive: preset.drive,
                })
              }
            >
              {w.presets[preset.id as keyof typeof w.presets]}
            </button>
          ))}
        </div>
        <ToneCurve sound={sound} curve={curve} set={set} w={w} />
        <LiveSlider
          label={w.distortion}
          detail={w.distortionDetail}
          saved={sound.drive ?? 0}
          format={(v) => `${v}%`}
          save={(drive) => set({ drive })}
        />
        <LiveSlider
          label={w.announceQuality}
          detail={w.announceQualityDetail}
          saved={100 - (sound.wear ?? 0)}
          format={(v) => (v < 34 ? w.qualityOld : v < 67 ? w.qualityUsed : w.qualityNew)}
          save={(v) => set({ wear: 100 - v })}
        />
      </Group>
      <Note tone="calm">{w.musicSoundNote}</Note>
    </>
  )
}

const BUFFERS = [32, 64, 128, 256, 512, 1024, 2048]

/**
 * The announcements' audio, as a DAW's audio settings: the driver (WASAPI shared with every app, WASAPI with the
 * microphone WestPlay's alone, or the interface's ASIO driver), the devices or the ASIO channels, and the buffer,
 * with the latencies it makes (westplay_host::announce).
 */
function AudioSetup({
  settings,
  update,
  status,
  w,
}: {
  settings: Settings
  update(settings: Settings): void
  status: PageProps['status']
  w: PageProps['cw']['controls']
}) {
  const announce: Settings['announce'] = { ...DEFAULT_ANNOUNCE, ...settings.announce }
  const audio = status?.announce
  const set = (change: Partial<Settings['announce']>) => update({ ...settings, announce: { ...announce, ...change } })
  const engine = audio?.engine ?? null
  const rate = engine?.rate ?? 48000
  const choice = (value: string, options: [string, string][], change: (value: string) => void, label: string) => (
    <select
      className="ob-button cp-keybind cp-select"
      aria-label={label}
      value={value}
      onChange={(event) => change(event.target.value)}
    >
      {options.map(([key, name]) => (
        <option key={key} value={key}>
          {name}
        </option>
      ))}
    </select>
  )
  const devices = (list: { id: string; name: string }[] | undefined): [string, string][] => [
    ['', w.audioDefault],
    ...(list ?? []).map((device): [string, string] => [device.id, device.name]),
  ]
  const channels = (names: string[] | undefined, pairs: boolean): [string, string][] => {
    const known = names && names.length > 0 ? names : Array.from({ length: 8 }, (_, i) => String(i + 1))
    return known
      .map((name, i): [string, string] => [String(i), pairs && known[i + 1] ? `${name} + ${known[i + 1]}` : name])
      .filter((_, i) => !pairs || i % 2 === 0)
  }
  return (
    <>
      <Group title={w.audio} place={6}>
        <Row
          title={w.audioDriver}
          accessory={choice(
            announce.driver,
            [
              ['wasapi', 'WASAPI'],
              ['exclusive', w.audioExclusive],
              ['asio', 'ASIO'],
            ],
            (driver) => set({ driver: driver as Settings['announce']['driver'] }),
            w.audioDriver,
          )}
        />
        {announce.driver !== 'asio' && (
          <>
            <Row
              title={w.audioInput}
              accessory={choice(
                announce.input ?? '',
                devices(audio?.inputs),
                (id) => set({ input: id || null }),
                w.audioInput,
              )}
            />
            <Row
              title={w.audioOutput}
              accessory={choice(
                announce.output ?? '',
                devices(audio?.outputs),
                (id) => set({ output: id || null }),
                w.audioOutput,
              )}
            />
          </>
        )}
        {announce.driver === 'asio' && (
          <>
            <Row
              title={w.asioDriver}
              accessory={choice(
                announce.asioDriver ?? audio?.asioDrivers[0] ?? '',
                (audio?.asioDrivers ?? []).length > 0
                  ? (audio?.asioDrivers ?? []).map((name): [string, string] => [name, name])
                  : [['', w.asioNone]],
                (name) => set({ asioDriver: name || null }),
                w.asioDriver,
              )}
            />
            <Row
              title={w.audioInput}
              accessory={choice(
                String(announce.asioInput),
                channels(audio?.asioInputs, false),
                (i) => set({ asioInput: Number(i) }),
                w.audioInput,
              )}
            />
            <Row
              title={w.audioOutput}
              accessory={choice(
                String(announce.asioOutput),
                channels(audio?.asioOutputs, true),
                (i) => set({ asioOutput: Number(i) }),
                w.audioOutput,
              )}
            />
            <Row
              title={w.streamCopy}
              detail={w.streamCopyDetail}
              accessory={
                <Switch on={announce.streamCopy} label={w.streamCopy} change={(on) => set({ streamCopy: on })} />
              }
            />
            {announce.streamCopy && (
              <Row
                title={w.streamOutput}
                accessory={choice(
                  announce.streamOutput ?? '',
                  devices(audio?.outputs),
                  (id) => set({ streamOutput: id || null }),
                  w.streamOutput,
                )}
              />
            )}
          </>
        )}
        <Row
          title={w.audioBuffer}
          detail={w.audioBufferDetail}
          accessory={choice(
            String(announce.buffer),
            BUFFERS.map((frames): [string, string] => [
              String(frames),
              `${frames} (${((frames * 1000) / rate).toFixed(1)} ms)`,
            ]),
            (frames) => set({ buffer: Number(frames) }),
            w.audioBuffer,
          )}
        />
        <Row
          title={w.audioLatency}
          detail={
            engine
              ? w.audioLatencyDetail
                  .replace('{driver}', engine.driver)
                  .replace('{rate}', String(engine.rate))
                  .replace('{buffer}', String(engine.buffer))
              : w.audioIdle
          }
          value={engine ? `${(engine.inputMs + engine.outputMs).toFixed(1)} ms` : undefined}
        />
      </Group>
      {audio?.error && <Note tone="bad">{audio.error}</Note>}
      <Note tone="calm">{w.audioNote}</Note>
    </>
  )
}

/** The tone as an equalizer's curve (EqCurve.tsx): what's heard follows the handles as they're dragged (saved at
 * most every 120 ms), and once let go. */
function ToneCurve({
  sound,
  curve,
  set,
  w,
}: {
  sound: Settings['sound']
  curve: { bass: number; mid: number; treble: number }
  set(change: Partial<Settings['sound']>): void
  w: PageProps['cw']['controls']
}) {
  const saved: Tone = { bass: curve.bass, mid: curve.mid, treble: curve.treble, boost: sound.boost ?? 0 }
  const [tone, setTone] = useState<Tone | null>(null)
  const sent = useRef(0)
  const save = (next: Tone) =>
    set({ custom: true, equalizer: { bass: next.bass, mid: next.mid, treble: next.treble }, boost: next.boost })
  const shown = tone ?? saved
  const db = (value: number) => `${value > 0 ? '+' : ''}${value} dB`
  return (
    <div className="ob-row dashly-eq-row">
      <EqCurve
        tone={shown}
        names={{ bass: w.bass, mid: w.mid, treble: w.treble, boost: w.bassBoost }}
        change={(next) => {
          setTone(next)
          const now = performance.now()
          if (now - sent.current >= 120) {
            sent.current = now
            save(next)
          }
        }}
        commit={(next) => {
          save(next)
          setTone(null)
        }}
      />
      <div className="dashly-eq-values">
        <span>
          <b>{w.bassBoost}</b>
          {shown.boost}%
        </span>
        <span>
          <b>{w.bass}</b>
          {db(shown.bass)}
        </span>
        <span>
          <b>{w.mid}</b>
          {db(shown.mid)}
        </span>
        <span>
          <b>{w.treble}</b>
          {db(shown.treble)}
        </span>
      </div>
    </div>
  )
}
