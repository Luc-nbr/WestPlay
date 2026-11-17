import { useState } from 'react'
import { call } from '../bridge'
import { Button, Group, Note, Panel, Row, Switch } from '../room/parts'
import { GLOW, useScreenAt } from '../room/screen'
import type { TouringActive, TouringStatus, TouringStopKind, TouringTrip } from '../useHostStatus'
import { TOURING_DEFAULTS, type TouringSettings } from '../useSettings'
import type { PageProps } from './page'
import { euro, gameWhen, lateWords, type TouringWords, touringWords } from './touringWords'
import './touring.css'

/** What the page asks of the company (`westplay_host::touring::Action`). */
type Action =
  | { action: 'start'; id: number }
  | { action: 'cancel' }
  | { action: 'refresh' }
  | { action: 'record'; kind: TouringStopKind; name: string }
  | { action: 'forget'; index: number }
  | { action: 'skip' }
  | { action: 'reset' }

function act(action: Action) {
  void call('touring_action', { action }).catch(() => {})
}

const RECORDABLE: TouringStopKind[] = ['pickup', 'coffee', 'hotel', 'depot']

function Meter({ label, value }: { label: string; value: number }) {
  return (
    <div className="touring-meter" data-tone={value >= 75 ? 'good' : value >= 50 ? 'fair' : 'poor'}>
      <span>{label}</span>
      <b>{Math.round(value)}</b>
      <i style={{ width: `${Math.max(0, Math.min(100, value))}%` }} />
    </div>
  )
}

function ActiveTrip({ active, t }: { active: TouringActive; t: TouringWords }) {
  const [cancelling, setCancelling] = useState(false)
  const next = active.trip.stops[active.next]
  const trip = active.trip
  return (
    <>
      <Group title={t.active} place={4}>
        <Row
          title={`${t.kinds[trip.kind]} · ${trip.title}`}
          detail={`${t.phases[active.phase]} · ${t.day} ${next?.day ?? trip.days}/${trip.days}`}
          value={`${active.onBoard}/${trip.booked}`}
        />
        {next && (
          <Row
            title={`${t.next}: ${next.point.name}`}
            detail={`${t.stopKinds[next.kind]} · ${gameWhen(next.at, t.days)}${
              active.distance !== null ? ` · ${Math.round(active.distance)} km` : ''
            }`}
            value={lateWords(t, active.late)}
          />
        )}
        <Row title={t.toCollect} value={String(active.toCollect)} />
        <Row
          title={t.meter}
          value={`${Math.round(active.meter.km)} km · ${Math.round(active.meter.litres)} l${
            active.meter.ferries ? ` · ${active.meter.ferries}×⛴` : ''
          }`}
        />
      </Group>
      <div className="touring-mood ob-in">
        <Meter label={t.mood} value={active.mood.total} />
        <Meter label={t.moodParts.punctuality} value={active.mood.punctuality} />
        <Meter label={t.moodParts.comfort} value={active.mood.comfort} />
        <Meter label={t.moodParts.breaks} value={active.mood.breaks} />
      </div>
      <Group title={t.stops} place={5}>
        {trip.stops.map((stop, i) => (
          <Row
            key={i}
            title={stop.point.name}
            detail={`${t.stopKinds[stop.kind]} · ${gameWhen(stop.at, t.days)}${
              stop.passengers && (stop.kind === 'pickup' || stop.kind === 'dropoff') ? ` · ${stop.passengers}` : ''
            }`}
            value={stop.missed ? t.missed : stop.reached !== null ? t.reached : i === active.next ? '→' : ''}
          />
        ))}
      </Group>
      <Group place={6}>
        <Row title={t.skip} onClick={() => act({ action: 'skip' })} />
        <Row
          danger
          wrap
          title={cancelling ? t.cancelConfirm : t.cancel}
          onClick={() => {
            if (!cancelling) {
              setCancelling(true)
              return
            }
            setCancelling(false)
            act({ action: 'cancel' })
          }}
        />
      </Group>
    </>
  )
}

function TripRow({ trip, t, busy }: { trip: TouringTrip; t: TouringWords; busy: boolean }) {
  const contract = trip.kind === 'school' || trip.kind === 'charter'
  const days = trip.days === 1 ? `1 ${t.day}` : t.daysN.replace('{n}', String(trip.days))
  const pickups = trip.stops.filter((stop) => stop.kind === 'pickup').length
  return (
    <Row
      title={`${t.kinds[trip.kind]} · ${trip.title}`}
      detail={`${gameWhen(trip.departure, t.days)} · ${days} · ${Math.round(trip.distance)} km · ${pickups}× ${t.stopKinds.pickup.toLowerCase()} · ${trip.booked}/${trip.seats} ${t.booked}`}
      value={`${euro(trip.price)} ${contract ? t.contract : t.perSeat}`}
      accessory={
        <Button small disabled={busy} onClick={() => act({ action: 'start', id: trip.id })}>
          {t.start}
        </Button>
      }
    />
  )
}

function Places({ touring, t }: { touring: TouringStatus; t: TouringWords }) {
  const [kind, setKind] = useState<TouringStopKind>('pickup')
  const [name, setName] = useState('')
  return (
    <Group title={t.points} place={7}>
      <Row title={t.pointsDetail} wrap />
      <Row
        title={t.record}
        accessory={
          <span className="touring-record">
            <select value={kind} onChange={(event) => setKind(event.target.value as TouringStopKind)}>
              {RECORDABLE.map((k) => (
                <option key={k} value={k}>
                  {t.stopKinds[k]}
                </option>
              ))}
            </select>
            <input
              className="tacho-input"
              value={name}
              placeholder={t.pointName}
              maxLength={40}
              onChange={(event) => setName(event.target.value)}
            />
            <Button
              small
              disabled={touring.now === null}
              onClick={() => {
                act({ action: 'record', kind, name })
                setName('')
              }}
            >
              +
            </Button>
          </span>
        }
      />
      {touring.recorded.map((recorded, index) => (
        <Row
          key={index}
          title={recorded.point.name}
          detail={t.stopKinds[recorded.kind]}
          accessory={
            <Button small onClick={() => act({ action: 'forget', index })}>
              {t.forget}
            </Button>
          }
        />
      ))}
    </Group>
  )
}

/** The Touringcar page: the company (its name, home, balance and reputation), the trip driven, the catalogue,
 * the player's own places, the history, the settings. On the room's screen, the map. */
export function Touringcar({ cw, status, settings: { settings, update } }: PageProps) {
  const t = touringWords(cw)
  useScreenAt('maps', GLOW.maps, null)
  const [resetting, setResetting] = useState(false)
  const touring = status?.touring ?? null
  const chosen: TouringSettings = { ...TOURING_DEFAULTS, ...settings?.touring }
  const change = (next: Partial<TouringSettings>) => {
    if (settings) void update({ ...settings, touring: { ...chosen, ...next } })
  }

  return (
    <Panel place="page" title={touring?.company || t.title} sub={t.sub}>
      <div className="cp-columns touring-page">
        <div>
          {touring ? (
            <>
              <div className="touring-books ob-in">
                <div>
                  <span>{t.balance}</span>
                  <b data-negative={touring.balance < 0 || undefined}>{euro(touring.balance)}</b>
                </div>
                <div>
                  <span>{t.reputation}</span>
                  <b>{Math.round(touring.reputation)}/100</b>
                </div>
                <div>
                  <span>{t.depot}</span>
                  <b>{touring.depot ?? '—'}</b>
                </div>
              </div>
              {touring.now === null && <Note tone="calm">{t.noGame}</Note>}
              {!touring.mapReady && <Note tone="calm">{t.noMap}</Note>}
              {touring.active ? <ActiveTrip active={touring.active} t={t} /> : <Note tone="calm">{t.noActive}</Note>}
              <Group title={t.catalogue} place={5}>
                {touring.catalogue.length === 0 ? (
                  <Row title={t.catalogueEmpty} />
                ) : (
                  touring.catalogue.map((trip) => (
                    <TripRow key={trip.id} trip={trip} t={t} busy={touring.active !== null} />
                  ))
                )}
                <Row title={t.refresh} onClick={() => act({ action: 'refresh' })} />
              </Group>
            </>
          ) : (
            <Note tone="calm">{chosen.enabled ? t.noGame : t.off}</Note>
          )}
        </div>
        <div>
          {settings && (
            <Group title={t.company} place={4}>
              <Row
                title={t.enabled}
                detail={t.enabledDetail}
                accessory={<Switch on={chosen.enabled} label={t.enabled} change={(enabled) => change({ enabled })} />}
              />
              <Row
                title={t.company}
                accessory={
                  <input
                    className="tacho-input"
                    value={chosen.company}
                    placeholder={t.companyPlaceholder}
                    maxLength={40}
                    onChange={(event) => change({ company: event.target.value })}
                  />
                }
              />
              <Row
                title={t.home}
                detail={t.homeDetail}
                accessory={
                  <input
                    className="tacho-input"
                    value={chosen.home}
                    placeholder={t.homePlaceholder}
                    maxLength={40}
                    onChange={(event) => change({ home: event.target.value.trim().toLowerCase() })}
                  />
                }
              />
              <Row
                title={t.seats}
                accessory={
                  <input
                    className="tacho-input touring-seats"
                    type="number"
                    min={8}
                    max={90}
                    value={chosen.seats}
                    onChange={(event) => change({ seats: Math.max(8, Math.min(90, Number(event.target.value) || 50)) })}
                  />
                }
              />
              <Row
                title={t.navigate}
                detail={t.navigateDetail}
                accessory={
                  <Switch on={chosen.navigate} label={t.navigate} change={(navigate) => change({ navigate })} />
                }
              />
              <Row
                title={t.announcements}
                detail={t.announcementsDetail}
                accessory={
                  <Switch
                    on={chosen.announcements}
                    label={t.announcements}
                    change={(announcements) => change({ announcements })}
                  />
                }
              />
            </Group>
          )}
          {touring && <Places touring={touring} t={t} />}
          {touring && (
            <Group title={t.history} place={8}>
              {touring.history.length === 0 ? (
                <Row title={t.historyEmpty} />
              ) : (
                touring.history.map((trip, i) => (
                  <Row
                    key={i}
                    wrap
                    title={`${t.kinds[trip.kind]} · ${trip.destination}${trip.cancelled ? ` (${t.cancelled})` : ''}`}
                    detail={`${gameWhen(trip.departure, t.days)} · ${trip.passengers} p · ${t.mood} ${Math.round(trip.mood)} · ${t.costs
                      .replace('{fuel}', String(Math.round(trip.costs.fuel)))
                      .replace('{tolls}', String(Math.round(trip.costs.tolls)))
                      .replace('{ferries}', String(Math.round(trip.costs.ferries)))
                      .replace('{hotel}', String(Math.round(trip.costs.hotel)))
                      .replace('{wage}', String(Math.round(trip.costs.wage)))}`}
                    value={`${t.profit} ${euro(trip.profit)}`}
                  />
                ))
              )}
            </Group>
          )}
          {touring && (
            <Group place={9}>
              <Row
                danger
                wrap
                title={resetting ? t.resetConfirm : t.reset}
                onClick={() => {
                  if (!resetting) {
                    setResetting(true)
                    return
                  }
                  setResetting(false)
                  act({ action: 'reset' })
                }}
              />
            </Group>
          )}
        </div>
      </div>
    </Panel>
  )
}
