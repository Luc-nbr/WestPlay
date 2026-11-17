import { type PointerEvent as ReactPointerEvent, type ReactNode, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { call } from '../bridge'
import type { TachoStatus } from '../useHostStatus'
import type { Settings } from '../useSettings'
import { ActivityIcon, CardIcon, ClockIcon, DriverIcon } from './TachoIcons'
import { cardNumber, clockTime, driverName, hours, receipt, type ReceiptLine } from './tachoReceipt'
import type { TachoWords } from './tachoWords'
import './tachoDevice.css'

/** The display's screens, ▲ ▼ through them: the standard one, driving and break, day and week, two weeks and the
 * daily rest, violations. */
const SCREENS = 5

/** How long the printer takes to print a receipt, ms. */
const PRINTING_MS = 2600

/**
 * The tachograph as a real one is built (a 1-DIN unit, as every EU digital tachograph): a two-line dot-matrix display,
 * four buttons, two card slots with their eject buttons, and a thermal printer. The driver's card is dragged into
 * slot 1 (or clicked), and ejected with button 1; OK on the standard screen prints the day's receipt, which comes
 * out of the printer and goes to the player's own (thermal) printer with Print. No maker's name or look.
 */
export function TachoDevice({
  tacho,
  settings,
  update,
  t,
}: {
  tacho: TachoStatus
  settings: Settings
  update(settings: Settings): void
  t: TachoWords
}) {
  const chosen = settings.tacho
  const [screen, setScreen] = useState(0)
  const [menu, setMenu] = useState(false)
  const [printing, setPrinting] = useState(false)
  const [paper, setPaper] = useState<ReceiptLine[] | null>(null)
  // The card in its slot is the setting's; what moves it, here.
  const [motion, setMotion] = useState<'none' | 'dragging' | 'going-in' | 'coming-out'>('none')
  const card = chosen.card && motion === 'none' ? 'in' : motion === 'none' ? 'tray' : motion
  const [drag, setDrag] = useState({ x: 0, y: 0 })
  const [over, setOver] = useState(false)
  const start = useRef<{ x: number; y: number; moved: boolean } | null>(null)
  const slot = useRef<HTMLDivElement>(null)
  const cardEl = useRef<HTMLDivElement>(null)
  const [to, setTo] = useState({ x: 0, y: 0 })

  const setCardIn = (inside: boolean) => update({ ...settings, tacho: { ...chosen, card: inside } })

  /** Where the slot is from the card, for it to fly there. */
  const towardsSlot = () => {
    const a = cardEl.current?.getBoundingClientRect()
    const b = slot.current?.getBoundingClientRect()
    if (!a || !b) return { x: 0, y: 0 }
    return { x: b.left + b.width / 2 - (a.left + a.width / 2), y: b.top + b.height / 2 - (a.top + a.height / 2) }
  }

  const insert = () => {
    setTo(towardsSlot())
    setMotion('going-in')
    setOver(false)
    window.setTimeout(() => {
      setCardIn(true)
      setMotion('none')
      setDrag({ x: 0, y: 0 })
    }, 520)
  }

  const eject = () => {
    if (card !== 'in') return
    setMenu(false)
    setCardIn(false)
    setMotion('coming-out')
    window.setTimeout(() => setMotion('none'), 560)
  }

  const overSlot = (event: ReactPointerEvent) => {
    const b = slot.current?.getBoundingClientRect()
    if (!b) return false
    const pad = 40
    return (
      event.clientX > b.left - pad &&
      event.clientX < b.right + pad &&
      event.clientY > b.top - pad &&
      event.clientY < b.bottom + pad
    )
  }

  const print = () => {
    if (printing) return
    setMenu(false)
    setPaper(receipt(tacho, chosen.driver, chosen.card, t))
    setPrinting(true)
    window.setTimeout(() => setPrinting(false), PRINTING_MS)
  }

  const press = (button: 'back' | 'up' | 'down' | 'ok') => {
    if (button === 'back') {
      setMenu(false)
      setScreen(0)
    } else if (button === 'up') setScreen((screen + SCREENS - 1) % SCREENS)
    else if (button === 'down') setScreen((screen + 1) % SCREENS)
    else if (menu) print()
    else setMenu(true)
  }

  const [top, bottom] = printing ? [t.lcdPrinting, ''] : menu ? [t.lcdPrint, t.lcdPrintOk] : ['', '']
  const s = tacho.summary
  const lines: [ReactNode, ReactNode] = printing || menu ? [top, bottom] : lcdScreen(screen, tacho, chosen.card, t)

  return (
    <div className="td-wrap">
      <div className="td-unit">
        <div className="td-face">
          <div className="td-lcd" data-warn={s.nextLeft <= 15 || undefined}>
            <div className="td-dots">
              <div className="td-line">{lines[0]}</div>
              <div className="td-line">{lines[1]}</div>
            </div>
          </div>
          <div className="td-printer-key" aria-hidden />
          <div className="td-printer" data-printing={printing || undefined}>
            <i />
          </div>
          <div className="td-buttons">
            <button type="button" aria-label="↵" onClick={() => press('back')}>
              ↵
            </button>
            <button type="button" aria-label="▲" onClick={() => press('up')}>
              ▲
            </button>
            <button type="button" aria-label="▼" onClick={() => press('down')}>
              ▼
            </button>
            <button type="button" aria-label="OK" onClick={() => press('ok')}>
              OK
            </button>
          </div>
          <div
            ref={slot}
            className="td-slot td-slot-1"
            data-over={over || undefined}
            data-in={card === 'in' || undefined}
          >
            <i />
          </div>
          <button type="button" className="td-eject td-eject-1" aria-label="Eject 1" onClick={eject}>
            <DriverIcon n={1} />
          </button>
          <button type="button" className="td-eject td-eject-2" aria-label="Eject 2">
            <DriverIcon n={2} />
          </button>
          <div className="td-slot td-slot-2">
            <i />
          </div>
          <span className="td-screw td-screw-l" />
          <span className="td-screw td-screw-r" />
        </div>
        {paper && (
          <div className="td-paper-out" data-paper={chosen.paper} data-printing={printing || undefined}>
            <Receipt lines={paper} />
            {!printing && (
              <div className="td-paper-actions">
                <button type="button" className="td-action" onClick={() => printOnPaper(chosen.paper)}>
                  {t.receiptPrint}
                </button>
                <button type="button" className="td-action" data-plain onClick={() => setPaper(null)}>
                  {t.receiptTear}
                </button>
              </div>
            )}
          </div>
        )}
      </div>

      <div className="td-tray">
        {card !== 'in' ? (
          <div
            ref={cardEl}
            className="td-card"
            data-state={card}
            style={
              card === 'dragging'
                ? { transform: `translate(${drag.x}px, ${drag.y}px) rotate(-4deg) scale(1.04)` }
                : card === 'going-in'
                  ? { transform: `translate(${to.x}px, ${to.y}px) rotateX(78deg) scale(0.42)` }
                  : undefined
            }
            onPointerDown={(event) => {
              if (card !== 'tray') return
              event.currentTarget.setPointerCapture(event.pointerId)
              start.current = { x: event.clientX, y: event.clientY, moved: false }
            }}
            onPointerMove={(event) => {
              const from = start.current
              if (!from) return
              const dx = event.clientX - from.x
              const dy = event.clientY - from.y
              if (!from.moved && Math.hypot(dx, dy) < 4) return
              from.moved = true
              setMotion('dragging')
              setDrag({ x: dx, y: dy })
              setOver(overSlot(event))
            }}
            onPointerUp={(event) => {
              const from = start.current
              start.current = null
              if (!from) return
              if (!from.moved || overSlot(event)) insert()
              else {
                setMotion('none')
                setDrag({ x: 0, y: 0 })
                setOver(false)
              }
            }}
          >
            <DriverCard
              name={driverName(chosen.driver, tacho, t)}
              company={tacho.profile?.company ?? ''}
              number={tacho.profile?.cardNumber ?? null}
              t={t}
            />
          </div>
        ) : (
          <div className="td-card-slotted" aria-hidden />
        )}
        <p className="td-hint">
          {card === 'in' ? t.cardIn : t.cardHint}
          <br />
          <span>{t.buttonsHint}</span>
        </p>
      </div>
      {paper && createPortal(<Receipt lines={paper} print />, document.body)}
    </div>
  )
}

/** What the display shows on each screen: two lines. */
function lcdScreen(screen: number, tacho: TachoStatus, card: boolean, t: TachoWords): [ReactNode, ReactNode] {
  const s = tacho.summary
  switch (screen) {
    case 1:
      return [
        <>
          <ActivityIcon activity="driving" />
          <span>{t.lcdLeft}</span>
          <b>{hours(s.nextLeft)}</b>
        </>,
        <>
          <ActivityIcon activity="rest" />
          <span>{t.lcdBreak}</span>
          <b>
            {hours(s.breakTaken)}/{hours(s.breakNeeded)}
          </b>
        </>,
      ]
    case 2:
      return [
        <>
          <span>Σ {t.lcdDay}</span>
          <b>
            {hours(s.dayDriving)}/{Math.round(s.dayLimit / 60)}h
          </b>
        </>,
        <>
          <span>Σ {t.lcdWeek}</span>
          <b>{hours(s.weekDriving)}/56h</b>
        </>,
      ]
    case 3:
      return [
        <>
          <span>Σ {t.lcdFortnight}</span>
          <b>{hours(s.fortnightDriving)}/90h</b>
        </>,
        <>
          <ActivityIcon activity="rest" />
          <span>{t.lcdRestIn}</span>
          <b>{hours(s.restDueIn)}</b>
        </>,
      ]
    case 4: {
      const violations = tacho.log.filter((entry) => entry.type === 'violation')
      const last = violations[0]
      return [
        <>
          <span>! {t.lcdViolations}</span>
          <b>{violations.length || t.lcdNone}</b>
        </>,
        <>{last && last.type === 'violation' && <span className="td-small">{t.violations[last.kind]}</span>}</>,
      ]
    }
    default:
      return [
        <>
          <b>{clockTime(tacho)}</b>
          <ClockIcon />
          <b className="td-right">{Math.round(tacho.speed)}km/h</b>
        </>,
        <>
          <ActivityIcon activity={s.activity} />
          <CardIcon in={card} />
          <b className="td-right">{tacho.distance.toFixed(1)}km</b>
          <ActivityIcon activity={card ? 'rest' : null} />
        </>,
      ]
  }
}

/** The driver's card: a generic EU-style driver card, with the driver's name (the game profile's, unless typed),
 * their company and the profile's own card number. */
function DriverCard({
  name,
  company,
  number,
  t,
}: {
  name: string
  company: string
  number: string | null
  t: TachoWords
}) {
  const shown = name.trim() || t.receiptDriver
  return (
    <div className="td-card-face">
      <div className="td-card-top">
        <span className="td-card-country">NL</span>
        <b>{t.cardTitle}</b>
      </div>
      <div className="td-card-chip" />
      <div className="td-card-lines">
        <span>1. {shown}</span>
        {company && <span>8. {company}</span>}
        <span>5b. {number ?? cardNumber(shown)}</span>
      </div>
      <div className="td-card-photo" />
    </div>
  )
}

/** The printout: on the paper coming out of the printer, or (`print`) as it goes to the player's printer. */
function Receipt({ lines, print }: { lines: ReceiptLine[]; print?: boolean }) {
  return (
    <div className={print ? 'td-receipt td-receipt-print' : 'td-receipt'}>
      {lines.map((line, i) => {
        switch (line.kind) {
          case 'rule':
            return <hr key={i} />
          case 'title':
            return (
              <p key={i} className="td-r-title">
                {line.text}
              </p>
            )
          case 'activity':
            return (
              <p key={i} className="td-r-activity">
                <ActivityIcon activity={line.activity} />
                <span>{line.from}</span>
                <b>{line.length}</b>
              </p>
            )
          case 'signature':
            return (
              <div key={i} className="td-r-signature">
                <p>{line.text}</p>
                <i />
              </div>
            )
          default:
            return (
              <p key={i} className="td-r-text" data-strong={line.strong || undefined}>
                <span>{line.text}</span>
                {line.right && <b>{line.right}</b>}
              </p>
            )
        }
      })}
    </div>
  )
}

/** The receipt to the player's printer: the page printed shows only it (tachoDevice.css), on paper as wide as the
 * printer's. Windows' print dialog picks the printer. */
function printOnPaper(paper: number) {
  const style = document.createElement('style')
  style.textContent = `.td-receipt-print { width: ${paper}mm; }`
  document.head.append(style)
  // The paper as long as the receipt (a page's "auto" length isn't printed as such: it came out a page long).
  const receipt = document.querySelector<HTMLElement>('.td-receipt-print')
  let length = 200
  if (receipt) {
    receipt.classList.add('td-measure')
    length = Math.ceil((receipt.getBoundingClientRect().height * 25.4) / 96) + 4
    receipt.classList.remove('td-measure')
  }
  style.textContent += ` @page { size: ${paper}mm ${length}mm; margin: 0 }`
  document.documentElement.classList.add('td-printing')
  const done = () => {
    style.remove()
    document.documentElement.classList.remove('td-printing')
    window.removeEventListener('afterprint', done)
  }
  window.addEventListener('afterprint', done)
  window.setTimeout(done, 120_000)
  // The app's own print (WebView2's), the browser's in a preview.
  call('print_receipt').catch(() => window.print())
}
