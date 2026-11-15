# Plan: movable frames

Status: started 2026-11-15, on `rust-port`.

## What moves

Seven frames, each dragged as a whole by its tab. Each can be put left,
right, above or below the plot or another frame, or tabbed with one:

1. Claude Code terminal (desktop only)
2. Metadata table
3. Feature table
4. Properties (Statistics and Filtering stay together inside it)
5. Profile and groups
6. Configure plot (was a floating window)
7. Results table (was a floating window)

The plot, with its plot selector bar, is fixed in the middle: it cannot be
dragged or closed, but frames can be put around it. The menu bar, status
bar and dialogs (Preferences, Select features, Save as PNG and so on) stay
as they are; plots sent to windows of their own stay so too.

## Default layout (today's)

- Profile and groups on the left; Properties on the right.
- The Feature table to the right of the plot.
- Metadata, Claude Code and Results at the bottom, as tabs.
- Configure plot opens to the right of the plot, as a tab beside
  Properties.
- Frames that are closed take no room; a row of tabs with none showing
  takes none either.

## How

- `egui_tiles` 0.18 (egui 0.37, pure Rust, builds for the browser). A
  `Pane` enum names the frames; its `Behavior` draws each with STAMP's
  existing code. The plot pane is not draggable or closable and has no tab
  bar.
- The `show_*` flags of `StampApp` stay the truth for which frames are
  shown (View menu, shortcuts, buttons that open frames); each frame's tab
  has a × that clears its flag rather than removing the frame. Tile and
  container visibility follow the flags each frame.
- The tree is taken out of `StampApp` while it is drawn, so that frame code
  can change the app.
- The layout is saved with STAMP's other remembered settings (`Saved`),
  with a `LAYOUT_VERSION`: a saved layout from another version, or one that
  does not hold every frame exactly once, is replaced by the default. View
  > Reset layout restores the default.

## Steps

1. Frames in today's layout, with the plot fixed.
2. View menu, ×, shortcuts; Configure plot and Results table as frames.
3. Remembered layout and Reset layout.
4. Tests (existing GUI tests updated; moving, hiding, showing, restart,
   reset, Configure plot and Results docked) and screenshots.
5. Documentation: `main_window.rst`, `NEW_FEATURES.md`.

Trade-off: each movable frame has a slim tab to drag it by, which replaces
its old heading.
