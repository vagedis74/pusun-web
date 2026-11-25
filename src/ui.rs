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
        "Pattern" => ("Selects this feed pattern. For example, select a volley preset and press Start to practise at the net.", "Kiest dit balpatroon. Kies bijvoorbeeld een volleypatroon en druk op Starten om aan het net te oefenen."),
        "Decrease" => ("Lowers the setting one step. For instance, shortening the interval brings the next ball sooner.", "Verlaagt de instelling één stap. Een kortere tussenpauze laat de volgende bal sneller komen."),
        "Increase" => ("Raises the setting one step. For instance, a longer interval gives more time between balls.", "Verhoogt de instelling één stap. Een langere tussenpauze geeft meer tijd tussen ballen."),
        "Aim left" => ("Moves the point left one step. Use it when a feed lands too far right.", "Verplaatst het punt één stap naar links. Gebruik dit als een bal te ver rechts landt."),
        "Aim right" => ("Moves the point right one step. Use it when a feed lands too far left.", "Verplaatst het punt één stap naar rechts. Gebruik dit als een bal te ver links landt."),
        _ => unreachable!("Unknown help topic"),
    };
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    view! {
        <button type="button" class="help-button"
            aria-label=move || format!("{} · {}", tr("Help"), tr(topic))
            on:click=move |_| {
                if let Some(dialog) = dialog.get() { let _ = dialog.show_modal(); }
            }>"?"</button>
        <dialog node_ref=dialog class="help-dialog" aria-label=move || tr("Help")>
            <p>{move || if dutch() { nl } else { en }}</p>
            <form method="dialog">
                <button autofocus>{move || tr("Close")}</button>
            </form>
        </dialog>
    }
}
