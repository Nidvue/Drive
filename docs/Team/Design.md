# DESIGN - UX, interaction, accessibility, visual assets

Owner: Designer. Developer-ready specs, not illustrations of finished code, plus
the record for every visual asset the deliverable needs.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

The Designer role is not staffed in this roster: the UI was specified and built
by the Developer lane against the platform's own controls, and the record below
is what was decided and where it lives. Add a Designer when the UI work stops
being "use the platform controls".

## Principles

| principle | what it means here | check |
|---|---|---|
| Simple | one primary action per screen; the dashboard is four numbers and a state | steps in the primary flow |
| Efficient | tray menu carries pause, sync now, mount and settings | actions to complete the task |
| Usable | platform palette, keyboard-navigable Qt Quick Controls | focus visible, contrast from the system theme |
| Honest | "checking for remote changes…", never a fake socket; a dead tray icon is not shown | every state specified |

## Typography and fonts

| field | value |
|---|---|
| type scale | Qt Quick Control defaults, with 22 px for a dashboard value and 12/11 px for label and detail |
| body size | platform default, scaled by the system font setting |
| measure | not applicable (no prose blocks) |
| font stack | the platform stack; no font is shipped |
| shipped font? | none — platform stack only |
| text-as-image | never; every string is real text (`qsTr`, translatable) |

## Flows

| flow | entry | steps | success | errors | a11y |
|---|---|---|---|---|---|
| first login | main window, empty state | paste a Personal Login Token, submit | account name in the header, health turns green | the token is rejected: the message names the endpoint, the token is never echoed | the token field is a password field with a label |
| watch a sync | tray icon | click the icon | popup with speed, queue depth and conflicts | degraded health shows amber with the reason | the same numbers are in the dashboard |
| resolve a conflict | Conflicts page | pick a version, confirm | the loser becomes a dated conflict copy, the row disappears | a resolve that fails keeps the row and shows the error | the list is keyboard navigable |
| hide to tray | window close | close | window hides, sync continues | no tray host: the app quits instead of hiding with no way back | documented in the tooltip |
| configure | Settings | change a value, apply | the value is validated and persisted to `config.toml` | an invalid value is refused with the expected type and range | every field has a visible label |

## Components and states

| component | states | tokens |
|---|---|---|
| dashboard card | default, loading (dash), error (reason) | `palette.base`, radius 8, border `#33ffffff` |
| health chip | green healthy, amber degraded, grey paused, red error | `palette.highlight` `#3d7eff` for selection |
| conflict row | default, selected, resolved | list view with `reuseItems` |
| log list | default, empty, filtered | `ListView` with `reuseItems` |
| tray icon | healthy, syncing, paused, error, unavailable | a themed icon per state |
| tray popup | default, no account | compact, one action row |
| settings window | default, dirty, applied, invalid | tabs via `SettingsPane` |

## Visual system

- Colour: the platform palette (`palette.window`, `palette.base`,
  `palette.highlight`) so the system light/dark theme is honoured without a
  Qt-version-specific API. One brand accent: `#3d7eff`.
- Spacing: 12 px inside a card, 16 px between cards, 20 px page margin.
- Layout: a four-column card grid at 1080x720; the grid reflows rather than
  scrolling horizontally.
- Motion: none beyond the platform's own transitions; the app never animates a
  number it cannot source.
- Icons: the system icon theme, so a desktop without a status-notifier host shows
  no dead icon.

## Copy

| context | string |
|---|---|
| not logged in | `not logged in` |
| remote poll | `checking for remote changes…` |
| degraded health | the reason, in the same chip as the state |
| conflict filename | `<name> (conflict Nidvue YYYY-MM-DD HHMMSS)<ext>` |

## Visual assets

| id | asset | medium | purpose / surface | path | size | candidates | status |
|---|---|---|---|---|---|---|---|
| IMG-1 | application icon | vector (SVG) | window, tray, package | `apps/nidvue-ui/qml/` + the system theme | 16-512 px | 1 | not produced |

`v0.1.0` ships no generated imagery: the UI uses the platform icon theme, and no
raster or vector asset was produced by a model. That is a deliberate choice, not
an omission — a hand-drawn or generated logo would be the only asset in a
repository that otherwise has none, and it would need a licence record of its
own. When the brand mark is commissioned, it is recorded here with its medium,
licence and the candidates reviewed.

## Review log

| candidate | verdict | reason | reviewer |
|---|---|---|---|
| - | - | no asset produced | - |

## Improvement log

| round | what changed | why (criterion) | review after |
|---|---|---|---|
| 1 | platform palette instead of hard-coded colours | the system theme must be honoured | `cargo xtask ui-smoke` |
| 2 | window close hides only when a tray host exists | no dead end on a bare Wayland session | [`../deviations.md`](../deviations.md) 9b |
