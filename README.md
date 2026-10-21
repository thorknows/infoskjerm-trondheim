# Infoskjerm for Trondheimkontoret

Infoskjerm for Trondheimkontoret, skrive i [Rust](https://www.rust-lang.org/), med [Slint](https://slint.dev/).

<img src="screenshot.png" width="500"/>

## Kom i gang!
* [Installer Rust](https://www.rust-lang.org/tools/install)
* Installer [Rust Analyzer for VSCode](https://code.visualstudio.com/docs/languages/rust)*
* Installer [Slint plugin for VSCode](https://marketplace.visualstudio.com/items?itemName=Slint.slint) *
* `cargo run` bygger og starter applikasjonen

    <sub>*[Rust Analyzer](https://rust-analyzer.github.io/manual.html) og [Slint plugin](https://slint.dev/get-started#integrate-with-ides) er også støtta av andre editors.</sub>

Kildekoden ligg i [src-mappa](./src). `main.rs` inneheld entrypoint for applikasjonen. Undermapper med ein `mod.rs-fil` er moduler for ulik funksjonalitet.


GUIet er laga med DSL'en Slint, og ligg i ui-mappa. [mainwindow.slint](./ui/mainwindow.slint) eksponerar `in properties` som blir tilgjengelige fra Rust-koden med setters.   F.eks:  
`in property <string> month: "jan";` eksponerar `set_month(string: SharedString)`, og har "jan" som defaultverdi.

SharedString er ein spesiell type String brukt av Slint, og lagast ved å kalle `.into()` på `&str` og `Strings`. Strengar er litt rare i Rust. Sjå gjerne [denne](https://doc.rust-lang.org/rust-by-example/std/str.html).

GUI-koden eksponerar også typar, som `Forecast`-structen i [weatherwidget.slint](./ui/forecastwidget.slint). Denne blir også tilgjengelig i Rust-koden gjennom `ui`-modulen.



## Implementert funksjonalitet
* Vær
* Klokke og dato
* Matsporing via Wolt
* Busstider
* Aktivitetskalender (inkludert bursdagar via eigen ICS-kalender)

## Idear til funksjonalitet
* Møteromsstatus
* Matsporing fra Foodora
* AI assistent: For *GPT med data og kontekst, dato, kalender, vær etc etc og be den gi råd for morgendagen eller lignende.
* Dagens visdomsord/quote
* Nyhende / overskrifter
* Countdown til neste seminar
* Comics?

## Andre tanker
* Koderydding
* Feilhåndtering
* Fikse minnelekkasje?

## Jobbe på den fysiske skjermen (kontoret)

Infoskjermen køyrer på ein Raspberry Pi plassert på Knowit-kontoret i Trondheim. For å gjere endringar eller feilsøke direkte på eininga, må du vere kopla til kontornettverket (Wi-Fi: `knowit-devices`), anten fysisk til stades eller via same nettverk.

### SSH

```shell
ssh infoskjerm@infoskjerm.local
```

* Brukarnamn: `infoskjerm`
* Passord: spør ein kollega eller sjekk passordhvelvet — det skal **ikkje** leggjast inn her, sidan dette repoet er offentleg.
* `infoskjerm.local` er eit mDNS-namn. Om det ikkje let seg slå opp (t.d. på Windows utan Bonjour installert), finn IP-adressa via ruterens tilkopla einingar i staden.

### Vanlege oppgåver

**Restarte applikasjonen** (t.d. etter å ha endra miljøvariablar eller konfigurasjonsfiler):
```shell
pkill infoskjerm
cd ~/bin && sh run.sh &
```

**Sjå loggar:**
```shell
tail -f ~/bin/infoskjerm.log
```

**Konfigurasjon som ligg lokalt på Pi-en** (ikkje i dette repoet):
* `~/bin/run.sh` — miljøvariablar som `CALENDAR_ICS_URL`, `CALENDAR_BIRTHDAYS_ICS_URL`, `CALENDAR_SERVER_URL`, `WOLT_TRACKING_URL` m.fl.
* `~/.transportation/trackedStops.json` — kva Entur-stoppestadar/kaier som vert viste. Sjå `src/transportation/trackedStops.json` for formatet; legg til fleire stopp ved å utvide `stops`-lista, med StopPlace- og Quay-IDar henta frå Entur sitt GraphQL-API.
* `~/.cal/calendar.json` — manuelt lagte kalenderhendingar (lagt til via webskjemaet på `CALENDAR_SERVER_URL`, t.d. `http://infoskjerm.local:1338/calendar`).

### Rulle ut ei ny utgåve

1. Push endringar og lag ein ny release-tag (GitHub Actions-workflowen `Build and release` køyrer automatisk på tag `v*`, eller trigges manuelt via `workflow_dispatch`).
2. På Pi-en, last ned og pakk ut den nye binærfila:
   ```shell
   cd ~/bin && python3 update.py
   ```
   Dette hentar siste release frå GitHub, stoppar den køyrande prosessen, og startar applikasjonen på nytt automatisk. Må køyrast frå `~/bin`, sidan `update.py` pakkar ut filer relativt til gjeldande mappe.
3. Om du berre har endra miljøvariablar/konfigurasjon (ikkje kildekoden), held det å restarte applikasjonen som over — ingen ny utrulling nødvendig.

Applikasjonen startar automatisk ved oppstart av Pi-en (autologin på skrivebordet → `~/.config/labwc/autostart` → `run.sh`), så ein restart av heile eininga fungerer også som ein enkel "restart alt"-knapp.

## Vedlikehold

### Oppdater skjema-definisjoner fra EnTurs API.

```shell
graphql-client introspect-schema "https://api.entur.io/journey-planner/v3/graphql" --output "src/resources/journey-planner_schema.json"
```
