# Rooms, patch and groups

## Rooms

Open **Rooms** to draw the spaces you light. A project can hold several rooms.

- **New room** creates a rectangle from the width and depth you enter (metres).
- Drag the purple **corners** to reshape the walls (snaps to 10 cm).
- **Double-click a wall's length label** to add a corner in the middle of that wall; **Alt-click a corner** to remove it.
- **Ceiling** is the room's height, for reference.
- **Show on Live** picks which room the Live screen shows.

### Placing fixtures

Fixtures belong to one room (or none). On the Rooms screen:

- **Place** brings a fixture from the "not in this room" list into the room.
- Drag fixtures to position them.
- Select one to set **X**, **Y**, **Height** and **Facing** exactly. The facing angle sets the direction the beam is drawn in.

Positions matter: **node hop** moves to the nearest fixture in 3D (height included), **Spread** palettes order colours by position, and **waves** sweep across positions.

With a fixture selected, **Aim at** turns its facing towards one of the room's focus points.

### Focus points

A **focus point** is a spot effects can be centred on, such as the DJ booth or the middle of the dance floor. Each has a position, height, colour and **radius** (its area of influence).

- **+ Add** in the Focus points panel creates one in the middle of the room. Drag it on the map; select it and drag the outer handle to set the radius.
- **Aim every fixture here at it** turns every fixture in the room to face it.
- Routes pick a focus point with their **from** setting (see [Routes → Focus points](routes.md#focus-points)). Deleting a focus point resets any routes that used it to the centre of their targets.

Focus points appear on the Live map too.

## Patch

**Patch** lists every fixture instance.

| Column | Notes |
|---|---|
| Name | Your name for it |
| Type / Mode | Fixture type and its DMX mode |
| Output | An Art-Net output, or **Zigbee** |
| Address / device | DMX start address (1–512), or the Zigbee device |
| Ch | Number of channels |
| Room | Which room it's in |
| On | Disabled fixtures are sent their off values |

A red **!** next to the address means it overlaps another fixture on the same output or runs past channel 512.

### Channels and manual control

Select a fixture to see its channels, grouped by projector for multi-head fixtures (Shared, Projector P1, Projector P2). Each row shows the live value and what that value does.

Moving a slider takes **manual control** of the channel. It's highlighted, and routes no longer change it until you click **Release** (or **Release all**, or the *Release N manual* button in the top bar). Manual values are also what [presets](control.md#presets) save.

### Limits

Click **Limits** on any channel to restrict it. Limits apply last, after routes, presets and manual sliders:

| Limit | Effect |
|---|---|
| Lock at | Always send this value |
| Range | Clamp into min–max. **max = now** sets the top to the current value. |
| Hold at least | A new value must last this long before it can change again (stops rapid flicker) |
| Random picks need tags | **Random value** routes only choose values carrying all these tags |

Limits belong to the fixture instance, not the type, so the same laser model can be calibrated differently in each venue.

## Fixture groups

The **Fixture groups** panel on Patch creates named sets of fixtures, like *Front pars* or *Ceiling bulbs*, to use as route and control targets.

- **New group**, then **Edit members** and click fixtures to add or remove them.
- **Add all** fills a group by fixture kind or by room.
- Each group has a colour, used for its chip in the route editor.

Deleting a fixture or group removes it from every route that used it.

## Art-Net outputs

**Art-Net** lists your outputs. Each one is one DMX universe on a node.

| Field | Notes |
|---|---|
| Send | Outputs start off, so nothing is sent until you're ready |
| IP | Send directly to the node's IP rather than broadcast where possible |
| Net / Subnet / Universe | Must match the node's settings; the resulting 15-bit port address is shown |

LightDeck sends every enabled output 40 times a second. The bar shows which addresses each fixture uses. Send errors appear here and as *Art-Net error* in the top bar.
