// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::orbtk_archive::ArchiveSelectionView;
//use ergibus_lib::snapshot;
use orbtk::prelude::*;

widget!(SnapshotSelectionView<SnapshotSelectionState>);

impl Template for SnapshotSelectionView {
    fn template(self, id: Entity, ctx: &mut BuildContext) -> Self {
        self.child(
            Stack::new()
                .child(
                    ArchiveSelectionView::new()
                        .on_changed("selected_index", move |states, _| {
                            states
                                .get_mut::<SnapshotSelectionState>(id)
                                .change_archive();
                        })
                        .build(ctx),
                )
                .child(ListView::new().build(ctx))
                .build(ctx),
        )
    }
}

#[derive(Debug, Default, AsAny)]
struct SnapshotSelectionState {
    change_archive: bool,
}

impl SnapshotSelectionState {
    fn change_archive(&mut self) {
        self.change_archive = true;
    }
}

impl State for SnapshotSelectionState {
    fn update(&mut self, _registry: &mut Registry, _ctx: &mut Context) {
        if self.change_archive {
            println!("new archive: {}", "whatever");
            self.change_archive = false;
        }
    }
}
