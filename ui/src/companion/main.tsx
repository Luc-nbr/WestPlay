import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { call } from './bridge'
import { Companion } from './Companion'

// No right-click menu, as in an app (the browser's Back, Reload and Inspect have no place here); a text field
// keeps its own, for copy and paste.
document.addEventListener('contextmenu', (event) => {
  const target = event.target instanceof Element ? event.target : null
  if (!target?.closest('input, textarea, [contenteditable="true"]')) event.preventDefault()
})

// F11: the window full screen, or back.
document.addEventListener('keydown', (event) => {
  if (event.key !== 'F11') return
  event.preventDefault()
  void call<boolean>('toggle_fullscreen').catch(() => {})
})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Companion />
  </StrictMode>,
)
