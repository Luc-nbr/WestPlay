import { type PointerEvent, useRef, useState } from 'react'

/** The tone the curve shapes: bass, mid and treble (dB, ±6) and the bass boost (0 to 100: up to 18 dB under
 * 60 Hz, and a sub peak of 8 dB at 45 Hz). */
export interface Tone {
  bass: number
  mid: number
  treble: number
  boost: number
}

type Band = keyof Tone

/** Each handle: where it sits on the curve (Hz), its range, and how a dB of the graph becomes its value. */
const HANDLES: { band: Band; at: number; min: number; max: number; step: number }[] = [
  { band: 'boost', at: 45, min: 0, max: 100, step: 5 },
  { band: 'bass', at: 100, min: -6, max: 6, step: 1 },
  { band: 'mid', at: 1000, min: -6, max: 6, step: 1 },
  { band: 'treble', at: 8000, min: -6, max: 6, step: 1 },
]

const WIDTH = 340
const HEIGHT = 230
const PAD = { left: 30, right: 10, top: 22, bottom: 26 }
const LOW = 20
const HIGH = 20000
/** The graph's dB, either way. */
const RANGE = 27
const RATE = 48000

/** A band's dB as the graph draws it: the boost's 0 to 100 is 0 to 12 dB. */
function dbOf(band: Band, value: number): number {
  return band === 'boost' ? (value / 100) * 18 : value
}

function valueOf(band: Band, db: number): number {
  return band === 'boost' ? (db / 26) * 100 : db
}

/** A biquad's gain (dB) at `f` (RBJ's cookbook, as Web Audio's filters: shelves at slope 1). */
function response(kind: 'lowshelf' | 'peaking' | 'highshelf', f0: number, gain: number, q: number, f: number): number {
  if (gain === 0) return 0
  const a = 10 ** (gain / 40)
  const w0 = (2 * Math.PI * f0) / RATE
  const cos = Math.cos(w0)
  const sin = Math.sin(w0)
  let b0: number
  let b1: number
  let b2: number
  let a0: number
  let a1: number
  let a2: number
  if (kind === 'peaking') {
    const alpha = sin / (2 * q)
    ;[b0, b1, b2, a0, a1, a2] = [1 + alpha * a, -2 * cos, 1 - alpha * a, 1 + alpha / a, -2 * cos, 1 - alpha / a]
  } else {
    const alpha = (sin / 2) * Math.SQRT2
    const root = 2 * Math.sqrt(a) * alpha
    if (kind === 'lowshelf') {
      ;[b0, b1, b2] = [
        a * (a + 1 - (a - 1) * cos + root),
        2 * a * (a - 1 - (a + 1) * cos),
        a * (a + 1 - (a - 1) * cos - root),
      ]
      ;[a0, a1, a2] = [a + 1 + (a - 1) * cos + root, -2 * (a - 1 + (a + 1) * cos), a + 1 + (a - 1) * cos - root]
    } else {
      ;[b0, b1, b2] = [
        a * (a + 1 + (a - 1) * cos + root),
        -2 * a * (a - 1 + (a + 1) * cos),
        a * (a + 1 + (a - 1) * cos - root),
      ]
      ;[a0, a1, a2] = [a + 1 - (a - 1) * cos + root, 2 * (a - 1 - (a + 1) * cos), a + 1 - (a - 1) * cos - root]
    }
  }
  const w = (2 * Math.PI * f) / RATE
  const re = (c0: number, c1: number, c2: number) => c0 + c1 * Math.cos(w) + c2 * Math.cos(2 * w)
  const im = (c1: number, c2: number) => -(c1 * Math.sin(w) + c2 * Math.sin(2 * w))
  const num = Math.hypot(re(b0, b1, b2), im(b1, b2))
  const den = Math.hypot(re(a0, a1, a2), im(a1, a2))
  return 20 * Math.log10(num / den)
}

/** The whole curve's gain at `f`, as the speakers' sound chain has it (cab_sound.js). */
function curveAt(tone: Tone, f: number): number {
  return (
    response('lowshelf', 100, tone.bass, 0, f) +
    response('peaking', 1000, tone.mid, 0.7, f) +
    response('highshelf', 8000, tone.treble, 0, f) +
    response('lowshelf', 60, dbOf('boost', tone.boost), 0, f) +
    response('peaking', 45, (tone.boost / 100) * 8, 1.1, f)
  )
}

const x = (f: number) =>
  PAD.left + ((Math.log10(f) - Math.log10(LOW)) / (Math.log10(HIGH) - Math.log10(LOW))) * (WIDTH - PAD.left - PAD.right)
const y = (db: number) => PAD.top + ((RANGE - db) / (2 * RANGE)) * (HEIGHT - PAD.top - PAD.bottom)

/**
 * The music's tone as a real equalizer's curve, in Dashly's look: its handles dragged up or down (bass, mid, treble,
 * and the bass boost under them), the curve the sound chain makes drawn live as they move. `change` as it's dragged
 * (the sound follows), `commit` once let go.
 */
export function EqCurve({
  tone,
  names,
  change,
  commit,
}: {
  tone: Tone
  names: Record<Band, string>
  change(tone: Tone): void
  commit(tone: Tone): void
}) {
  const svg = useRef<SVGSVGElement>(null)
  const [dragging, setDragging] = useState<Band | null>(null)
  const latest = useRef(tone)
  latest.current = tone

  const points: string[] = []
  for (let i = 0; i <= 120; i++) {
    const f = LOW * (HIGH / LOW) ** (i / 120)
    points.push(`${x(f).toFixed(1)},${y(curveAt(tone, f)).toFixed(1)}`)
  }
  const line = `M${points.join(' L')}`
  const area = `${line} L${x(HIGH)},${y(-RANGE)} L${x(LOW)},${y(-RANGE)} Z`

  const at = (event: PointerEvent, band: Band) => {
    const box = svg.current?.getBoundingClientRect()
    if (!box) return latest.current
    const py = ((event.clientY - box.top) / box.height) * HEIGHT
    const db = RANGE - ((py - PAD.top) / (HEIGHT - PAD.top - PAD.bottom)) * 2 * RANGE
    const handle = HANDLES.find((h) => h.band === band)!
    const value = Math.round(valueOf(band, db) / handle.step) * handle.step
    return { ...latest.current, [band]: Math.max(handle.min, Math.min(handle.max, value)) }
  }

  return (
    <svg
      ref={svg}
      className="dashly-eq"
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      role="group"
      aria-label="Equalizer"
      onPointerMove={(event) => {
        if (!dragging) return
        change(at(event, dragging))
      }}
      onPointerUp={(event) => {
        if (!dragging) return
        commit(at(event, dragging))
        setDragging(null)
      }}
      onPointerCancel={() => setDragging(null)}
    >
      <defs>
        <linearGradient id="dashly-eq-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#3b6bff" stopOpacity="0.55" />
          <stop offset="1" stopColor="#3b6bff" stopOpacity="0" />
        </linearGradient>
        <linearGradient id="dashly-eq-line" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#ff9f0a" />
          <stop offset="0.25" stopColor="#3b6bff" />
          <stop offset="1" stopColor="#5ac8fa" />
        </linearGradient>
        <filter id="dashly-eq-glow" x="-10%" y="-30%" width="120%" height="160%">
          <feGaussianBlur stdDeviation="4" />
        </filter>
      </defs>
      {/* The grid: every 5 dB, and the decades and their middles. */}
      {[-20, -10, 0, 10, 20].map((db) => (
        <g key={db}>
          <line
            className={db === 0 ? 'dashly-eq-zero' : 'dashly-eq-grid'}
            x1={PAD.left}
            x2={WIDTH - PAD.right}
            y1={y(db)}
            y2={y(db)}
          />
          {db % 10 === 0 && (
            <text className="dashly-eq-label" x={PAD.left - 6} y={y(db) + 4} textAnchor="end">
              {db > 0 ? `+${db}` : db}
            </text>
          )}
        </g>
      ))}
      {[20, 100, 1000, 10000].map((f) => (
        <g key={f}>
          <line className="dashly-eq-grid" x1={x(f)} x2={x(f)} y1={PAD.top} y2={HEIGHT - PAD.bottom} />
          <text className="dashly-eq-label" x={x(f)} y={HEIGHT - 8} textAnchor="middle">
            {f >= 1000 ? `${f / 1000}k` : f}
          </text>
        </g>
      ))}
      <path d={area} fill="url(#dashly-eq-fill)" />
      <path d={line} className="dashly-eq-shine" stroke="url(#dashly-eq-line)" filter="url(#dashly-eq-glow)" />
      <path d={line} className="dashly-eq-line" stroke="url(#dashly-eq-line)" />
      {HANDLES.map((handle) => {
        const cx = x(handle.at)
        const cy = y(curveAt(tone, handle.at))
        return (
          <g
            key={handle.band}
            className="dashly-eq-handle"
            data-band={handle.band}
            data-dragging={dragging === handle.band || undefined}
            onPointerDown={(event) => {
              event.preventDefault()
              ;(event.currentTarget.ownerSVGElement ?? svg.current)?.setPointerCapture(event.pointerId)
              setDragging(handle.band)
            }}
          >
            <circle className="dashly-eq-halo" cx={cx} cy={cy} r={16} />
            <circle className="dashly-eq-dot" cx={cx} cy={cy} r={7} />
            <text className="dashly-eq-name" x={cx} y={cy - 16 < PAD.top + 8 ? cy + 24 : cy - 16} textAnchor="middle">
              {names[handle.band]}
            </text>
          </g>
        )
      })}
    </svg>
  )
}
