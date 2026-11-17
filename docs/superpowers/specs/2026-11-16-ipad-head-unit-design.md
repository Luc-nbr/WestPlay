# iPad als CarPlay-hoofdunit — ontwerp

Datum: 2026-11-16 · Status: ter goedkeuring (bouwer-check verwerkt)

## Bedoeling en waarom

De speler wil WestPlay bedienen met een iPad naast het stuur, als een echt touchscreen, in plaats van met muis +
interactietoets. De iPad wordt een eigen CarPlay-scherm: **scherp op de iPad-resolutie en vloeiend (minimaal 30 fps,
ook de kaart)**. Zolang de iPad verbonden is, is hij de **hoofdunit**: alle bediening gaat via de iPad en het scherm in
de truck wordt een vast tweede scherm.

Waarom niet gewoon het truckbeeld spiegelen: dat beeld is 1280×720 en zou op de iPad ~2× opgeschaald en zacht zijn,
en het kan nooit vloeiender zijn dan het beeld in de truck (dat op zwakke GPU's naast het spel naar 5–15 fps zakt,
zie `crates/host/src/gpu_contention.rs`). Daarom tekent de iPad de pagina zelf.

## Doelen

- Een webapp in Safari op de iPad (thuisscherm-app, schermvullend) die over wifi met WestPlay op de PC praat.
- De iPad draait **dezelfde cabin-pagina** (`ui/cabin.html`, React) als de truck, zelf getekend op de GPU van de iPad,
  met de kaart (MapLibre) erbij. Kaart en UI halen op de iPad ≥30 fps (doel 60), los van hoe druk de PC is.
- De iPad krijgt dezelfde host-berichten (telemetrie, route, spelers, instellingen, …) als de truckpagina en stuurt
  zijn opdrachten naar dezelfde host. Wat je op de iPad doet gebeurt echt: stop toevoegen zet hem in de GPS van het
  spel, "volgende" verandert het nummer in de truck.
- **Eén gedeelde stand** (besloten door de speler): wat de pagina nu zelf bewaart in `localStorage` wordt van de
  host, zodat iPad en truck hetzelfde hebben en de truck alles nog heeft als de iPad wegvalt. Het gaat om:
  instellingen (uiterlijk, tekstgrootte, Dashboard-onderdelen, opgeslagen navigatie e.d.,
  `ui/src/cabin/state/store.ts:153-236`), recente bestemmingen (`ui/src/cabin/maps/recents.ts`), Safari-tabs en
  -geschiedenis (`ui/src/cabin/apps/safari/safari.ts:96`) en de bewaarde stand van de YouTube-app (`youtube.ts:137`).
- Wat de iPad niet zelf kan tekenen wordt gespiegeld vanaf de PC, met aanraking terug naar die speler, ≥30 fps:
  de video van YouTube en YouTube Music, en Safari-pagina's. Inloggen bij de muziekdiensten blijft zoals nu in een
  venster op de PC (`crates/host/src/signin.rs`), getypt op het echte toetsenbord: geen wachtwoorden over wifi.
  *(Keuze bouwer-check.)*
- Hoofdunit-modus: zolang de iPad verbonden is
  - is het truckscherm een **vast tweede scherm**: het toont het **Dashboard** (standaard), alleen de **kaart**, of
    het **eigen scherm van het spel** (de huidige "auto"-knop, `truckScreen`) — een keuze op de iPad — en reageert
    niet op muis/interactietoets (geen cursor);
  - neemt de plugin de muis en het toetsenbord niet meer over voor interactie of typen: die gaan terug naar het spel.
    De sneltoetsen (volume, afspelen/pauze, helderheid) blijven werken. *(Keuze bouwer-check.)*
  - gaan video en Safari-pagina's **alleen naar de iPad**.
- Valt de iPad weg (scherm op slot, Safari dicht, wifi weg): na **10 seconden** zonder verbinding wordt de truck weer
  gewoon bedienbaar zoals nu. Komt de iPad terug, dan neemt hij het weer over.
- Koppelen: de companion-app toont een code; die tik je eenmalig op de iPad in, daarna onthoudt de iPad hem.

## Niet-doelen

- Geen native iOS-app (React Native of Swift): overwogen en afgewezen, zie Aanpakken.
- Geen geluid op de iPad: muziek en video klinken uit de speakers van de truck, zoals nu.
- De iPad en de truck houden hun vluchtige UI-stand niet gelijk (welke app open, scrollpositie): de truck is vast
  tweede scherm. (Bewaarde gegevens worden wél gedeeld, zie Doelen.)
- Geen toegang van buiten het eigen netwerk; geen accounts.
- Eén hoofdunit tegelijk. Een tweede iPad die verbindt neemt het over van de eerste (de nieuwste wint).
- Stuurknoppen (Moza R5) voor volume/pauze/skip: apart vervolgproject, ná dit project.

## Anti-doelen (mislukt, ook als het technisch werkt)

- Het truckscherm wordt trager of minder betrouwbaar voor spelers **zonder** iPad.
- De PC is zonder koppeling vanaf het netwerk te bedienen, of de server staat open zonder dat de speler dat koos.
- Er ontstaat een tweede UI-codebase die naast de truck-UI bijgehouden moet worden.
- Op de iPad is de kaart schokkerig (<30 fps) of de tekst wazig.

## Randvoorwaarden

- iPadOS 16.4 of nieuwer (moderne WebGL2 in Safari).
- iPad en PC op hetzelfde netwerk. HTTP (geen HTTPS): zelfondertekende certificaten zijn op iOS onwerkbaar voor
  spelers; de koppelcode beschermt de toegang.
- Scherm aan houden: Screen Wake Lock werkt in Safari alleen via HTTPS, dus niet hier. Het koppelscherm en de
  companion vragen de speler om op de iPad Automatisch slot op "Nooit" te zetten. Gaat de iPad toch op slot, dan
  geldt de gewone terugval na 10 s. *(Keuze bouwer-check.)*
- Windows vraagt bij de eerste keer om de server door de firewall te laten (privé-netwerk).
- WestPlay valt onder GPL-3.0; een webapp heeft geen App Store nodig.

## Aanpakken (overwogen)

| Aanpak | Wat | Waarom wel/niet |
|---|---|---|
| A. Beeld spiegelen (JPEG over websocket) | Truckbeeld naar iPad, aanraking terug | Eenvoudig, maar zacht (1280×720 ~2× opgeschaald) en nooit vloeiender dan de truck. Afgewezen. |
| B. Spiegelen met H.264 (hardware) | Idem, als video | Zuiniger, maar dezelfde scherpte- en fps-grens, en veel complexer. Afgewezen. |
| X. Spiegel + eigen kaart | Spiegel alles behalve de kaart, die tekent de iPad | Kaart vloeiend, maar knoppen/tekst blijven zacht. Afgewezen door de speler (scherpte). |
| **Y. iPad draait de cabin-pagina zelf** | Zelfde pagina, berichten via websocket, alleen video/Safari gespiegeld | **Gekozen.** Alles scherp en vloeiend; één UI-codebase. Prijs: de twee schermen zijn onafhankelijk → opgelost met hoofdunit-modus. |
| Native app (React Native / Swift) | Schermen opnieuw bouwen, native kaart | Tweede UI-codebase, Mac nodig voor Swift, App Store botst met GPL. Een RN-schil om de pagina voegt weinig toe boven een thuisscherm-webapp. Afgewezen door de speler. |

## Hoe het werkt (besloten)

### Wat er al is en hergebruikt wordt
- De cabin-pagina krijgt álles via één kanaal: host → pagina met `PostWebMessageAsJson`
  (`crates/host/src/webview.rs:692`), pagina → host met `chrome.webview.postMessage`
  (`ui/src/shared/host.ts:1187-1199`). Geen host objects, geen Tauri-API's in de cabin.
- Op `{type:'ready'}` stuurt de host de hele stand opnieuw (`crates/host/src/lib.rs:847-915`).
- Er is al een **`tablet`-indeling**, gemodelleerd naar het glas van een iPad Air (pagina 1280×898, 1,425:1;
  `ui/src/cabin/state/screen.ts`). De iPad gebruikt die.
- De kaart is één gedeelde MapLibre-instantie (`ui/src/cabin/maps/liveMap.ts`) die zijn camera zelf uit de telemetrie
  berekent (~elke 50 ms een sample, interpolatie per frame) — dus op de iPad vloeiend op eigen tempo.
- Kaartdata zijn bestanden in `<data_dir>/maps` (vector tiles `.pbf`, `map.json`, `.bin`-meshes), nu bereikbaar via
  de virtuele host `https://westplay.maps/`.

### Onderdelen
1. **Lokale server in de host (Rust).** HTTP + WebSocket op het LAN.
   - Serveert: de gebouwde cabin-UI, `maps`, `customize`, muziekhoezen en Safari-tabplaatjes (de mappen die nu als
     virtuele hosts `westplay.cabin/maps/customize/music-covers/safari-tabs` zijn gekoppeld, `lib.rs:272-299`).
   - WebSocket: elk host-bericht dat naar de truckpagina gaat, gaat ook naar de verbonden iPad; berichten van de iPad
     komen in dezelfde verwerking als die van de truckpagina, met afzender erbij.
   - Staat **standaard uit**; aan te zetten in de companion-app (sectie "iPad"), die daar ook het adres en de
     koppelcode toont.
   - Koppeling: de companion toont een code; de iPad wisselt die in voor een token (in `localStorage`) dat elk
     verzoek meestuurt. Zonder geldig token: niets.
2. **Pagina in browsermodus.** De cabin-pagina herkent dat hij buiten WebView2 draait als iPad-hoofdunit:
   - Transport: WebSocket in plaats van `chrome.webview` (één plek: `ui/src/shared/host.ts`).
   - URL's van de virtuele hosts worden server-URL's.
   - Indeling `tablet`, geschaald naar het iPad-scherm (liggend), scherp op `devicePixelRatio`.
   - Bediening met aanraken (de pagina gebruikt al pointer-events); de eigen WestPlay-cursor en cursor-berichten
     (`cursorArea`, `cursorLight`) vallen weg.
   - Toetsenbord: het eigen WestPlay-toetsenbord van de pagina, niet het iOS-toetsenbord.
   - Thuisscherm-app: manifest + Apple-meta's (schermvullend), automatisch opnieuw verbinden, koppelscherm bij
     geen/ongeldig token (met de tip over Automatisch slot).
   - Keuze "Truckscherm: Dashboard / Kaart / Spelscherm".
3. **Hoofdunit-modus in host en truckpagina.**
   - Host weet of er een iPad verbonden is (met 10 s respijt na verbreken).
   - Truckpagina krijgt een bericht dat hij vast tweede scherm is (Dashboard of Kaart) en toont dat.
   - Host stuurt geen muis-input meer naar de truckpagina en tekent geen cursor zolang de iPad hoofdunit is; de
     plugin hoort via de bridge dat hij interactie en typen niet meer overneemt.
   - Plaatsing van spelers (`show`/`hide` met rechthoek, YouTube/YouTube Music/Safari) komt van de iPad.
4. **Gedeelde stand in de host.**
   - De host bewaart de gedeelde stand in een bestand in `<data_dir>` (naast de bestaande instellingen van
     `crates/host/src/settings.rs`) en stuurt hem bij `ready` mee; een wijziging van de ene pagina gaat als bericht
     naar de host, die hem opslaat en naar álle verbonden pagina's stuurt.
   - Overgang: bij de eerste start met deze versie stuurt de truckpagina zijn bestaande `localStorage`-waarden eenmalig
     naar de host, zodat niemand zijn instellingen, recente bestemmingen of tabs kwijtraakt. *(Mijn keuze.)*
   - Wat over één scherm zelf gaat (helderheid van dat scherm, maten die van de vorm van dat scherm afhangen) blijft
     per scherm. *(Mijn keuze.)*
   - Dit werkt ook zonder iPad, dus het kan als eerste stap los worden gebouwd en getest.
5. **Spiegelen van spelers naar de iPad.**
   - De speler-webviews (YouTube, YouTube Music, Safari-tabs) worden apart vastgelegd (Windows
     Graphics Capture op de visual van die webview), als JPEG over de websocket gestuurd en op de iPad getekend
     onder/achter de pagina op de plek van de uitsparing, zoals op de PC.
   - Aanraking op die plek gaat terug naar die webview als muis-/aanraakinvoer.
   - ≥30 fps.
   - In hoofdunit-modus zitten de spelers niet in het truckbeeld.

### Gedrag bij verbinden en verbreken
- iPad verbindt → stuurt `ready` → krijgt de hele stand (alleen naar die client) → host gaat in hoofdunit-modus.
- Verbinding weg → 10 s respijt → truck weer normaal bedienbaar; spelers terug naar de truckpagina (die zijn
  stand opnieuw doorgeeft).

## Overgelaten aan de bouwer

- Keuze van HTTP/WebSocket-bibliotheek, poort, en hoe de server in de host-lus hangt.
- Welke `PageMessage`-types per afzender betekenis hebben (bv. `cursorArea`, `cursorLight`, `typing` alleen van de
  truck; `brightness` van de iPad stelt de iPad zelf in) — met de regel: de iPad bedient spelers, route en
  instellingen; alles wat over het truckscherm zelf gaat blijft van de truck.
- JPEG-encoder en -kwaliteit, resolutie waarop spelers voor de iPad worden gerenderd (binnen ≥30 fps en scherp genoeg).
- Hoe de pagina de browsermodus en de serverbasis-URL bepaalt; hoe dev-modus (`npm run dev`, Vite) en release
  samengaan.
- Welke `localStorage`-sleutels precies gedeeld worden en welke per scherm blijven, volgens de regel hierboven; de
  vorm van het opslagbestand en de berichten.
- Fijnafstemming van de `tablet`-indeling op echte iPad-formaten (11", 13").
- Teksten van de companion-sectie en het koppelscherm (alle talen die de UI al heeft).
