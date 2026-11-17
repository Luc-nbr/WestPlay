import { afterEach, describe, expect, it, vi } from 'vitest'
import type { PageMessage } from '../../shared/host'

function setUp(items: Record<string, string>) {
  const kept = new Map(Object.entries(items))
  const posted: PageMessage[] = []
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => kept.get(key) ?? null,
    setItem: (key: string, value: string) => void kept.set(key, value),
    removeItem: (key: string) => void kept.delete(key),
  })
  vi.stubGlobal('window', { chrome: { webview: { postMessage: (message: PageMessage) => posted.push(message) } } })
  return { kept, posted }
}

afterEach(() => {
  vi.unstubAllGlobals()
  vi.resetModules()
})

describe('sharedStorage', () => {
  it('hands WestPlay what this page kept when it keeps nothing yet, without the screen’s own', async () => {
    const { posted } = setUp({
      'westplay.recents': 'R',
      'westplay.map.view': 'V',
      'westplay.cabin': JSON.stringify({ state: { textSize: 2, brightness: 0.4 }, version: 3 }),
    })
    const { localFields, receivePageState } = await import('./sharedStorage')
    localFields('westplay.cabin', ['brightness'])
    receivePageState({ type: 'pageState', values: null })
    expect(posted).toEqual([
      {
        type: 'pageStateImport',
        values: {
          'westplay.cabin': JSON.stringify({ state: { textSize: 2 }, version: 3 }),
          'westplay.recents': 'R',
        },
      },
    ])
  })

  it('tells a change once, and keeps what isn’t shared to itself', async () => {
    const { posted } = setUp({})
    const { sharedStorage } = await import('./sharedStorage')
    sharedStorage.setItem('westplay.recents', 'A')
    sharedStorage.setItem('westplay.recents', 'A')
    sharedStorage.setItem('westplay.map.view', 'x')
    expect(posted).toEqual([{ type: 'pageState', key: 'westplay.recents', value: 'A' }])
  })

  it('takes WestPlay’s value with the screen’s own fields kept, and loads the store again', async () => {
    const { kept } = setUp({
      'westplay.cabin': JSON.stringify({ state: { textSize: 2, brightness: 0.4 }, version: 3 }),
    })
    const { localFields, onSharedChange, receivePageState } = await import('./sharedStorage')
    localFields('westplay.cabin', ['brightness'])
    const rehydrate = vi.fn()
    onSharedChange('westplay.cabin', rehydrate)
    const value = JSON.stringify({ state: { textSize: 5 }, version: 3 })
    receivePageState({ type: 'pageStateChanged', key: 'westplay.cabin', value })
    expect(JSON.parse(kept.get('westplay.cabin')!)).toEqual({ state: { textSize: 5, brightness: 0.4 }, version: 3 })
    expect(rehydrate).toHaveBeenCalledOnce()
    receivePageState({ type: 'pageStateChanged', key: 'westplay.cabin', value })
    expect(rehydrate).toHaveBeenCalledOnce()
  })
})
