use leptos::prelude::*;

pub fn language() -> RwSignal<bool> {
    const KEY: &str = "pusun-language";
    let window = web_sys::window();
    let storage = window.as_ref().and_then(|w| w.local_storage().ok().flatten());
    let dutch = RwSignal::new(storage.as_ref().and_then(|s| s.get_item(KEY).ok().flatten()).as_deref() == Some("nl"));
    provide_context(dutch);
    Effect::new(move |_| {
        let lang = if dutch.get() { "nl" } else { "en" };
        if let Some(storage) = &storage { let _ = storage.set_item(KEY, lang); }
        if let Some(root) = window.as_ref().and_then(|w| w.document()).and_then(|d| d.document_element()) {
            let _ = root.set_attribute("lang", lang);
        }
    });
    dutch
}

pub fn dutch() -> bool {
    use_context::<RwSignal<bool>>().is_some_and(|language| language.get())
}

pub fn tr(en: &str) -> &str {
    if !dutch() { return en; }
    match en {
        "PT-9001 control" => "PT-9001-bediening",
        "Language" => "Taal",
        "Connect machine" => "Machine verbinden",
        "Connecting…" => "Verbinden…",
        "Disconnect" => "Verbinding verbreken",
        "Reboot" => "Herstart",
        "Quick Drill" => "Snelle oefening",
        "Connect the machine to start a drill." => "Verbind de machine om een oefening te starten.",
        "Drill Builder" => "Oefeningbouwer",
        "Describe each ball on its own line, numbered from 1: the shot the machine feeds and where the ball first bounces. In doubles, describe the right-hand player's balls, with left and right meaning their own half; each ball is mirrored for the left-hand player." => "Beschrijf elke bal op een eigen regel, genummerd vanaf 1: de slag die de machine geeft en waar de bal voor het eerst stuit. Beschrijf bij dubbelspel de ballen van de rechterspeler, waarbij links en rechts binnen zijn eigen helft vallen; elke bal wordt gespiegeld voor de linkerspeler.",
        "Play style" => "Speelstijl",
        "Players" => "Spelers",
        "Offensive" => "Aanvallend",
        "Defensive" => "Verdedigend",
        "Generate Drill" => "Oefening maken",
        "Saved drills" => "Opgeslagen oefeningen",
        "Export file" => "Exporteren naar bestand",
        "Import file" => "Importeren uit bestand",
        "Right-hand player" => "Rechterspeler",
        "Left-hand player" => "Linkerspeler",
        "Battery" => "Batterij",
        "Dismiss" => "Sluiten",
        "Save default" => "Als standaard opslaan",
        "Start" => "Starten",
        "Stop" => "Stoppen",
        "Shot" => "Slag",
        "Speed" => "Snelheid",
        "Frequency" => "Baltempo",
        "Spin (− back / + top)" => "Effect (− backspin / + topspin)",
        "Aim " => "Richting ",
        "left" => "links",
        "right" => "rechts",
        "center" => "midden",
        "Height " => "Hoogte ",
        "Height" => "Hoogte",
        "Court height" => "Baanhoogte",
        "Low" => "Laag",
        "Mid" => "Midden",
        "High" => "Hoog",
        "Patterns" => "Patronen",
        "Basics" => "Basis",
        "Padel shots" => "Padelslagen",
        "Horizontal" => "Horizontaal",
        "Vertical" => "Verticaal",
        "Crosscourt" => "Diagonaal",
        "Your own" => "Eigen patronen",
        "Undo" => "Ongedaan maken",
        "Clear" => "Wissen",
        "Saved programs" => "Opgeslagen programma’s",
        "Left ↔ right" => "Links ↔ rechts",
        "Interval" => "Tussenpauze",
        "Interval (s)" => "Tussenpauze (s)",
        "Play order:" => "Speelvolgorde:",
        "Reset points" => "Punten herstellen",
        "Saved point sets" => "Opgeslagen puntensets",
        "Vary Speed" => "Variabele snelheid",
        "No pause" => "Geen pauze",
        "Short pause" => "Korte pauze",
        "Long pause" => "Lange pauze",
        "Start drill" => "Oefening starten",
        "Saved routes" => "Opgeslagen routes",
        "AI Coach" => "AI-coach",
        "First ball" => "Eerste bal",
        "Left" => "Links",
        "Right" => "Rechts",
        "Level" => "Niveau",
        "Balls" => "Ballen",
        "Generate" => "Genereren",
        "Show advanced (factory) controls" => "Geavanceerde fabrieksinstellingen tonen",
        "BLE log" => "Bluetooth-logboek",
        "Load" => "Laden",
        "Delete" => "Verwijderen",
        "Save current" => "Huidige opslaan",
        "Advanced" => "Geavanceerd",
        "Read value" => "Waarde uitlezen",
        "Read value (0x71)" => "Waarde uitlezen (0x71)",
        "No 0x30 reply yet" => "Nog geen 0x30-antwoord",
        "Point editor" => "Puntenbewerker",
        "Point (1–41)" => "Punt (1–41)",
        "Point" => "Punt",
        "point" => "punt",
        "Load current" => "Huidige waarden laden",
        "Write point" => "Punt schrijven",
        "Courses" => "Cursussen",
        "Drills" => "Oefeningen",
        "Singles" => "Enkelspel",
        "Doubles" => "Dubbelspel",
        "Setup" => "Opstelling",
        "Pusun 9001 Pro, centre, about 1 m behind the baseline" => "Pusun 9001 Pro, in het midden, ongeveer 1 m achter de baseline",
        "Player 1, first ball" => "Speler 1, eerste bal",
        "Player, first ball" => "Speler, eerste bal",
        "Where each ball lands, in order; ring colour shows the player" => "Waar elke bal landt, op volgorde; de kleur van de rand toont de speler",
        "Where each ball lands, in order" => "Waar elke bal landt, op volgorde",
        "Player 2, partner’s first ball" => "Speler 2, eerste bal van de partner",
        "Difficulty" => "Moeilijkheid",
        "every" => "elke",
        "Ready-made tennis drills for one player or a doubles pair. Start plays one straight away; Edit opens it in Vary Speed so you can tune each ball first." => "Kant-en-klare tennisoefeningen voor één speler of een dubbelspelpaar. Starten speelt er meteen een; Bewerken opent hem in Variabele snelheid zodat je eerst elke bal kunt afstellen.",
        "Edit" => "Bewerken",
        "ball" => "bal",
        "balls" => "ballen",
        "Load courses" => "Cursussen laden",
        "Loading…" => "Laden…",
        "Retry" => "Opnieuw proberen",
        "← All courses" => "← Alle cursussen",
        "Practice in Vary Speed" => "Oefenen met variabele snelheid",
        "Used" => "Gebruikt",
        "times." => "keer.",
        "Fixed point" => "Vast punt",
        "Primary" => "Beginners",
        "Moon" => "Lob",
        "Random" => "Willekeurig",
        "Program panel" => "Programmeren",
        "Custom points" => "Eigen punten",
        "Serve return" => "Service retourneren",
        "Horizontal random" => "Horizontaal willekeurig",
        "Wide line" => "Brede lijn",
        "Middle line" => "Middelbrede lijn",
        "Narrow line" => "Smalle lijn",
        "Three line" => "Drie lijnen",
        "Vertical random" => "Willekeurige diepte",
        "Vertical circulation" => "Wisselende diepte",
        "Deep and light" => "Diep en kort",
        "Left deep & middle light" => "Links diep en midden kort",
        "Right deep & middle light" => "Rechts diep en midden kort",
        "Left light & middle deep" => "Links kort en midden diep",
        "Middle deep & right light" => "Midden diep en rechts kort",
        "Left deep & right light" => "Links diep en rechts kort",
        "Left light & right deep" => "Links kort en rechts diep",
        "Wheel fault" => "Storing aan het werpwiel",
        "Ball gate fault" => "Storing aan de baltoevoer",
        "Out of balls" => "Geen ballen meer",
        "Close" => "Sluiten",
        "Help" => "Uitleg",
        "Saved copy of the course list from when this site was built." => "Opgeslagen cursuslijst van het moment waarop deze site werd gebouwd.",
        "Training videos and drills from Pusun's servers. Course texts are translated from Chinese; the videos keep their original audio." => "Trainingsvideo’s en oefeningen van Pusun, met de oorspronkelijke audio en vertaalde ondertiteling.",
        "Tap court points in play order (max " => "Tik de baanpunten in speelvolgorde aan (maximaal ",
        "). Far side is at the top. 36–41 are your custom points." => "). De overkant staat bovenaan. Punten 36–41 zijn je eigen punten.",
        "Edit a point (changes are sent to the machine and saved in this browser), then tap points in play order." => "Bewerk een punt. Wijzigingen gaan naar de machine en worden in deze browser opgeslagen. Tik daarna de punten in speelvolgorde aan.",
        "Tap targets in play order (max " => "Tik de doelen in speelvolgorde aan (maximaal ",
        "), far side at the top. Each shot can be tuned below; settings belong to the point, so a repeated point plays the same." => "), met de overkant bovenaan. Stel elke slag hieronder af. Een herhaald punt gebruikt dezelfde instellingen.",
        "Generates a random drill: the first ball goes deep to the chosen side, the rest anywhere. Level low adds 0.5 s between balls, high removes 0.5 s." => "Maakt een willekeurige oefening. De eerste bal gaat diep naar de gekozen kant, de rest kan overal komen. Laag voegt 0,5 s toe tussen ballen, hoog haalt er 0,5 s af.",
        "This machine's name starts with PT7. The official app sends nothing to PT7 machines from its padel screen, so padel mode may not be supported on this model. Untested." => "De machinenaam begint met PT7. De officiële app stuurt vanuit het padelscherm geen commando’s naar PT7-machines. De padelmodus is op dit model niet getest en wordt mogelijk niet ondersteund.",
        "Hidden factory controls from the official app. They were never shown to users and have not been tested on a machine. Use at your own risk." => "Verborgen fabrieksinstellingen uit de officiële app. Ze zijn nooit aan gebruikers getoond en zijn nog niet op een machine getest. Gebruik op eigen risico.",
        "Writes one stored court point with unchecked values, then sends apply. Points 1–35 are rewritten from the height table on reconnect or height change; 36–41 from the custom points." => "Schrijft één baanpunt met ongecontroleerde waarden. Bij opnieuw verbinden of een hoogtewijziging komen punten 1–35 uit de hoogtetabel en 36–41 uit de eigen punten.",
        "No change for default value." => "De standaardwaarde is ongewijzigd.",
        "Current values have been saved as default." => "De huidige waarden zijn als standaard opgeslagen.",
        "Default removed; the pattern's built-in preset applies next time." => "De standaard is verwijderd. De volgende keer gebruikt het patroon de fabrieksinstellingen.",
        "Select at least one point first." => "Kies eerst minstens één punt.",
        "Remove this pattern's saved default" => "De opgeslagen standaard voor dit patroon verwijderen",
        "Save the current aim, speed, frequency, spin and height as this pattern's default" => "De huidige slaginstellingen als standaard voor dit patroon opslaan",
        "Reload the values currently uploaded for this point" => "De huidige waarden voor dit punt opnieuw laden",
        _ => en,
    }
}

pub fn message(en: &str) -> String {
    if dutch() {
        for (prefix, translated) in [
            ("Write failed: ", "Verzenden mislukt: "),
            ("Fault code ", "Storingscode "),
        ] {
            if let Some(detail) = en.strip_prefix(prefix) {
                return format!("{translated}{detail}");
            }
        }
    }
    tr(en).to_owned()
}

#[component]
pub fn Help(topic: &'static str) -> impl IntoView {
    let (en, nl) = match topic {
        "Tennis" => ("Selects tennis presets. Choose Tennis for a baseline drill.", "Kiest tennisinstellingen. Kies Tennis voor een oefening vanaf de achterlijn."),
        "Padel" => ("Selects padel presets. Choose Padel to see Bandeja and Bajada.", "Kiest padelinstellingen. Kies Padel voor Bandeja en Bajada."),
        "Connect machine" => ("Opens the Bluetooth chooser. Turn on the launcher and select its name.", "Opent de Bluetooth-kiezer. Zet de machine aan en kies de naam."),
        "↻" => ("Requests the battery level again. Refresh after charging.", "Vraagt de batterijstand opnieuw op. Vernieuw na het opladen."),
        "Drill text" => ("One ball per line, numbered 1, 2, 3, …: a shot (flat, drive, topspin, slice, lob, high ball, drop shot, volley) and where it first bounces (short, service line, midcourt or deep; wide left, left, centre left, centre, centre right, right or wide right; or point 1–28). In doubles, left and right are within the right-hand player's own half, and a point number is a fixed spot. For example: 1. topspin deep left.", "Eén bal per regel, genummerd 1, 2, 3, …: een slag (vlak, drive, topspin, slice, lob, hoge bal, dropshot, volley) en waar de bal voor het eerst stuit (kort, servicelijn, middenveld of diep; breed links, links, midden links, midden, midden rechts, rechts of breed rechts; of punt 1–28). Bij dubbelspel vallen links en rechts binnen de helft van de rechterspeler, en is een puntnummer een vaste plek. Bijvoorbeeld: 1. topspin diep links."),
        "Export file" => ("Downloads all saved drills as one .json file. Keep it as a backup or import it on another phone or browser.", "Downloadt alle opgeslagen oefeningen als één .json-bestand. Bewaar het als reservekopie of importeer het op een andere telefoon of browser."),
        "Import file" => ("Adds the drills from an exported .json file to your saved drills. A drill whose name is already taken gets a number after its name; nothing is overwritten.", "Voegt de oefeningen uit een geëxporteerd .json-bestand toe aan je opgeslagen oefeningen. Een oefening met een naam die al bestaat krijgt een nummer achter de naam; er wordt niets overschreven."),
        "Generate Drill" => ("Reads the text, builds the drill with a court preview and puts its balls in Vary Speed. Offensive gives slower balls and more time; Defensive faster balls and less time. Doubles mirrors each ball for the left-hand player.", "Leest de tekst, maakt de oefening met een baanvoorbeeld en zet de ballen in Variabele snelheid. Aanvallend geeft langzamere ballen en meer tijd; Verdedigend snellere ballen en minder tijd. Dubbelspel spiegelt elke bal voor de linkerspeler."),
        "Drill Builder" => ("Shows or hides the Drill Builder panel. Tap it again to close the panel.", "Toont of verbergt het paneel Oefeningbouwer. Tik nogmaals om het paneel te sluiten."),
        "Quick Drill" => ("Opens a short list of the built-in drills. One tap loads the drill at the chosen difficulty and starts the machine.", "Opent een korte lijst met de ingebouwde oefeningen. Eén tik laadt de oefening op de gekozen moeilijkheid en start de machine."),
        "Reboot" => ("Restarts the Bluetooth connection to the same machine, without the device chooser, and sends the court settings again. Use it when commands stop getting through, for example after \"GATT operation already in progress\".", "Herstart de Bluetooth-verbinding met dezelfde machine, zonder apparaatkeuze, en stuurt de baaninstellingen opnieuw. Gebruik dit als opdrachten niet meer aankomen, bijvoorbeeld na \"GATT operation already in progress\"."),
        "Disconnect" => ("Ends the Bluetooth connection. Disconnect before connecting from another device.", "Verbreekt Bluetooth. Verbreek de verbinding voordat je vanaf een ander apparaat verbindt."),
        "Dismiss" => ("Closes the message. Use it after reading an error.", "Sluit het bericht. Gebruik dit nadat je een foutmelding hebt gelezen."),
        "Save default" => ("Saves this pattern’s current shot settings. Save speed 5 to use it next time.", "Slaat de huidige slaginstellingen van dit patroon op. Sla snelheid 5 op voor de volgende keer."),
        "✕" => ("Removes this pattern’s saved settings. Select the pattern again to load its original speed.", "Verwijdert de opgeslagen instellingen. Kies het patroon opnieuw voor de oorspronkelijke snelheid."),
        "Start" => ("Starts feeding balls using the selected pattern. Select a fixed point to repeat one shot.", "Start de baltoevoer met het gekozen patroon. Kies een vast punt om één slag te herhalen."),
        "Stop" => ("Stops feeding balls. Stop before collecting balls.", "Stopt de baltoevoer. Stop voordat je ballen raapt."),
        "Undo" => ("Removes the final target. For example, 3, 5, 8 becomes 3, 5.", "Verwijdert het laatste doel. Zo wordt 3, 5, 8 bijvoorbeeld 3, 5."),
        "Clear" => ("Empties the sequence. Clear it before building a new drill.", "Wist de reeks. Wis de reeks voordat je een nieuwe oefening maakt."),
        "Reset points" => ("Restores all six custom points. Reset after experimenting with speed and aim.", "Herstelt alle zes eigen punten. Herstel ze nadat je met snelheid en richting hebt geëxperimenteerd."),
        "Start drill" => ("Feeds the selected targets in order. A drill of 3, 8, 3 feeds point 3 twice.", "Speelt de gekozen doelen in volgorde. Bij 3, 8, 3 komt punt 3 twee keer."),
        "Left" => ("Starts the generated drill deep on the left. Use it to practise opening from the left side.", "Begint de gegenereerde oefening diep links. Gebruik dit om vanaf links te beginnen."),
        "Right" => ("Starts the generated drill deep on the right. Use it to practise opening from the right side.", "Begint de gegenereerde oefening diep rechts. Gebruik dit om vanaf rechts te beginnen."),
        "Generate" => ("Creates a random drill. Choose Left and 4 balls to get a four-ball drill starting on the left.", "Maakt een willekeurige oefening. Kies Links en 4 ballen voor een reeks die links begint."),
        "Load" => ("Restores the selected saved item. Load a route to bring back its targets.", "Laadt het gekozen item. Laad een route om de doelen terug te halen."),
        "Delete" => ("Removes the selected item from this browser. Delete an old route to remove it from the list.", "Verwijdert het gekozen item uit deze browser. Verwijder een oude route uit de lijst."),
        "Save current" => ("Stores the current setup under a name. Save a route as Warm-up, for example.", "Slaat de huidige instellingen onder een naam op. Sla een route bijvoorbeeld op als Warming-up."),
        "Read value (0x71)" => ("Requests the raw factory status. Read the va1 and va2 fields when the machine replies.", "Vraagt de ruwe fabrieksstatus op. Lees va1 en va2 als de machine antwoordt."),
        "Load current" => ("Reloads the selected point into the editor. Use it to discard an unsent edit to point 4.", "Laadt het geselecteerde punt opnieuw. Gebruik dit om een ongeschreven wijziging aan punt 4 te wissen."),
        "Write point" => ("Sends the displayed values for the selected point to the machine. Select point 4 to write point 4.", "Schrijft de getoonde waarden naar de machine. Kies punt 4 om punt 4 te schrijven."),
        "Load courses" => ("Fetches the course list. Load it to find the forehand course.", "Haalt de cursuslijst op. Laad de lijst om de forehandcursus te vinden."),
        "Retry" => ("Tries to load courses again. Retry after reconnecting to the internet.", "Probeert cursussen opnieuw te laden. Probeer opnieuw zodra je internet hebt."),
        "← All courses" => ("Returns to the course list. Go back to choose another course.", "Gaat terug naar de cursuslijst. Ga terug om een andere cursus te kiezen."),
        "Practice in Vary Speed" => ("Copies course targets into Vary Speed. Adjust the speed there before starting the drill.", "Kopieert cursusdoelen naar Variabele snelheid. Pas daar vóór de oefening de snelheid aan."),
        "Difficulty" => ("Sets how hard every drill plays, from 1 (very easy) to 5 (very hard). 3 plays each drill as written; each step up feeds the next ball 0.5 s sooner and a little faster. Choose it before pressing Start or Edit.", "Bepaalt hoe zwaar elke oefening is, van 1 (heel makkelijk) tot 5 (heel moeilijk). Bij 3 speelt elke oefening zoals bedoeld; elke stap hoger komt de volgende bal 0,5 s eerder en iets sneller. Kies dit vóór Starten of Bewerken."),
        "Drill start" => ("Loads this drill and starts it at once. Connect the machine first.", "Laadt deze oefening en start hem meteen. Verbind eerst de machine."),
        "Drill stop" => ("Stops the machine feeding balls, whichever drill or pattern is playing. Stop before collecting balls.", "Stopt de baltoevoer, welke oefening of welk patroon er ook speelt. Stop voordat je ballen raapt."),
        "Drill edit" => ("Copies this drill into Vary Speed without starting it. Change a ball's speed or interval there, then press Start drill.", "Kopieert deze oefening naar Variabele snelheid zonder te starten. Pas daar de snelheid of tussenpauze van een bal aan en druk op Oefening starten."),
        "Course" => ("Opens this course. Open Forehand to see its follow-along lesson.", "Opent deze cursus. Open Forehand om de meedoenles te zien."),
        "Video" => ("Plays this lesson in the course player. Choose a breakdown to study the stroke step by step.", "Speelt deze les in de cursusspeler. Kies een analyse om de slag in stappen te bekijken."),
        "Target" => ("Adds this point to the sequence. Tap it twice to feed two balls there.", "Voegt dit punt toe aan de reeks. Tik twee keer om twee ballen daarheen te sturen."),
        "Edit point" => ("Selects a custom point for editing. Select Point 2 to adjust its speed.", "Kiest een eigen punt om te bewerken. Kies Punt 2 om de snelheid aan te passen."),
        "Court Low" => ("Applies the low court-height table. Use it for low feeds.", "Past de lage baanhoogtetabel toe. Gebruik dit voor lage ballen."),
        "Court Mid" => ("Applies the middle court-height table. Use its plus or minus button to fine-tune the level.", "Past de middelste baanhoogtetabel toe. Stel het niveau fijner af met plus of min."),
        "Court High" => ("Applies the high court-height table. Use it for higher feeds.", "Past de hoge baanhoogtetabel toe. Gebruik dit voor hogere ballen."),
        "AI Low" => ("Adds 0.5 seconds between balls. Use it for extra recovery time.", "Voegt 0,5 seconde toe tussen ballen. Gebruik dit voor extra hersteltijd."),
        "AI Mid" => ("Uses the standard interval. Try it after warming up at Low.", "Gebruikt de standaardpauze. Probeer dit na opwarmen op Laag."),
        "AI High" => ("Removes 0.5 seconds between balls. Use it to practise faster recovery.", "Haalt 0,5 seconde weg tussen ballen. Oefen zo om sneller te herstellen."),
        "Height −" => ("Lowers the middle court-height level one step, for example from 3 to 2.", "Verlaagt het middelste baanhoogteniveau één stap, bijvoorbeeld van 3 naar 2."),
        "Height +" => ("Raises the middle court-height level one step, for example from 2 to 3.", "Verhoogt het middelste baanhoogteniveau één stap, bijvoorbeeld van 2 naar 3."),
        "Balls −" => ("Feeds one fewer ball in the generated drill. Reduce five to four, for example.", "Laat de gegenereerde oefening één bal minder geven. Ga bijvoorbeeld van vijf naar vier."),
        "Balls +" => ("Feeds one more ball in the generated drill. Increase four to five, for example.", "Laat de gegenereerde oefening één bal meer geven. Ga bijvoorbeeld van vier naar vijf."),
        "Decrease" => ("Lowers the setting one step. For instance, shortening the interval brings the next ball sooner.", "Verlaagt de instelling één stap. Een kortere tussenpauze laat de volgende bal sneller komen."),
        "Increase" => ("Raises the setting one step. For instance, a longer interval gives more time between balls.", "Verhoogt de instelling één stap. Een langere tussenpauze geeft meer tijd tussen ballen."),
        "Aim left" => ("Moves the point left one step. Use it when a feed lands too far right.", "Verplaatst het punt één stap naar links. Gebruik dit als een bal te ver rechts landt."),
        "Aim right" => ("Moves the point right one step. Use it when a feed lands too far left.", "Verplaatst het punt één stap naar rechts. Gebruik dit als een bal te ver links landt."),
        "pat:fix1" => ("Fixed point 1: every ball goes to the same spot in the centre of the court. Example: press Start to groove your forehand or backhand from the middle. In padel mode the balls fly higher.", "Vast punt 1: elke bal gaat naar dezelfde plek in het midden van de baan. Voorbeeld: druk op Starten om je forehand of backhand vanuit het midden in te slijpen. In padelmodus vliegen de ballen hoger."),
        "pat:fix2" => ("Fixed point 2: every ball goes to the same spot on the right side. Example: a right-hander practises forehands without moving.", "Vast punt 2: elke bal gaat naar dezelfde plek aan de rechterkant. Voorbeeld: een rechtshandige oefent forehands zonder te lopen."),
        "pat:fix3" => ("Fixed point 3: every ball goes to the same spot on the left side. Example: a right-hander practises backhands without moving.", "Vast punt 3: elke bal gaat naar dezelfde plek aan de linkerkant. Voorbeeld: een rechtshandige oefent backhands zonder te lopen."),
        "pat:random" => ("Random: each ball goes to a random spot anywhere on the court. Example: use it at the end of a session to train reading and footwork.", "Willekeurig: elke bal gaat naar een willekeurige plek op de baan. Voorbeeld: gebruik dit aan het eind van een training om lezen en voetenwerk te oefenen."),
        "pat:primary" => ("Primary (beginner): slow, central balls without spin. Speed drops to 1. Example: a first lesson for a child or a new player.", "Beginners: langzame ballen naar het midden zonder spin. De snelheid gaat naar 1. Voorbeeld: een eerste les voor een kind of nieuwe speler."),
        "pat:smash" => ("Smash: high balls to the centre so you can practise overheads. Example: stand near the net and hit smashes. In padel mode the lob is a little lower.", "Smash: hoge ballen naar het midden om overheadslagen te oefenen. Voorbeeld: ga dicht bij het net staan en sla smashes. In padelmodus is de lob iets lager."),
        "pat:moon" => ("Moon: high, looping balls with heavy topspin (spin set to +6). Example: practise taking a high bouncing ball on the rise.", "Lob: hoge, boogballen met veel topspin (spin op +6). Voorbeeld: oefen een hoog opspringende bal vroeg te nemen."),
        "pat:volley1" => ("Volley 1: low, flat balls to the centre for net play. Example: stand at the net and block volleys straight back.", "Volley 1: lage, vlakke ballen naar het midden voor netspel. Voorbeeld: ga aan het net staan en blokkeer volleys recht terug."),
        "pat:volley2" => ("Volley 2: low, flat balls to the right for net play. Example: a right-hander practises forehand volleys.", "Volley 2: lage, vlakke ballen naar rechts voor netspel. Voorbeeld: een rechtshandige oefent forehandvolleys."),
        "pat:volley3" => ("Volley 3: low, flat balls to the left for net play. Example: a right-hander practises backhand volleys.", "Volley 3: lage, vlakke ballen naar links voor netspel. Voorbeeld: een rechtshandige oefent backhandvolleys."),
        "pat:bandeja" => ("Bandeja (padel): high balls at speed 3 for the controlled padel overhead. Example: hit bandejas from the service line back to the baseline.", "Bandeja (padel): hoge ballen op snelheid 3 voor de gecontroleerde padel-overhead. Voorbeeld: sla bandeja's vanaf de servicelijn terug naar de achterlijn."),
        "pat:bajada" => ("Bajada (padel): high balls at speed 4 that come off the back wall so you can attack them. Example: let the ball rebound off the glass and drive it down.", "Bajada (padel): hoge ballen op snelheid 4 die van de achterwand komen zodat je ze kunt aanvallen. Voorbeeld: laat de bal van het glas terugkomen en sla hem hard naar beneden."),
        "pat:sreturn" => ("Serve return (padel): low, slow balls at speed 2, like a padel serve. Example: practise returning the serve cross-court.", "Service retourneren (padel): lage, langzame ballen op snelheid 2, zoals een padelservice. Voorbeeld: oefen een cross-court return."),
        "pat:lr1" => ("Horizontal random: balls go left and right at random across the middle of the court. Example: train side-to-side footwork without knowing the next side.", "Horizontaal willekeurig: ballen gaan willekeurig links en rechts over het midden van de baan. Voorbeeld: train zijwaarts voetenwerk zonder te weten welke kant de volgende is."),
        "pat:lr2" => ("Wide line: balls alternate between the far left and far right (points 15 and 21). Example: a tough corner-to-corner running drill.", "Brede lijn: ballen wisselen tussen uiterst links en uiterst rechts (punten 15 en 21). Voorbeeld: een zware loopoefening van hoek naar hoek."),
        "pat:lr3" => ("Middle line: balls alternate left and right, a little inside the sidelines (points 16 and 20). Example: forehand, backhand, forehand at a steady pace.", "Middelbrede lijn: ballen wisselen links en rechts, iets binnen de zijlijnen (punten 16 en 20). Voorbeeld: forehand, backhand, forehand in een rustig tempo."),
        "pat:lr4" => ("Narrow line: balls alternate just left and right of centre (points 17 and 19). Example: warm up both wings with little running.", "Smalle lijn: ballen wisselen net links en rechts van het midden (punten 17 en 19). Voorbeeld: warm beide kanten op met weinig lopen."),
        "pat:lr5" => ("Three line: balls cycle left, centre, right (points 15, 18 and 21). Example: recover to the middle after every wide ball.", "Drie lijnen: ballen gaan om de beurt links, midden, rechts (punten 15, 18 en 21). Voorbeeld: herstel na elke wijde bal naar het midden."),
        "pat:ud1" => ("Vertical random: balls land short or deep at random down the middle. Example: practise moving in and back.", "Willekeurige diepte: ballen landen willekeurig kort of diep door het midden. Voorbeeld: oefen naar voren en naar achteren bewegen."),
        "pat:ud2" => ("Vertical circulation: balls step from short to deep down the centre (points 4, 11, 18, 25, 32). Example: start at the service line and move back with each ball.", "Wisselende diepte: ballen gaan in stappen van kort naar diep door het midden (punten 4, 11, 18, 25, 32). Voorbeeld: begin bij de servicelijn en ga bij elke bal verder naar achteren."),
        "pat:ud3" => ("Deep and light: balls alternate short and deep in the centre (points 4 and 25). Example: a drop shot then a deep ball, to practise moving forward and back.", "Diep en kort: ballen wisselen kort en diep in het midden (punten 4 en 25). Voorbeeld: een korte bal en dan een diepe, om vooruit en achteruit te oefenen."),
        "pat:cross1" => ("Left deep & middle light: one ball deep on the left (point 22), then one short in the middle (point 4), repeating.", "Links diep en midden kort: één bal diep links (punt 22), dan één kort in het midden (punt 4), steeds opnieuw."),
        "pat:cross2" => ("Right deep & middle light: one ball deep on the right (point 28), then one short in the middle (point 4), repeating.", "Rechts diep en midden kort: één bal diep rechts (punt 28), dan één kort in het midden (punt 4), steeds opnieuw."),
        "pat:cross3" => ("Left light & middle deep: one ball short on the left (point 1), then one deep in the middle (point 25), repeating.", "Links kort en midden diep: één bal kort links (punt 1), dan één diep in het midden (punt 25), steeds opnieuw."),
        "pat:cross4" => ("Middle deep & right light: one ball deep in the middle (point 25), then one short on the right (point 7), repeating.", "Midden diep en rechts kort: één bal diep in het midden (punt 25), dan één kort rechts (punt 7), steeds opnieuw."),
        "pat:cross5" => ("Left deep & right light: one ball deep on the left (point 22), then one short on the right (point 7). The longest run of the six.", "Links diep en rechts kort: één bal diep links (punt 22), dan één kort rechts (punt 7). De langste looproute van de zes."),
        "pat:cross6" => ("Left light & right deep: one ball short on the left (point 1), then one deep on the right (point 28), repeating.", "Links kort en rechts diep: één bal kort links (punt 1), dan één diep rechts (punt 28), steeds opnieuw."),
        "pat:program" => ("Program panel: build your own sequence from the 41 court points (up to 28 balls). Example: tap 22, 28, 4 for deep left, deep right, short middle, then press Start.", "Programmeren: maak je eigen reeks uit de 41 baanpunten (tot 28 ballen). Voorbeeld: tik 22, 28, 4 voor diep links, diep rechts, kort midden en druk op Starten."),
        "pat:custom" => ("Custom points: tune six points of your own (aim, height, speed, interval) and play them in any order. Example: make point 1 a slow lob and point 2 a fast drive, then alternate them.", "Eigen punten: stel zes eigen punten in (richting, hoogte, snelheid, tussenpauze) en speel ze in elke volgorde. Voorbeeld: maak punt 1 een langzame lob en punt 2 een harde slag en wissel ze af."),
        _ => unreachable!("Unknown help topic"),
    };
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    // Each opening gets a number, so a timer from an earlier opening cannot close it.
    let opened = StoredValue::new(0u32);
    let shown_ms = RwSignal::new(0u32);
    let close = move || {
        if let Some(dialog) = dialog.get_untracked() { dialog.close(); }
    };
    view! {
        <button type="button" class="help-button"
            aria-label=move || format!("{} · {}", tr("Help"), tr(topic))
            on:click=move |_| {
                let Some(d) = dialog.get_untracked() else { return };
                let text = if dutch() { nl } else { en };
                // Long enough to read: a base plus time per character.
                let ms = (2_500 + 55 * text.chars().count() as u32).clamp(4_000, 12_000);
                shown_ms.set(ms);
                let n = opened.get_value().wrapping_add(1);
                opened.set_value(n);
                let _ = d.show_modal();
                leptos::task::spawn_local(async move {
                    gloo_timers::future::TimeoutFuture::new(ms).await;
                    if opened.get_value() == n { close(); }
                });
            }>"?"</button>
        // Closes by itself; a tap anywhere closes it sooner.
        <dialog node_ref=dialog class="help-dialog" aria-label=move || tr("Help")
            on:click=move |_| close()>
            <p>{move || if dutch() { nl } else { en }}</p>
            <div class="help-timer" style=move || format!("animation-duration: {}ms", shown_ms.get())></div>
        </dialog>
    }
}
