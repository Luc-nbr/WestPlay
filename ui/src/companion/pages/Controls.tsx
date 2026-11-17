import { useCallback, useEffect, useState } from 'react'
import { call } from '../bridge'
import { useKeyName } from '../keys'
import { Group, KeyChooser, Note, Panel, Row, Slider, Switch } from '../room/parts'
import { n, useWords } from '../room/room'
import { GLOW, openAndClose, useScreenAt, useTour } from '../room/screen'
import {
  type Binding,
  CONTROLLER_ACTIONS,
  type ControllerAction,
  DEFAULT_ANNOUNCE,
  DEFAULT_KEY,
  KEYBIND_IDS,
  type KeybindId,
  SENSITIVITY,
  type Settings,
} from '../useSettings'
import { KeyButton } from './KeyButton'
import type { PageProps } from './page'

/**
 * How the player uses the truck's screen: the key they hold (the onboarding's keycaps), how fast the pointer
 * goes, and whether the truck's own strip stays beside WestPlay; and every keybind (owner 2026-10-21): the
 * screen's key, any key, held or pressed once, and the music's volume and play/pause. On the room's screen
 * the pointer uses it, opening an app and coming back Home, again and again.
 */
export function Controls({ settings: state, cw }: PageProps) {
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
      {settings && <Keybinds settings={settings} update={update} w={w} />}
      {error && <Note tone="bad">{error}</Note>}
    </Panel>
  )
}

/** The wheel's buttons (a Moza's rim, a button box): each bound to what Dashly does. */
export function WheelPage({ settings: state, status, cw }: PageProps) {
  const w = cw.controls
  const { settings, update, error } = state
  useScreenAt('home', GLOW.home, null)
  return (
    <Panel place="page" title={w.wheelButtons} sub={w.wheelSub}>
      {settings && <WheelButtons settings={settings} update={update} status={status} w={w} />}
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
      {settings && (
        <Group title={w.musicSpeakers} place={8}>
          <QualityRow
            saved={100 - (settings.sound.wear ?? 0)}
            w={w}
            save={(value) => update({ ...settings, sound: { ...settings.sound, wear: 100 - value } })}
          />
        </Group>
      )}
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
  w,
}: {
  settings: Settings
  update(settings: Settings): void
  w: PageProps['cw']['controls']
}) {
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
            }
          />
        ))}
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
 * The wheel's buttons (a Moza's rim, a button box: any controller Windows has): each of the keybinds' actions, and
 * the next and previous song, on a button of the player's. Choose one, then press the button on the wheel: the next
 * button pressed is taken (WestPlay reads the controllers: westplay_host::controller).
 */
function WheelButtons({
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
  const [learning, setLearning] = useState<{ action: ControllerAction; from: number } | null>(null)
  const press = status?.controllerPress ?? null
  const names: Record<ControllerAction, string> = {
    volumeUp: w.volumeUp,
    volumeDown: w.volumeDown,
    playPause: w.playPause,
    nextTrack: w.nextTrack,
    previousTrack: w.previousTrack,
    brightnessDown: w.brightnessDown,
    brightnessUp: w.brightnessUp,
    pushToTalk: w.pushToTalk,
    pushToRecord: w.pushToRecord,
    announce: w.announce,
    slot1: w.playSlot.replace('{n}', '1'),
    slot2: w.playSlot.replace('{n}', '2'),
    slot3: w.playSlot.replace('{n}', '3'),
    slot4: w.playSlot.replace('{n}', '4'),
    slot5: w.playSlot.replace('{n}', '5'),
    slot6: w.playSlot.replace('{n}', '6'),
  }
  useEffect(() => {
    if (!learning || !press || press.count <= learning.from) return
    const button = { device: press.device, name: press.name, button: press.button }
    // A button does one thing: taken from whatever had it.
    const controller = Object.fromEntries(
      Object.entries(settings.controller ?? {}).filter(
        ([, other]) => !(other && other.device === button.device && other.button === button.button),
      ),
    )
    update({ ...settings, controller: { ...controller, [learning.action]: button } })
    setLearning(null)
  }, [learning, press, settings, update])
  return (
    <>
      <Group title={w.wheelButtons} place={7}>
        {CONTROLLER_ACTIONS.map((action) => {
          const bound = settings.controller?.[action]
          const listening = learning?.action === action
          return (
            <Row
              key={action}
              title={names[action]}
              detail={
                bound
                  ? w.wheelButton.replaceAll('{name}', bound.name).replaceAll('{n}', String(bound.button))
                  : w.wheelNone
              }
              accessory={
                <span style={{ display: 'flex', gap: 8 }}>
                  {bound && !listening && (
                    <button
                      type="button"
                      className="ob-button cp-keybind"
                      onClick={() => {
                        const controller = { ...settings.controller }
                        delete controller[action]
                        update({ ...settings, controller })
                      }}
                    >
                      {w.wheelClear}
                    </button>
                  )}
                  <button
                    type="button"
                    className="ob-button cp-keybind"
                    onClick={() => setLearning(listening ? null : { action, from: press?.count ?? 0 })}
                  >
                    {listening ? w.wheelPress : w.wheelChoose}
                  </button>
                </span>
              }
            />
          )
        })}
      </Group>
      <Note tone="calm">{w.wheelNote}</Note>
    </>
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
  const [quality, setQuality] = useState(announce.quality)
  const [savedQuality, setSavedQuality] = useState(announce.quality)
  if (announce.quality !== savedQuality) {
    setSavedQuality(announce.quality)
    setQuality(announce.quality)
  }
  const [volume, setVolume] = useState(announce.volume)
  const [saved, setSaved] = useState(announce.volume)
  if (announce.volume !== saved) {
    setSaved(announce.volume)
    setVolume(announce.volume)
  }
  return (
    <>
      <Group title={w.announce} place={6}>
        <Row
          title={w.announceOn}
          detail={w.announceDetail}
          accessory={
            <Switch
              on={announce.on}
              label={w.announceOn}
              change={(on) => update({ ...settings, announce: { ...announce, on } })}
            />
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
            </span>
          }
        />
        <div className="ob-row cp-slider-row">
          <span className="ob-row-text">
            <b>{w.announceVolume}</b>
          </span>
          <span className="ob-row-value">{`${volume}%`}</span>
          <Slider
            value={volume}
            min={0}
            max={100}
            step={5}
            label={w.announceVolume}
            change={setVolume}
            commit={(value) => update({ ...settings, announce: { ...announce, volume: value } })}
          />
        </div>
        <div className="ob-row cp-slider-row">
          <span className="ob-row-text">
            <b>{w.announceQuality}</b>
            <span>{w.announceQualityDetail}</span>
          </span>
          <span className="ob-row-value">
            {quality < 34 ? w.qualityOld : quality < 67 ? w.qualityUsed : w.qualityNew}
          </span>
          <Slider
            value={quality}
            min={0}
            max={100}
            step={5}
            label={w.announceQuality}
            change={setQuality}
            commit={(value) => update({ ...settings, announce: { ...announce, quality: value } })}
          />
        </div>
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

/** The speakers' quality, old to new: its slider follows the thumb, and it's saved once it's let go. */
function QualityRow({ saved, w, save }: { saved: number; w: PageProps['cw']['controls']; save(value: number): void }) {
  const [quality, setQuality] = useState(saved)
  const [lastSaved, setLastSaved] = useState(saved)
  if (saved !== lastSaved) {
    setLastSaved(saved)
    setQuality(saved)
  }
  return (
    <div className="ob-row cp-slider-row">
      <span className="ob-row-text">
        <b>{w.announceQuality}</b>
        <span>{w.announceQualityDetail}</span>
      </span>
      <span className="ob-row-value">{quality < 34 ? w.qualityOld : quality < 67 ? w.qualityUsed : w.qualityNew}</span>
      <Slider value={quality} min={0} max={100} step={5} label={w.announceQuality} change={setQuality} commit={save} />
    </div>
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
