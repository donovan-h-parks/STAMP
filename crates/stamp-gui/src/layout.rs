//! The arrangement of the main window: frames that can be moved around the
//! plot by dragging their tabs, using `egui_tiles`.
//!
//! The plot stays in the middle. Seven frames can be put to any side of it
//! or of each other, or tabbed together: Profile and groups, Properties,
//! the Feature table, the Metadata table, the Results table, Configure plot
//! and (on the desktop) Claude Code. Which frames are shown is decided by
//! the window's own flags (the View menu); the layout only follows them.
//! See `docs/LAYOUT_PLAN.md`.

use eframe::egui;
use egui_tiles::{Container, Tile, TileId, Tiles, Tree};

/// Identifies the layout of a version of STAMP. A remembered layout made
/// under another number is replaced by the default. **Increase it whenever
/// a frame is added or removed.**
pub const LAYOUT_VERSION: u32 = 1;

/// A part of the window that has a place in the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Frame {
    /// The plot and the bar of controls beneath it: fixed.
    Plot,
    Profile,
    Properties,
    Features,
    Metadata,
    Results,
    ConfigurePlot,
    ClaudeCode,
}

impl Frame {
    pub const ALL: [Frame; 8] = [
        Frame::Plot,
        Frame::Profile,
        Frame::Properties,
        Frame::Features,
        Frame::Metadata,
        Frame::Results,
        Frame::ConfigurePlot,
        Frame::ClaudeCode,
    ];

    /// The name on its tab.
    pub fn title(self) -> &'static str {
        match self {
            Frame::Plot => "Plot",
            Frame::Profile => "Profile and groups",
            Frame::Properties => "Properties",
            Frame::Features => "Feature table",
            Frame::Metadata => "Metadata table",
            Frame::Results => "Results table",
            Frame::ConfigurePlot => "Configure plot",
            Frame::ClaudeCode => "Claude Code",
        }
    }

    /// Whether the frame can be moved and closed (all but the plot).
    pub fn is_movable(self) -> bool {
        self != Frame::Plot
    }
}

/// The layout STAMP starts with, as the window was before frames could be
/// moved: Profile and groups on the left; the plot, the Feature table and
/// Properties (with Configure plot as a tab beside it) to its right; the
/// Metadata table, Claude Code and the Results table as tabs along the
/// bottom.
pub fn default_tree() -> Tree<Frame> {
    let mut tiles = Tiles::default();
    let tabs = |tiles: &mut Tiles<Frame>, frames: &[Frame]| {
        let children = frames.iter().map(|f| tiles.insert_pane(*f)).collect();
        tiles.insert_tab_tile(children)
    };
    let profile = tabs(&mut tiles, &[Frame::Profile]);
    let plot = tabs(&mut tiles, &[Frame::Plot]);
    let features = tabs(&mut tiles, &[Frame::Features]);
    let properties = tabs(&mut tiles, &[Frame::Properties, Frame::ConfigurePlot]);
    let bottom = tabs(
        &mut tiles,
        &[Frame::Metadata, Frame::ClaudeCode, Frame::Results],
    );
    let top = tiles.insert_horizontal_tile(vec![profile, plot, features, properties]);
    // Widths and heights in points of a 1280 by 800 window (only their
    // proportions matter): the side frames as wide as their controls.
    set_shares(
        &mut tiles,
        top,
        &[
            (profile, 250.0),
            (plot, 435.0),
            (features, 270.0),
            (properties, 325.0),
        ],
    );
    let root = tiles.insert_vertical_tile(vec![top, bottom]);
    set_shares(&mut tiles, root, &[(top, 560.0), (bottom, 220.0)]);
    Tree::new(egui::Id::unique("stamp-layout"), root, tiles)
}

/// Set the sizes of a row's or column's children, given in points, as
/// shares around 1.0 (a frame of average size), the scale egui_tiles gives a
/// frame dropped into the row: it then gets an ordinary size.
fn set_shares(tiles: &mut Tiles<Frame>, container: TileId, sizes: &[(TileId, f32)]) {
    let mean = sizes.iter().map(|(_, size)| size).sum::<f32>() / sizes.len().max(1) as f32;
    if let Some(Tile::Container(Container::Linear(linear))) = tiles.get_mut(container) {
        for (child, size) in sizes {
            linear.shares.set_share(*child, size / mean);
        }
    }
}

/// The tile holding `frame`.
pub fn tile_of(tree: &Tree<Frame>, frame: Frame) -> Option<TileId> {
    tree.tiles.find_pane(&frame)
}

/// Whether a layout (remembered from an earlier run) holds every frame
/// exactly once, and so can be used.
pub fn is_complete(tree: &Tree<Frame>) -> bool {
    tree.root().is_some()
        && Frame::ALL.iter().all(|frame| {
            tree.tiles
                .tiles()
                .filter(|tile| matches!(tile, Tile::Pane(f) if f == frame))
                .count()
                == 1
        })
}

/// The children of each row and column, with whether each takes room.
fn linear_children(tree: &Tree<Frame>) -> Vec<(TileId, Vec<(TileId, bool)>)> {
    tree.tiles
        .iter()
        .filter_map(|(id, tile)| match tile {
            Tile::Container(Container::Linear(linear)) => Some((
                *id,
                linear
                    .children
                    .iter()
                    .map(|c| (*c, tree.tiles.is_visible_in_layout(*c)))
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

/// The plot's tile and the containers it is in.
fn plot_path(tree: &Tree<Frame>) -> Vec<TileId> {
    let mut path = Vec::new();
    let mut tile = tile_of(tree, Frame::Plot);
    while let Some(id) = tile {
        path.push(id);
        tile = tree.tiles.parent_of(id);
    }
    path
}

/// Show the frames `shown` says are shown and hide the others. A frame that
/// has just been shown is brought to the front of its tabs, and tabs whose
/// open tab is hidden open a shown one instead. The room of a frame that is
/// hidden goes to the plot (to the part of its row or column that holds
/// the plot), and comes back when it is shown again.
pub fn sync_visibility(tree: &mut Tree<Frame>, shown: impl Fn(Frame) -> bool) {
    let before = linear_children(tree);
    let mut appeared = Vec::new();
    for frame in Frame::ALL {
        let Some(tile) = tile_of(tree, frame) else {
            continue;
        };
        let visible = shown(frame);
        if visible && !tree.tiles.is_visible(tile) {
            appeared.push(tile);
        }
        tree.tiles.set_visible(tile, visible);
    }
    for tile in appeared {
        tree.make_active(|id, _| id == tile);
    }
    let path = plot_path(tree);
    for (container, children) in before {
        let Some(plot_child) = children.iter().map(|(c, _)| *c).find(|c| path.contains(c)) else {
            continue;
        };
        let now: Vec<(TileId, bool)> = children
            .iter()
            .map(|(c, _)| (*c, tree.tiles.is_visible_in_layout(*c)))
            .collect();
        if let Some(Tile::Container(Container::Linear(linear))) = tree.tiles.get_mut(container) {
            for ((child, was), (_, is)) in children.iter().zip(&now) {
                if *child == plot_child || was == is {
                    continue;
                }
                let share = linear.shares[*child];
                let plot = linear.shares[plot_child];
                let moved = if *is { -share } else { share };
                linear
                    .shares
                    .set_share(plot_child, (plot + moved).max(0.05));
            }
        }
    }
    let visible: Vec<TileId> = tree
        .tiles
        .tile_ids()
        .filter(|id| tree.tiles.is_visible(*id))
        .collect();
    for tile in tree.tiles.tiles_mut() {
        if let Tile::Container(Container::Tabs(tabs)) = tile {
            let active_visible = tabs.active.is_some_and(|a| visible.contains(&a));
            if !active_visible {
                if let Some(first) = tabs.children.iter().find(|c| visible.contains(c)) {
                    tabs.active = Some(*first);
                }
            }
        }
    }
}

/// How the frames are drawn and what the tabs allow. Frame contents are
/// drawn by `draw` (the window's own code).
pub struct Behavior<'a> {
    pub draw: &'a mut dyn FnMut(Frame, &mut egui::Ui),
    /// Frames whose tab was closed this frame.
    pub closed: Vec<Frame>,
    /// The tabs container that holds only the plot, whose tab bar is not
    /// shown.
    pub plot_tabs: Option<TileId>,
}

impl egui_tiles::Behavior<Frame> for Behavior<'_> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: TileId,
        frame: &mut Frame,
    ) -> egui_tiles::UiResponse {
        // egui_tiles leaves the background to the frame.
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, ui.visuals().panel_fill);
        (self.draw)(*frame, ui);
        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, frame: &Frame) -> egui::WidgetText {
        frame.title().into()
    }

    fn is_tab_closable(&self, tiles: &Tiles<Frame>, tile_id: TileId) -> bool {
        tiles.get_pane(&tile_id).is_some_and(|f| f.is_movable())
    }

    /// Closing a tab hides its frame (the View menu shows it again); the
    /// frame keeps its place.
    fn on_tab_close(&mut self, tiles: &mut Tiles<Frame>, tile_id: TileId) -> bool {
        if let Some(frame) = tiles.get_pane(&tile_id) {
            self.closed.push(*frame);
        }
        false
    }

    fn is_tile_draggable(&self, tiles: &Tiles<Frame>, tile_id: TileId) -> bool {
        match tiles.get(tile_id) {
            Some(Tile::Pane(frame)) => frame.is_movable(),
            // Containers are moved by moving their frames.
            _ => false,
        }
    }

    fn is_tab_bar_visible(&self, tile_id: TileId, _tabs: &egui_tiles::Tabs) -> bool {
        Some(tile_id) != self.plot_tabs
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            // Every frame has a tab to drag it by.
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }

    /// The keys go to the frames (Claude Code needs Ctrl+Tab and the like).
    fn cycle_tabs_with_keyboard(&self) -> bool {
        false
    }

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        2.0
    }

    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        22.0
    }
}

/// The tabs container of the plot, while it holds the plot alone.
pub fn plot_tabs(tree: &Tree<Frame>) -> Option<TileId> {
    let plot = tile_of(tree, Frame::Plot)?;
    let parent = tree.tiles.parent_of(plot)?;
    match tree.tiles.get(parent) {
        Some(Tile::Container(Container::Tabs(tabs))) if tabs.children.len() == 1 => Some(parent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_layout_holds_every_frame_once() {
        let tree = default_tree();
        assert!(is_complete(&tree));
        assert!(plot_tabs(&tree).is_some());
        let mut missing = default_tree();
        let tile = tile_of(&missing, Frame::Results).unwrap();
        missing.remove_recursively(tile);
        assert!(!is_complete(&missing));
    }

    #[test]
    fn hidden_frames_leave_their_tabs_on_a_shown_one() {
        let mut tree = default_tree();
        sync_visibility(&mut tree, |f| f != Frame::Metadata);
        let bottom = tree
            .tiles
            .parent_of(tile_of(&tree, Frame::Metadata).unwrap())
            .unwrap();
        let Some(Tile::Container(Container::Tabs(tabs))) = tree.tiles.get(bottom) else {
            panic!("the bottom frames are tabs");
        };
        assert_ne!(tabs.active, tile_of(&tree, Frame::Metadata));
        // Shown again, it comes to the front.
        sync_visibility(&mut tree, |_| true);
        let Some(Tile::Container(Container::Tabs(tabs))) = tree.tiles.get(bottom) else {
            panic!("the bottom frames are tabs");
        };
        assert_eq!(tabs.active, tile_of(&tree, Frame::Metadata));
    }

    #[test]
    fn a_hidden_frame_gives_its_room_to_the_plot() {
        let mut tree = default_tree();
        let plot_tabs = tree
            .tiles
            .parent_of(tile_of(&tree, Frame::Plot).unwrap())
            .unwrap();
        let row = tree.tiles.parent_of(plot_tabs).unwrap();
        let share = |tree: &Tree<Frame>, tile| match tree.tiles.get(row) {
            Some(Tile::Container(Container::Linear(linear))) => linear.shares[tile],
            _ => panic!("the plot is in a row"),
        };
        sync_visibility(&mut tree, |_| true);
        let plot = share(&tree, plot_tabs);
        let features = tree
            .tiles
            .parent_of(tile_of(&tree, Frame::Features).unwrap())
            .unwrap();
        let room = share(&tree, features);
        sync_visibility(&mut tree, |f| f != Frame::Features);
        assert_eq!(share(&tree, plot_tabs), plot + room);
        sync_visibility(&mut tree, |_| true);
        assert_eq!(share(&tree, plot_tabs), plot);
    }

    #[test]
    fn layouts_are_saved_and_read_back() {
        let tree = default_tree();
        let text = serde_json::to_string(&tree).unwrap();
        let back: Tree<Frame> = serde_json::from_str(&text).unwrap();
        assert!(is_complete(&back));
        assert_eq!(back, tree);
    }
}
