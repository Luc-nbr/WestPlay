# Gedeelde paginastand — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wat de cabin-pagina nu alleen in zijn eigen `localStorage` bewaart (instellingen, recente bestemmingen, Safari-tabs, YouTube/Spotify/YouTube Music-stand) wordt bewaard door de host, zodat elke pagina die later verbindt (de iPad) dezelfde stand heeft.

**Architecture:** De host krijgt een kleine sleutel→tekst-opslag (`page_state.json` in `data_dir`), stuurt die bij `ready` mee en stuurt elke wijziging door. De pagina krijgt een `sharedStorage` met de `Storage`-vorm die `localStorage` als snelle cache gebruikt en elke wijziging naar de host post; zustand-stores die erop draaien worden opnieuw gehydrateerd als de host andere waarden stuurt. Bestaande spelers verliezen niets: heeft de host nog niets, dan stuurt de pagina eenmalig zijn huidige waarden ("import").

**Tech Stack:** Rust (crates/host, serde_json), TypeScript/React + zustand `persist` (ui/src/cabin), vitest, `cargo test`.

**Spec:** `docs/superpowers/specs/2026-11-16-ipad-head-unit-design.md` (onderdeel 4, "Gedeelde stand in de host"). Dit is plan 1 van 4; daarna volgen: 2. LAN-server + koppeling + pagina in browsermodus op de iPad, 3. hoofdunit-modus (truck als vast tweede scherm, plugin geeft input terug), 4. spelers spiegelen naar de iPad.

## Global Constraints

- Gedeeld: instellingen, recente bestemmingen, Safari-tabs en -geschiedenis, bewaarde stand van YouTube, YouTube Music, Spotify (incl. hun volumes).
- Per scherm (niet gedeeld): wat over één scherm zelf gaat — helderheid, de open app/navigatie, de kaartweergave (`westplay.map.view`), caches (`launchCopies`), en alles van de onboarding-demo (`westplay.cabin.demo`).
- Overgang: bij de eerste start met deze versie stuurt de truckpagina zijn bestaande waarden eenmalig naar de host; niemand verliest iets.
- Werkt zonder iPad; niets aan het gedrag voor spelers zonder iPad mag merkbaar veranderen (anti-doel: truckscherm trager of minder betrouwbaar).
- Bestandsformaat volgt `settings.rs`: vergevingsgezind laden, onleesbaar bestand opzij als `.json.bad`, opslaan via `.json.partial` + rename.

## Review Focus

1. **Echo-lus:** een rehydratie roept zustand's `setItem` aan met dezelfde waarde → er mag niets naar de host gaan, en de host stuurt een ongewijzigde waarde niet rond (tests in Task 1 en Task 3).
2. **Kapot `page_state.json`:** opzij gezet en behandeld als "nog nooit geïmporteerd", zodat de truck zijn eigen waarden opnieuw importeert in plaats van leeg te beginnen (test in Task 1).
3. **Per-scherm velden overleven een gedeelde update:** komt `westplay.cabin` van de host, dan blijven `navigation` en `brightness` van dit scherm staan (test in Task 3).
4. **Import maar één keer:** een tweede import wanneer de host al waarden heeft wordt genegeerd — anders overschrijft een later verbonden iPad met een lege stand alles (test in Task 1).
5. **Vreemde of te grote sleutels/waarden** (de iPad wordt later een netwerkclient): sleutel moet met `westplay.` beginnen en ≤ 64 tekens zijn, waarde ≤ 1 MiB, totaal ≤ 8 MiB; anders genegeerd en gelogd (test in Task 1).

---

### Task 1: Host-opslag `page_state`

**Files:**
- Create: `crates/host/src/page_state.rs`
- Modify: `crates/host/src/lib.rs` (module-declaratie bij de andere `mod`-regels, ~r. 7–44)
- Test: in `crates/host/src/page_state.rs` (`#[cfg(test)] mod tests`, zoals `settings.rs:499`)

**Interfaces:**
- Produces:
  - `pub struct PageState` — `fn load(dir: &Path) -> PageState` (leest `dir/page_state.json`)
  - `fn all_message(&self) -> String` → `{"type":"pageState","values":{…}}` of `{"type":"pageState","values":null}` als er nooit iets is geïmporteerd
  - `fn set(&mut self, key: &str, value: Option<&str>) -> Option<String>` → bij een echte wijziging opslaan en `{"type":"pageStateChanged","key":…,"value":…|null}` teruggeven; anders `None`
  - `fn import(&mut self, values: BTreeMap<String, String>) -> Option<String>` → alleen als er nog nooit is geïmporteerd: geldige paren bewaren, opslaan, `all_message()` teruggeven; anders `None`
  - Constanten: `KEY_PREFIX = "westplay."`, `KEY_MOST = 64`, `VALUE_MOST = 1 << 20`, `TOTAL_MOST = 8 << 20`

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn nothing_saved_says_values_null() // load(tempdir).all_message() == r#"{"type":"pageState","values":null}"#
#[test] fn import_once_then_ignored() // import({"westplay.recents":"[1]"}) -> Some(all with that); second import -> None; value unchanged
#[test] fn set_saves_and_reports_changes_only() // set("westplay.recents", Some("a")) -> Some(changed msg); same again -> None; load(dir) sees "a"
#[test] fn set_none_removes() // set(k, None) after a value -> Some(msg with "value":null); key gone after reload
#[test] fn refuses_foreign_or_oversized() // key "other", key of 65 chars, value of 1 MiB + 1, total over 8 MiB -> None, nothing stored
#[test] fn unreadable_file_is_set_aside_and_counts_as_never_imported() // write "{not json" -> load: values null, page_state.json.bad exists; import then works
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p westplay-host page_state`
Expected: FAIL (module/functions not defined)

- [ ] **Step 3: Implement `PageState` in `crates/host/src/page_state.rs`**

Bestand als `{"version":1,"values":{…}}`; ontbrekend bestand = nooit geïmporteerd (`values: None`). Laden/opslaan zoals `SettingsFile` (`settings.rs:443-469`): `.json.bad` bij onleesbaar, `.json.partial` + rename bij opslaan, fouten via `log!`. Bovenaan een `//!`-uitleg in de stijl van de andere modules.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p westplay-host page_state`
Expected: PASS (6 tests)

- [ ] **Step 5: Commit**

```bash
git add crates/host/src/page_state.rs crates/host/src/lib.rs
git commit -m "host: keep the cabin page's saved state (page_state.json)"
```

### Task 2: Host koppelt `page_state` aan de pagina

**Files:**
- Modify: `crates/host/src/lib.rs` — `enum PageMessage` (~r. 2343), de `Ready`-tak (~r. 849-915), de afhandeling van paginaberichten in de hoofdlus, het laden bij het opstarten (naast `maps_dir` e.d., ~r. 284)
- Test: `crates/host/src/lib.rs` (`#[cfg(test)]`-module ~r. 2557)

**Interfaces:**
- Consumes: `PageState::{load, all_message, set, import}` (Task 1)
- Produces (berichten die Task 3 verwacht):
  - pagina → host: `{"type":"pageState","key":string,"value":string|null}` en `{"type":"pageStateImport","values":{string:string}}`
  - host → pagina: `{"type":"pageState","values":{…}|null}` (bij `ready`) en `{"type":"pageStateChanged","key":…,"value":…|null}`

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn parses_page_state_messages() {
    // r#"{"type":"pageState","key":"westplay.recents","value":"x"}"# -> PageMessage::PageState { key, value: Some("x") }
    // r#"{"type":"pageState","key":"westplay.recents","value":null}"# -> value: None
    // r#"{"type":"pageStateImport","values":{"westplay.recents":"x"}}"# -> PageMessage::PageStateImport { values }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p westplay-host parses_page_state_messages`
Expected: FAIL (variants not defined)

- [ ] **Step 3: Implement**

`PageMessage::PageState { key: String, value: Option<String> }` en `PageMessage::PageStateImport { values: BTreeMap<String, String> }`. `PageState::load(&config.data_dir)` bij het opstarten. In de `Ready`-tak `webview.post_json(&page_state.all_message())`, vóór `settings_message`. `PageState` → `set`, en bij `Some(msg)` naar de pagina posten. `PageStateImport` → `import`, en bij `Some(msg)` posten. Dit is één plek "naar alle pagina's sturen"; plan 2 voegt daar de iPad aan toe.

- [ ] **Step 4: Run to verify it passes, plus the whole crate**

Run: `cargo test -p westplay-host`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/host/src/lib.rs
git commit -m "host: send the page its saved state and keep its changes"
```

### Task 3: `sharedStorage` in de pagina

**Files:**
- Create: `ui/src/cabin/state/sharedStorage.ts`
- Modify: `ui/src/shared/host.ts` — `HostMessage` (~r. 94-269) en `PageMessage` (~r. 310-363) met de vier berichten uit Task 2
- Test: `ui/src/cabin/state/sharedStorage.test.ts`

**Interfaces:**
- Consumes: berichten uit Task 2; `isInHost`, `postToHost` (`ui/src/shared/host.ts:1177-1199`)
- Produces:
  - `SHARED_KEYS: readonly string[]` = `['westplay.cabin', 'westplay.recents', 'westplay.safari', 'westplay.youtube', 'westplay.youtubeMusic', 'westplay.spotify.kept', 'westplay.spotify.volume', 'westplay.youtubeMusic.volume']`
  - `sharedStorage: Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>` — leest uit `localStorage` (cache); `setItem`/`removeItem` schrijven de cache en posten `pageState` alleen als de waarde verandert en `isInHost()`
  - `localFields(key: string, fields: string[]): void` — voor die sleutel gaan de genoemde velden van de zustand-`state` niet naar de host en blijven bij een update van de host lokaal staan (gebruikt voor `westplay.cabin`: `['navigation', 'brightness']`)
  - `onSharedChange(key: string, rehydrate: () => unknown): void` — registreert de store die bij een update van de host opnieuw moet laden
  - `receivePageState(message: Extract<HostMessage, { type: 'pageState' | 'pageStateChanged' }>): void` — `values: null` → post `pageStateImport` met alle `SHARED_KEYS` die in `localStorage` staan (zonder lokale velden); `values` → cache bijwerken (lokale velden behouden) en geregistreerde stores waarvan de waarde veranderde rehydrateren; `pageStateChanged` → hetzelfde voor één sleutel

- [ ] **Step 1: Write the failing tests** (stub `localStorage` en `window.chrome.webview` zoals `connectHost.test.ts:4-18`)

```ts
it('imports what this page kept when the host has nothing yet') // localStorage has westplay.recents='R', westplay.map.view='V'; receivePageState({type:'pageState',values:null}) -> posted [{type:'pageStateImport', values:{'westplay.recents':'R'}}]
it('posts a change once, and not again for the same value') // setItem('westplay.recents','A') twice -> one {type:'pageState',key,value:'A'}
it('keeps keys that are not shared to itself') // setItem('westplay.map.view','x') -> nothing posted
it("keeps this screen's own fields when the host's value comes") // localFields('westplay.cabin',['navigation','brightness']); local {state:{navigation:N,brightness:0.4,textSize:2},version:3}; host value {state:{textSize:5},version:3} -> getItem gives navigation N, brightness 0.4, textSize 5
it('sends shared values without the local fields') // setItem('westplay.cabin', json with brightness) -> posted value has no brightness/navigation
it('rehydrates only the stores whose value changed') // two registered keys, pageState values changes one -> only that rehydrate called
it('does nothing with the host outside WestPlay') // no window.chrome -> setItem writes localStorage, posts nothing
```

- [ ] **Step 2: Run to verify they fail**

Run: `npm test --prefix ui -- sharedStorage`
Expected: FAIL (module not found)

- [ ] **Step 3: Implement `ui/src/cabin/state/sharedStorage.ts`**

Elke toegang tot `localStorage` in `try/catch` zoals `mapView.ts:25-31`. Lokale velden zitten in `value.state` van zustand's JSON (`{state, version}`); `version` gaat mee naar de host.

- [ ] **Step 4: Run to verify they pass**

Run: `npm test --prefix ui -- sharedStorage`
Expected: PASS (7 tests)

- [ ] **Step 5: Commit**

```bash
git add ui/src/cabin/state/sharedStorage.ts ui/src/cabin/state/sharedStorage.test.ts ui/src/shared/host.ts
git commit -m "cabin: a storage shared through the host"
```

### Task 4: Stores op `sharedStorage` zetten

**Files:**
- Modify: `ui/src/cabin/state/store.ts:226` (useCabin, behalve in de demo), en `receiveHostMessage` (`store.ts:435`) met de cases `pageState` en `pageStateChanged` → `receivePageState`
- Modify: `ui/src/cabin/maps/recents.ts:27`, `ui/src/cabin/apps/safari/safari.ts:96`, `ui/src/cabin/apps/youtube/youtube.ts:137`, `ui/src/cabin/apps/youtubeMusic/youtubeMusic.ts:157` en `:485-504`, `ui/src/cabin/apps/spotify/spotify.ts:480-499`, `ui/src/cabin/apps/spotify/kept.ts:74-92`
- Test: `ui/src/cabin/state/connectHost.test.ts`

**Interfaces:**
- Consumes: `sharedStorage`, `localFields`, `onSharedChange`, `receivePageState` (Task 3)

- [ ] **Step 1: Write the failing tests** (in `connectHost.test.ts`, met `fakeWebView`)

```ts
it('gives the host what it kept the first time') // localStorage westplay.recents kept; connectHost(); send {type:'pageState',values:null} -> posted pageStateImport contains 'westplay.recents'
it("shows the host's recent destinations") // send {type:'pageState',values:{'westplay.recents': JSON {state:{places:[P]},version:1}}} -> useRecents.getState().places equals [P]
it("keeps the screen's brightness when settings come from the host") // useCabin brightness 0.4; send pageStateChanged for westplay.cabin with textSize 5 -> textSize 5, brightness 0.4
it('the demo keeps its own') // IS_DEMO page: changing textSize posts no pageState
```

- [ ] **Step 2: Run to verify they fail**

Run: `npm test --prefix ui -- connectHost`
Expected: FAIL

- [ ] **Step 3: Implement**

`createJSONStorage(() => sharedStorage)` in de vijf `persist`-stores (useCabin alleen als het niet de demo is; daar blijft `localStorage`). `localFields('westplay.cabin', ['navigation', 'brightness'])` en `onSharedChange(name, () => store.persist.rehydrate())` naast elke store. De drie losse `localStorage`-aanroepen (Spotify `kept.ts` en de twee `VOLUME_KEY`s) via `sharedStorage`. `mapView.ts` en `launchCopies.ts` blijven ongewijzigd.

- [ ] **Step 4: Run the whole UI suite, lint and types**

Run: `npm test --prefix ui` then `npm run lint --prefix ui` then `npm run build --prefix ui`
Expected: alle tests PASS, geen lint-fouten, build slaagt

- [ ] **Step 5: Commit**

```bash
git add ui/src
git commit -m "cabin: settings, recents, Safari and players' saved state kept by WestPlay"
```

### Task 5: Proef in de echte app

**Files:** geen nieuwe; eventueel kleine fixes uit de proef.

- [ ] **Step 1:** `npm run dev`; in de cabin: tekstgrootte wijzigen, een bestemming kiezen, een Safari-tab openen. Expected: `%LOCALAPPDATA%\WestPlay\page_state.json` (of de `data_dir` van de app) bevat `westplay.cabin`, `westplay.recents`, `westplay.safari`.
- [ ] **Step 2:** App afsluiten, `page_state.json` verwijderen, opnieuw starten. Expected: alles staat er nog (de pagina importeert opnieuw) en het bestand is terug.
- [ ] **Step 3:** In het bestand de tekstgrootte met de hand wijzigen terwijl de app dicht is, starten. Expected: de cabin toont die tekstgrootte, en de helderheid van het scherm is ongewijzigd.
- [ ] **Step 4:** `cargo test --workspace` en `npm test --prefix ui`. Expected: PASS.
- [ ] **Step 5: Commit** eventuele fixes.
