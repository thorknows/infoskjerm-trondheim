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
* Aktivitetskalender (inkludert bursdager via egen ICS-kalender)

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

Infoskjermen kjører på en Raspberry Pi plassert på Knowit-kontoret i Trondheim. For å gjøre endringer eller feilsøke direkte på enheten, må du være koblet til kontornettverket (Wi-Fi: `knowit-devices`), enten fysisk til stede eller via samme nettverk.

### SSH

```shell
ssh infoskjerm@infoskjerm.local
```

* Brukernavn: `infoskjerm`
* Passord: spør en kollega eller sjekk passordhvelvet — det skal **ikke** legges inn her, siden dette repoet er offentlig.
* `infoskjerm.local` er et mDNS-navn. Om det ikke lar seg slå opp (f.eks. på Windows uten Bonjour installert), finn IP-adressen via ruterens tilkoblede enheter i stedet.

### Vanlige oppgaver

**Restarte applikasjonen** (f.eks. etter å ha endret miljøvariabler eller konfigurasjonsfiler):
```shell
pkill infoskjerm
cd ~/bin && sh run.sh &
```

**Se logger:**
```shell
tail -f ~/bin/infoskjerm.log
```

**Konfigurasjon som ligger lokalt på Pi-en** (ikke i dette repoet):
* `~/bin/run.sh` — miljøvariabler som `CALENDAR_ICS_URL`, `CALENDAR_BIRTHDAYS_ICS_URL`, `CALENDAR_SERVER_URL`, `WOLT_TRACKING_URL` m.fl.
* `~/.transportation/trackedStops.json` — hvilke Entur-stoppesteder/kaier som vises. Se `src/transportation/trackedStops.json` for formatet; legg til flere stopp ved å utvide `stops`-listen, med StopPlace- og Quay-IDer hentet fra Entur sitt GraphQL-API.
* `~/.cal/calendar.json` — manuelt lagte kalenderhendelser (lagt til via webskjemaet på `CALENDAR_SERVER_URL`, f.eks. `http://infoskjerm.local:1338/calendar`).

### Rulle ut en ny utgave

1. Push endringer og lag en ny release-tag (GitHub Actions-workflowen `Build and release` kjører automatisk på tag `v*`, eller trigges manuelt via `workflow_dispatch`).
2. På Pi-en, last ned og pakk ut den nye binærfila:
   ```shell
   cd ~/bin && python3 update.py
   ```
   Dette henter siste release fra GitHub, stopper den kjørende prosessen, og starter applikasjonen på nytt automatisk. Må kjøres fra `~/bin`, siden `update.py` pakker ut filer relativt til gjeldende mappe.
3. Om du bare har endret miljøvariabler/konfigurasjon (ikke kildekoden), holder det å restarte applikasjonen som over — ingen ny utrulling nødvendig.

Applikasjonen starter automatisk ved oppstart av Pi-en (autologin på skrivebordet → `~/.config/labwc/autostart` → `run.sh`), så en restart av hele enheten fungerer også som en enkel "restart alt"-knapp.

## Vedlikehold

### Oppdater skjema-definisjoner fra EnTurs API.

```shell
graphql-client introspect-schema "https://api.entur.io/journey-planner/v3/graphql" --output "src/resources/journey-planner_schema.json"
```
