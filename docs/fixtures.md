# Fixture types

A **fixture type** (profile) describes a kind of light: its channels, what each value range does, its modes, and its heads. You patch **fixtures** (instances) of a type on the [Patch](rooms-and-patch.md) screen.

Open **Fixture types** in the sidebar.

## Importing from QLC+

Click **Import QLC+ .qxf** and pick one or more files. QLC+ keeps its fixtures in `~/Library/Application Support/QLC+/Fixtures/`, and its built-in library is inside the QLC+ app.

The importer:

- converts channels, value ranges, colours and modes;
- turns QLC+ **heads** into separate projectors (P1, P2…), which show up separately in Patch and Routes;
- maps QLC+ channel groups to LightDeck's (intensity, colour, pattern, beam, effect, mode…);
- adds starting tags: ranges labelled *auto* or *sound* on a mode channel, and *undefined*/*reserved* ranges, are tagged **blocked**; ranges marked lamp-off are tagged **off**; *dynamic…*, *static…* and *strobe* ranges get matching tags.

Always review the tags after importing; the heuristics are a starting point.

## Interpreting with AI

For fixtures without a QLC+ file, click **Interpret with AI** and paste anything that describes the DMX chart: a manual page, an Open Fixture Library JSON file, a spreadsheet export. You can also load a text file. LightDeck sends it to the AI provider configured on the **AI** screen and turns the reply into a fixture type you can review, then **Add as new type** or **Replace** the selected one.

**✦ AI tag** on an existing type asks the model to improve groups and tags without changing channels or values.

### AI settings

On the **AI** screen:

| Setting | Notes |
|---|---|
| API schema | *Anthropic Messages* or *OpenAI-compatible* |
| Base URL or proxy | Requests go to `{base}/v1/messages` (Anthropic) or `{base}/chat/completions` (OpenAI-compatible). A full endpoint URL also works, so gateways and proxies are fine. |
| API key / Send key as | `x-api-key`, `Authorization: Bearer`, or not sent (when your proxy adds it) |
| Model, Max tokens | Defaults: `claude-opus-5-5`, 16000 |
| Extra headers | Any headers your gateway needs |

**Save and test** sends a one-word request to check the connection. The key is stored only on this Mac (see [Projects and settings](projects-and-settings.md#where-data-lives)), and it's left out of settings exports unless you choose to include it.

## Editing a fixture type

Select a type to edit its manufacturer, model and **kind** (laser, par, fog, moving head, strobe, zigbee, other). Under **Channels and tags**, click a channel to see its value ranges:

- **From / To / Label**: the range and what it does.
- **Colour**: shown for colour-preset ranges; effects use these to pick the nearest colour.
- **Tags**: free-form labels. Some tags have meaning to the engine:

| Tag | Effect |
|---|---|
| `blocked` | Never sent. If a route or slider asks for a blocked value, the nearest allowed value is used instead. |
| `off` | The value used for blackout and disabled fixtures. |
| anything else (`safe`, `peak`, `chill`, `large_projection`…) | Used to filter **Random value** routes and limits. |

The **group** of each channel matters too: *intensity* channels follow the master fader and blackout; *intensity* and *colour* channels are what effects drive.

**Export…** saves the fixture type, with its looks, as a `.lightdeck-fixture` file. **Import LightDeck fixture** brings types in from any of these:

- a `.lightdeck-fixture` file;
- a library export;
- a `.lightdeck` project (takes all of its types);
- a settings export (takes its library);
- JSON from older versions' **Copy JSON**.

If a type with the same id is already in the project, you choose: **Merge** it in (see below), **Add as copy**, or **Skip**.

## Fixture library

The library keeps fixture types and their looks for every project on this Mac. It's how you set up a laser's looks once and reuse them in every show.

Each project keeps **its own copy** of every type it uses. So a project file always works on its own (at a gig, offline, or sent to someone else), and changing the library never changes an existing show.

Switch the list at the top of **Fixture types** between **This project** and **Library**.

- **Save to library** (on a project's fixture type) adds it to the library, or updates the library's copy.
- **Add to this project** (on a library type) copies it into the current project.
- Each project type shows how it compares with the library, and a dot in the list means it's in the library (amber if they differ). For example: "2 looks only in the library · 1 only here · channels, tags or details differ". **Update from library** brings the library's version in.
- **Export whole library…** saves every type to one file. **Import into library** reads one back (or any of the files above).
- **New project** uses your library's version of the bundled starter types, so their looks come with them.

Copying in either direction **merges**:

- The fixture type itself (channels, ranges, tags, blocked values, modes, names) becomes the source's.
- New looks and look sets are added, and ones that exist on both sides take the source's version.
- Looks that only exist on the receiving side are **kept**. A venue's own looks survive an update from the library, and saving to the library never deletes looks other projects added there.

Per-venue calibration is not part of the fixture type, so it never goes to the library: limits (lock, range, hold, required tags), addresses and positions live on each patched fixture in the project.

Library types are edited in a project: add or open the type there, change it, then **Save to library**. The library is included in [settings export](projects-and-settings.md#app-settings).

## The pattern browser

Lasers and effect fixtures have hundreds of values, most of which you'll never want. The pattern browser lets you step through them on the real fixture and catalogue the good ones.

1. Open a fixture type and click **Pattern browser**.
2. Pick the **channel** (e.g. *P1 Pattern*) and the patched fixture to **preview on**.
3. Step through values with `←` / `→` (hold `⇧` to jump by 10), or drag the slider.
4. Tag what you see with `1`–`7`: safe, chill, mid, peak, favourite, large projection, blocked. Type any other tag in **+ your own tag**; tags you've used before become one-click buttons.
5. Rate with `⇧1`–`⇧5`, and optionally name the value.
6. `↑` / `↓` jump between values you've already catalogued.

The catalogue on the right lists every tagged value. **Copy catalog to…** copies it to a matching channel, e.g. from P1 Pattern to P2 Pattern.

**Random value** routes pick from catalogued values first, weighted by rating, and only fall back to the channel's full range when nothing in the catalogue matches.

Preview takes manual control of the channels you touch; **Release preview** or leaving the tab releases them.

### Looks: a pattern plus secondary options

Often a pattern only looks right with other settings: the beam gallery, a dynamic zoom, a rotation, a colour chase. A **look** saves them together as one option.

1. In the pattern browser, find the pattern.
2. In **Secondary options**, set the other channels on the same projector with their sliders. Tick the ones to keep; moving a slider ticks it automatically.
3. Name the look, add tags, pick sets and a rating, then click **Save look**.

**Your looks** on the right lists the looks for this projector. Click one to **edit** it: it previews on the fixture, and its name, tags, sets and rating load into the form, where changes save straight away. Change channels or motions, then click **Update look** to save them into it, or **Save as new** to keep both. **New look** clears the form. Routes apply looks with the [Look action](routes.md#looks).

### Moving channels (motions)

Any channel in a look, including the pattern itself, can **move** instead of holding one value. Each secondary option has a dropdown next to its name (and the pattern has one under its slider): leave it on **Fixed**, or choose how it moves. Its settings open right under the row:

| Motion | What it does |
|---|---|
| Sweep | Glides through the ranges: there and back, up then jump, down then jump, or smooth (sine) |
| Step | Jumps through the ranges by a step size |
| Random (seeded) | Jumps to random values in the ranges. The **seed** makes it repeatable: the same seed always plays the same sequence. **New seed** tries another. |
| List of values | Plays your own list in order (**from catalogue** fills it with your catalogued values) |
| Catalogued values | Plays the catalogued values that carry the chosen tags, in order: your known patterns as an animation |
| Follow audio level | Moves through the ranges with a level source (Bass level, Energy…) |

- **Timing:** every N **beats** (locked to the BPM), every N **seconds**, or every N **route hits** (each time the Look route fires).
- **Ranges:** you can give several, and they're played in order as if they were one long range. The chips under the ranges add a capability range in one click (e.g. *Dynamic zoom in*), or **all usable** adds every range that isn't blocked.

While any channel is moving, the browser previews the whole look live on the fixture. Saved looks keep their motions, and the Patterns tab shows moving channels with ⟳.

Because everything runs from the beat, the hit count or the seed, motions are predictable: a look plays the same way each time it's triggered.

## Patterns tab: looks, sets and defaults

The **Patterns** tab manages every look on a fixture type.

- **Looks:** each row has the name, its projector, **ⓘ** (hover for the channel list, click to show each channel's value or motion), rating and **Preview**. Tags and sets are on the second line; a set with ✓ is on. The **⋯** menu has Keep (for defaults), Duplicate, Show channels and Delete. Filter by *Yours* / *Defaults*, by set, or by search.
- **Sets:** named groups of looks such as *Chill*, *Build*, *Drop*. A look can be in several sets. Sets are matched by name, so a *Drop* set on the laser and a *Drop* set on another fixture type work together in one route.
- **Default looks:** **Generate defaults** creates starter looks from the fixture's own pattern ranges (a few values spread through each usable range, per projector), tagged `default`. **Regenerate** replaces only those, never your own looks. **Keep** turns a default into one of yours so it survives regeneration. Routes can include or leave out defaults.

## Laser safety and low ceilings

Many lasers' built-in sound modes project a full square that hits a low ceiling, and they ignore size and position channels in those modes. To keep everything below the ceiling, run the laser in **DMX manual** and let LightDeck choose patterns inside a safe area:

1. **Lock the mode.** In Patch, open the laser's *Mode* channel → **Limits** → **Lock at** a DMX-manual value (the starter show locks it at 50). The auto and sound ranges are also tagged `blocked`.
2. **Set the clipping window.** On lasers like the L2600, *P1 Out-of-bounds* in its *Blanking* range (200–255) blanks anything outside a window. Use a **Range** limit to keep it there.
3. **Clamp size and position.** Give *P1 Size X/Y* a **Range** limit inside the static range (e.g. 20–95), and give *P1 Position Y* a range that shifts the image down. Block the *dynamic move up* range with a `blocked` tag.
4. **Calibrate.** With a test pattern showing, adjust those limits until nothing reaches the ceiling. The **max = now** button sets a range's top to the current value.
5. **Catalogue safe patterns** with the pattern browser, tagging them `safe`, then give *P1 Pattern* a limit that requires the `safe` tag. Random routes then only use those.
6. Repeat for P2.

Limits apply after routes, presets and sliders, so nothing can push the laser outside them.
