# Pattern Arranger

Resizable panel on the left side of the pattern editor. Open by default. Toggled via the header button.
Provides an easy and flexible way to build a song structure from individual patterns.

## Overview

The arranger defines the playback order of your project. It displays all patterns as a vertical list that can be reordered, grouped, colored, and repeated.

## Adding Patterns

Click the **+** button at the bottom of the panel. The new pattern inherits tempo, time signature, note value, measures, repeat count, and color from the currently active pattern.

## Selecting

- **Click** a pattern or group to make it active. The pattern editor always shows the currently selected or currently playing pattern.
- **Shift+Click** to select/deselect multiple patterns.
- **Right Click** the selection to open the context menu to perform actions on the selection.
- **Click a sub-pattern** inside a group to select that specific pattern within the group.

## Renaming

**Double-click** any pattern or group to rename it inline. Press **Enter** to save or **Escape** to cancel. Leaving the field empty keeps the original name.

## Reordering

**Drag and drop** any pattern or group to change its position in the sequence. Puts the pattern/group before the item it is dropped on.

## Grouping

Select 2 or more items with Shift+Click, then **right-click → Group**. The selected items merge into a named group. Sub-patterns remain individually selectable and editable within the group. The group is automatically assigned a random color that can be changed later.
Click on the arrow button to the right on the group name header to open/collapse the group.
You can not create nested groups (yet?)

## Context Menu - Pattern

Right-click a single pattern:

- **Group** — appears when 2 or more items are selected. Merges them into a group.
- **Repeat** — set how many times this pattern plays (1–999, default 1).
- **Color** — choose from 10 colors (Coral, Amber, Lime, Teal, Sky, Indigo, Violet, Rose, Mint, Slate) or clear the color. Shown as a vertical bar on the left edge.
- **Clone** — creates a linked copy. Edits to one clone affect all others.
- **Duplicate** — creates an independent copy with an incremented name.
- **Delete** — removes the pattern. Disabled when only one item remains.

## Context Menu — Group

Right-click a group header:

- **Group Repeat** — set how many times the entire group loops (1–999, default 1). This is independent of individual pattern repeats within the group.
- **Color** — applied to the group's sidebar bar.
- **Clone** — creates a linked copy of the whole group. Edits to patterns in one clone affect all others.
- **Duplicate** — creates a fully independent copy of the group and all its patterns.
- **Ungroup** — dissolves the group back into individual patterns.
- **Delete** — removes the group and its patterns. Clones and duplicates are kept.

## Clone vs Duplicate

- **Clone**: linked copy. Pattern data is shared — editing a note in one copy changes it in all clones.
- **Duplicate**: independent copy. Each copy has its own data. The name is auto-incremented (e.g. "Pattern 01" → "Pattern 02", "My Song" → "My Song 2").
