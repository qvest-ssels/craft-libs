//! Window › Arrange › Tile fills its grid column by column and leaves floating windows floating
//! (photocraft#16887).

use craft_behavior::dock::{Arrangement, DocumentGroup, DocumentWindows, arrange_rects};
use craft_ui::docking::{ArrangeCommands, ArrangeSpace, Arranged, arrange};
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Vec2};

/// A document area of 900 x 600 starting at y 40: Tile's cells of six documents are 300 x 300.
const AREA: Rect = Rect { min: Pos2::new(0.0, 40.0), max: Pos2::new(900.0, 640.0) };

/// The column and row of `cell` in a grid of 300 x 300 cells over `AREA`.
fn grid_place(cell: Rect) -> (u32, u32) {
    ((cell.left() / 300.0).round() as u32, ((cell.top() - AREA.top()) / 300.0).round() as u32)
}

/// Tile's six cells go down the first column, then the second, then the third: W1 over W2, W3 over W4, W5 over W6.
#[trial]
fn tile_fills_its_grid_column_by_column() {
    let cells = arrange_rects(Arrangement::Tile, AREA, 6).expect("Tile has cells for six documents");
    let places: Vec<_> = cells.iter().map(|cell| grid_place(*cell)).collect();
    assert_eq!(places, [(0, 0), (0, 1), (1, 0), (1, 1), (2, 0), (2, 1)], "column by column: {cells:?}");
    assert!(cells.iter().all(|cell| (cell.width() - 300.0).abs() < 0.01 && (cell.height() - 300.0).abs() < 0.01), "even cells: {cells:?}");
}

/// The n-up layouts keep their reading order: 6-up's cells run across the first row, then the second.
#[trial]
fn six_up_keeps_its_reading_order() {
    let cells = arrange_rects(Arrangement::SixUp, AREA, 6).expect("6-up has six cells");
    let places: Vec<_> = cells.iter().map(|cell| grid_place(*cell)).collect();
    assert_eq!(places, [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]);
}

/// Six tabbed documents tile into six docked groups in tab order, so the first cell column holds W1 over W2.
#[trial]
fn tile_docks_tabbed_documents_in_tab_order() {
    let mut windows = DocumentWindows::default();
    windows.docked = vec![DocumentGroup::new(0, vec![1, 2, 3, 4, 5, 6])];
    let space = ArrangeSpace::new(AREA, Vec2::new(400.0, 300.0), 24.0, Some(1));
    assert_eq!(arrange(&mut windows, ArrangeCommands::Tile, &space), Arranged::Windows);
    let groups: Vec<_> = windows.docked.iter().map(|group| group.documents.clone()).collect();
    assert_eq!(groups, [vec![1], vec![2], vec![3], vec![4], vec![5], vec![6]]);
    assert_eq!(windows.arrangement, Some(Arrangement::Tile));
}

/// Six documents, each in its own floating window.
fn floating_six() -> DocumentWindows<u32> {
    let mut windows = DocumentWindows::default();
    windows.floating = (1..=6)
        .map(|document| {
            let at = Pos2::new(20.0 * document as f32, 60.0 + 20.0 * document as f32);
            (DocumentGroup::new(u64::from(document), vec![document]), Rect::from_min_size(at, Vec2::new(400.0, 300.0)))
        })
        .collect();
    windows
}

/// Tile with all six documents floating keeps them floating, one per cell of a 3 x 2 grid over the document area, the
/// active one bottom right.
#[trial]
fn tile_lays_floating_windows_out_in_the_grid_and_keeps_them_floating() {
    for active in [1_u32, 4, 6] {
        let mut windows = floating_six();
        let space = ArrangeSpace::new(AREA, Vec2::new(400.0, 300.0), 24.0, Some(active));
        assert_eq!(arrange(&mut windows, ArrangeCommands::Tile, &space), Arranged::Windows);
        assert!(windows.docked.iter().all(|group| group.documents.is_empty()), "W{active} active: nothing is docked: {:?}", windows.docked);
        assert_eq!(windows.floating.len(), 6, "W{active} active: the six windows stay floating");
        assert_eq!(windows.arrangement, None, "W{active} active: the document area shows no split");
        let mut places: Vec<_> = windows.floating.iter().map(|(_, rect)| grid_place(*rect)).collect();
        let (_, active_rect) = windows.floating.iter().find(|(group, _)| group.documents == [active]).expect("the active window");
        assert_eq!(grid_place(*active_rect), (2, 1), "W{active} active: it takes the bottom-right cell");
        places.sort_unstable();
        assert_eq!(places, [(0, 0), (0, 1), (1, 0), (1, 1), (2, 0), (2, 1)], "W{active} active: one window per cell");
        assert!(windows.floating.iter().all(|(_, rect)| AREA.contains_rect(*rect)), "W{active} active: {:?}", windows.floating);
    }
}

/// With a document docked, a single floating window or a floating window with tabs, Tile docks the documents as before.
#[trial]
fn tile_docks_a_mix_of_docked_and_floating_documents_as_before() {
    let space = ArrangeSpace::new(AREA, Vec2::new(400.0, 300.0), 24.0, Some(1));
    let mut mixed = floating_six();
    mixed.docked = vec![DocumentGroup::new(10, vec![7])];
    arrange(&mut mixed, ArrangeCommands::Tile, &space);
    assert!(mixed.floating.is_empty(), "a docked document docks the floating ones: {:?}", mixed.floating);
    assert_eq!(mixed.docked.len(), 7);

    let mut single = DocumentWindows::default();
    single.floating = vec![(DocumentGroup::new(0, vec![1]), Rect::from_min_size(Pos2::new(50.0, 80.0), Vec2::new(300.0, 200.0)))];
    arrange(&mut single, ArrangeCommands::Tile, &space);
    assert!(single.floating.is_empty(), "a single floating window is docked: {:?}", single.floating);

    let mut tabbed = floating_six();
    tabbed.floating.truncate(2);
    tabbed.floating[0].0.documents.push(3);
    arrange(&mut tabbed, ArrangeCommands::Tile, &space);
    assert!(tabbed.floating.is_empty(), "a floating window with tabs docks them all: {:?}", tabbed.floating);
    assert_eq!(tabbed.docked.len(), 3, "each document gets its own cell");
}
