import { type CSSProperties, useEffect, useRef, useState } from 'react'
import { call } from '../bridge'
import { Group, Note, Panel, Row, Switch, Tick } from '../room/parts'
import { n } from '../room/room'
import { GLOW, useScreenAt } from '../room/screen'
import type { TachoLogEntry, TachoStatus } from '../useHostStatus'
import type { PageProps } from './page'
import { ActivityIcon } from './TachoIcons'
import { TachoDevice } from './TachoDevice'
import { hm, type TachoWords, tachoWords, when } from './tachoWords'
import './tacho.css'

function Meter({ label, used, limit }: { label: string; used: number; limit: number }) {
  const share = Math.min(1, Math.max(0, used / limit))
  return (
    <div className="tacho-meter" data-over={used > limit || undefined}>
      <span className="tacho-label">{label}</span>
      <b>{hm(used)}</b>
      <span className="tacho-of">/ {hm(limit)}</span>
      <i style={{ '--share': share } as CSSProperties} />
    </div>
  )
}

/** The display: what the driver does, the driving left until the break, the day's, week's and two weeks'. */
function Display({ tacho, t }: { tacho: TachoStatus; t: TachoWords }) {
  const s = tacho.summary
  const resting = s.activity === 'rest'
  const breakTaken = resting && s.drivingSinceBreak === 0
  const dailyRest = resting && s.restTaken >= 9 * 60
  const urgent = s.nextLeft <= 15
  return (
    <div className="tacho">
      <div className="tacho-lcd" data-activity={s.activity ?? 'none'}>
        <div className="tacho-top">
          <ActivityIcon activity={s.activity} />
          <span className="tacho-doing">{s.activity ? t.activity[s.activity] : t.none}</span>
          {s.activity && (
            <span className="tacho-since">
              {t.since} {hm(s.activityFor)}
            </span>
          )}
          <span className="tacho-clock">
            {when(tacho.now, tacho.clock, t.days)}
            <small>{tacho.clock === 'game' ? t.gameClock : t.realClock}</small>
          </span>
        </div>

        <div className="tacho-main" data-urgent={urgent || undefined} data-over={s.nextLeft <= 0 || undefined}>
          {dailyRest ? (
            <>
              <span className="tacho-label">{t.restTaking}</span>
              <span className="tacho-big">{hm(s.restTaken)}</span>
              <span className="tacho-goal">/ 11:00</span>
            </>
          ) : resting && s.drivingSinceBreak > 0 ? (
            <>
              <span className="tacho-label">{t.breakTaking}</span>
              <span className="tacho-big">{hm(s.breakTaken)}</span>
              <span className="tacho-goal">/ {hm(s.breakNeeded)}</span>
            </>
          ) : breakTaken ? (
            <>
              <span className="tacho-label">{t.breakDone}</span>
              <span className="tacho-big">{hm(Math.min(s.breakLeft, s.nextLeft))}</span>
            </>
          ) : (
            <>
              <span className="tacho-label">
                {t.untilBreak}
                {s.nextLimit !== 'break' && ` (${t.limits[s.nextLimit]})`}
              </span>
              <span className="tacho-big">{hm(s.nextLeft)}</span>
            </>
          )}
          <div className="tacho-blocks" aria-hidden>
            {Array.from({ length: 18 }, (_, i) => (
              <i key={i} data-on={i < Math.round((s.drivingSinceBreak / 270) * 18) || undefined} />
            ))}
          </div>
        </div>

        <div className="tacho-meters">
          <Meter label={t.day} used={s.dayDriving} limit={s.dayLimit} />
          <Meter label={t.week} used={s.weekDriving} limit={56 * 60} />
          <Meter label={t.fortnight} used={s.fortnightDriving} limit={90 * 60} />
        </div>

        <div className="tacho-foot">
          <span data-late={s.restDueIn <= 0 || undefined}>
            {s.restDueIn <= 0 ? t.restLate : t.restDue} {s.restDueIn > 0 && <b>{hm(s.restDueIn)}</b>}
          </span>
          <span>
            {t.extended} <b>{Math.min(s.extendedDaysUsed, 9)}/2</b>
          </span>
          <span>
            {t.reduced} <b>{s.reducedRestsUsed}/3</b>
          </span>
        </div>
      </div>
      <div className="tacho-slots" aria-hidden>
        <span>
          <i />1
        </span>
        <span>
          <i />2
        </span>
      </div>
    </div>
  )
}

function LogRow({ entry, tacho, t }: { entry: TachoLogEntry; tacho: TachoStatus; t: TachoWords }) {
  if (entry.type === 'violation') {
    return (
      <Row
        danger
        wrap
        title={t.violations[entry.kind]}
        detail={when(entry.at, tacho.clock, t.days)}
        value={t.over.replace('{n}', hm(entry.over))}
        accessory={
          <span className="tacho-log-icon" data-violation>
            !
          </span>
        }
      />
    )
  }
  return (
    <Row
      title={t.activity[entry.activity]}
      detail={`${when(entry.start, tacho.clock, t.days)} – ${when(entry.end, tacho.clock, t.days)}`}
      value={hm(entry.end - entry.start)}
      accessory={
        <span className="tacho-log-icon" data-activity={entry.activity}>
          <ActivityIcon activity={entry.activity} />
        </span>
      }
    />
  )
}

/** The Tachograph: its display, its log, and its settings. On the room's screen, the dashboard. */
export function Tachograph({ cw, status, settings: { settings, update } }: PageProps) {
  const t = tachoWords(cw)
  useScreenAt('dashboard', GLOW.dashboard, null)
  const [clearing, setClearing] = useState(false)
  const tacho = status?.tacho ?? null
  const chosen = settings?.tacho
  const profile = tacho?.profile ?? null

  // The driver's name from the game's profile: filled in while it's empty, or still the last profile's (another
  // profile played); a name typed stays.
  const filled = useRef<string | null>(null)
  useEffect(() => {
    if (!settings || !chosen || !profile?.name || profile.name === filled.current) return
    const was = filled.current
    filled.current = profile.name
    if (chosen.driver.trim() === '' || chosen.driver === was) {
      void update({ ...settings, tacho: { ...chosen, driver: profile.name } })
    }
  }, [profile?.name, settings, chosen, update])

  return (
    <Panel place="page" title={t.title} sub={t.sub}>
      {tacho && settings && chosen && (
        <div className="ob-in" style={n(4)}>
          <TachoDevice tacho={tacho} settings={settings} update={(next) => void update(next)} t={t} />
        </div>
      )}
      <div className="cp-columns tacho-page">
        <div>
          {tacho ? (
            <div className="ob-in" style={n(4)}>
              <Display tacho={tacho} t={t} />
            </div>
          ) : (
            <Note tone="calm">{chosen?.enabled === false ? t.off : t.none}</Note>
          )}
        </div>
        <div>
          {tacho && (
            <Group title={t.profileTitle} place={5}>
              {profile ? (
                <>
                  <Row title={t.profileDriver} value={profile.name} />
                  <Row title={t.profileCompany} value={profile.company} />
                  <Row title={t.profileCard} value={profile.cardNumber} />
                  <Row title={t.profileDistance} value={`${profile.distance.toLocaleString('nl-NL')} km`} />
                  <Row title={t.profileDeliveries} value={profile.deliveries.toLocaleString('nl-NL')} />
                  <Row title={t.profileExperience} value={`${profile.experience.toLocaleString('nl-NL')} XP`} />
                  <Row title={t.profileMoney} value={`€ ${profile.money.toLocaleString('nl-NL')}`} />
                </>
              ) : (
                <Row title={t.profileWaiting} wrap />
              )}
            </Group>
          )}
          {tacho && (
            <Group title={t.log} place={5}>
              {tacho.log.length === 0 ? (
                <Row title={t.logEmpty} wrap />
              ) : (
                tacho.log.map((entry, i) => <LogRow key={i} entry={entry} tacho={tacho} t={t} />)
              )}
            </Group>
          )}

          {settings && chosen && (
            <>
              <Group title={t.settings} place={6}>
                <Row
                  title={t.enabled}
                  detail={t.enabledDetail}
                  accessory={
                    <Switch
                      on={chosen.enabled}
                      label={t.enabled}
                      change={(enabled) => void update({ ...settings, tacho: { ...chosen, enabled } })}
                    />
                  }
                />
                <Row
                  title={t.cabWarnings}
                  detail={t.cabWarningsDetail}
                  accessory={
                    <Switch
                      on={chosen.cabWarnings}
                      disabled={!chosen.enabled}
                      label={t.cabWarnings}
                      change={(cabWarnings) => void update({ ...settings, tacho: { ...chosen, cabWarnings } })}
                    />
                  }
                />
              </Group>
              <Group place={7} disabled={!chosen.enabled}>
                <Row
                  title={t.driverName}
                  detail={profile?.name ? t.driverAuto : undefined}
                  accessory={
                    <input
                      className="tacho-input"
                      value={chosen.driver}
                      placeholder={t.driverPlaceholder}
                      maxLength={40}
                      onChange={(event) =>
                        void update({ ...settings, tacho: { ...chosen, driver: event.target.value } })
                      }
                    />
                  }
                />
                <Row
                  title={t.paper}
                  detail={t.paperDetail}
                  wrap
                  accessory={
                    <span className="tacho-paper">
                      {[58, 80].map((mm) => (
                        <button
                          key={mm}
                          type="button"
                          data-on={chosen.paper === mm || undefined}
                          onClick={() => void update({ ...settings, tacho: { ...chosen, paper: mm } })}
                        >
                          {mm} mm
                        </button>
                      ))}
                    </span>
                  }
                />
              </Group>
              <Group place={7} disabled={!chosen.enabled}>
                {(['game', 'real'] as const).map((clock) => (
                  <Row
                    key={clock}
                    title={clock === 'game' ? t.clockGame : t.clockReal}
                    detail={clock === 'game' ? t.clockGameDetail : t.clockRealDetail}
                    accessory={chosen.clock === clock ? <Tick key={clock} /> : undefined}
                    onClick={() => void update({ ...settings, tacho: { ...chosen, clock } })}
                  />
                ))}
              </Group>
              <Note tone="calm">{t.clockNote}</Note>
              <Group place={8} disabled={!chosen.enabled}>
                <Row
                  danger
                  title={clearing ? t.clearConfirm : t.clear}
                  onClick={() => {
                    if (!clearing) {
                      setClearing(true)
                      return
                    }
                    setClearing(false)
                    void call('tacho_clear').catch(() => {})
                  }}
                />
              </Group>
            </>
          )}
        </div>
      </div>
    </Panel>
  )
}
