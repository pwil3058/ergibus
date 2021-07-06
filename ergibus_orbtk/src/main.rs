// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

mod orbtk_archive;
mod orbtk_snapshots;

use orbtk::prelude::*;

use orbtk_archive::ArchiveSelectionView;
use orbtk_snapshots::SnapshotSelectionView;

fn main() {
    orbtk::initialize();

    Application::new()
        .window(|ctx| {
            Window::new()
                .title("Ergibus (OrbTk)")
                .position((2000.0, 1000.0))
                .size(300.0, 100.0)
                .resizeable(true)
                .child(
                    Stack::new()
                        .child(SnapshotSelectionView::new().build(ctx))
                        .child(
                            TextBlock::new()
                                .text("Ergibus OrbTk GUI is under construction")
                                .v_align("center")
                                .h_align("center")
                                .build(ctx),
                        )
                        .child(ArchiveSelectionView::new().build(ctx))
                        .child(ListView::new().build(ctx))
                        .build(ctx),
                )
                .build(ctx)
        })
        .run();
}
