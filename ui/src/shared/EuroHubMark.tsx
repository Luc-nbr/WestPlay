import type { CSSProperties } from 'react'

/* Euro-Hub's mark, in Omsi-Hub's style (its sister product): the blue ring that runs out into a road, a truck's
   front inside it (public/euro-hub-mark.svg is the same on its navy tile). Drawn inline so the ring can draw
   itself in (dashly.css `.eh-ring`). */
const RING = 'M485.35 362.58 A167 167 0 1 0 334 459 L420 459'
const TRUCK =
  'M290 200H378L392 218H276ZM288 214H380A22 22 0 0 1 402 236V356A8 8 0 0 1 394 364H274A8 8 0 0 1 266 356V236A22 22 0 0 1 288 214ZM289 228H379A9 9 0 0 1 388 237V269A9 9 0 0 1 379 278H289A9 9 0 0 1 280 269V237A9 9 0 0 1 289 228ZM302 292H366A4 4 0 0 1 370 296V296A4 4 0 0 1 366 300H302A4 4 0 0 1 298 296V296A4 4 0 0 1 302 292ZM302 306H366A4 4 0 0 1 370 310V310A4 4 0 0 1 366 314H302A4 4 0 0 1 298 310V310A4 4 0 0 1 302 306ZM302 320H366A4 4 0 0 1 370 324V324A4 4 0 0 1 366 328H302A4 4 0 0 1 298 324V324A4 4 0 0 1 302 320ZM283 340H295A4 4 0 0 1 299 344V347A4 4 0 0 1 295 351H283A4 4 0 0 1 279 347V344A4 4 0 0 1 283 340ZM373 340H385A4 4 0 0 1 389 344V347A4 4 0 0 1 385 351H373A4 4 0 0 1 369 347V344A4 4 0 0 1 373 340Z'
const TRUCK_EXTRAS =
  'M257 236H265A3 3 0 0 1 268 239A3 3 0 0 1 265 242H257A3 3 0 0 1 254 239A3 3 0 0 1 257 236ZM247 228H249A7 7 0 0 1 256 235V265A7 7 0 0 1 249 272H247A7 7 0 0 1 240 265V235A7 7 0 0 1 247 228ZM403 236H411A3 3 0 0 1 414 239A3 3 0 0 1 411 242H403A3 3 0 0 1 400 239A3 3 0 0 1 403 236ZM419 228H421A7 7 0 0 1 428 235V265A7 7 0 0 1 421 272H419A7 7 0 0 1 412 265V235A7 7 0 0 1 419 228ZM276 360H302V380A6 6 0 0 1 296 386H282A6 6 0 0 1 276 380ZM366 360H392V380A6 6 0 0 1 386 386H372A6 6 0 0 1 366 380Z'

/** The mark alone: on its navy tile (`tile`), or bare on what's behind it. */
export function EuroHubMark({ size = 32, tile = false, style }: { size?: number; tile?: boolean; style?: CSSProperties }) {
  return (
    <svg
      className={tile ? 'eh-mark eh-mark-tile' : 'eh-mark'}
      width={size}
      height={size}
      viewBox={tile ? '54 42 560 560' : '144 102 380 380'}
      style={style}
      aria-hidden="true"
    >
      {tile && <rect x="54" y="42" width="560" height="560" rx="123.2" fill="#0a142e" />}
      <path className="eh-ring" d={RING} fill="none" stroke="#2a75f7" strokeWidth="46" strokeLinecap="round" />
      <path d={TRUCK} fill="var(--eh-logo-ink, #eef1f5)" fillRule="evenodd" />
      <path d={TRUCK_EXTRAS} fill="var(--eh-logo-ink, #eef1f5)" />
    </svg>
  )
}

/** The logo: the mark, then "Euro-" and an orange, slanted "Hub", as Omsi-Hub writes its name; `road`: the ring's
 * road running on under the name, as on Omsi-Hub's banner. */
export function EuroHubLogo({ height = 32, road = false }: { height?: number; road?: boolean }) {
  return (
    <span className="eh-logo" data-road={road || undefined} style={{ '--eh-h': `${height}px` } as CSSProperties}>
      <EuroHubMark size={height} />
      <span className="eh-word">
        Euro-<em>Hub</em>
      </span>
      {road && (
        <span className="eh-stops" aria-hidden>
          {/* A truck, a trailer and a coach on the road, as Omsi-Hub's signs on its line. */}
          <i data-kind="truck">
            <svg viewBox="0 0 24 24">
              <path d="M2 6h11v9H2zM13 9h4.5l3.5 3.5V15h-8z" />
              <circle cx="6" cy="17" r="2" />
              <circle cx="17" cy="17" r="2" />
            </svg>
          </i>
          <i data-kind="trailer">
            <svg viewBox="0 0 24 24">
              <path d="M1 6h17v9H1zM18 12h4v1.5h-4z" />
              <circle cx="5" cy="17" r="2" />
              <circle cx="10" cy="17" r="2" />
            </svg>
          </i>
          <i data-kind="coach">
            <svg viewBox="0 0 24 24">
              <path d="M3 5h15.5a2.5 2.5 0 0 1 2.5 2.5V15H3z" />
              <path d="M5 7.5h3.5v3H5zM10 7.5h3.5v3H10zM15 7.5h3.5v3H15z" fill="var(--eh-stop-bg)" />
              <circle cx="7" cy="16.5" r="2" />
              <circle cx="17" cy="16.5" r="2" />
            </svg>
          </i>
        </span>
      )}
    </span>
  )
}
