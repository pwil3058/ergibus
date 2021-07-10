// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::orbtk_archive::{ArchiveSelectionState, ArchiveSelectionView};
//use ergibus_lib::snapshot;
use orbtk::prelude::*;

widget!(SnapshotSelectionView<SnapshotSelectionState>);

impl Template for SnapshotSelectionView {
    fn template(self, id: Entity, ctx: &mut BuildContext) -> Self {
        println!("{:?}", id.type_id());
        self.child(
            Stack::new()
                .id("stack")
                .child(
                    ArchiveSelectionView::new()
                        .on_changed("selected_index", move |states, entity| {
                            let type_id = entity.type_id();
                            println!("{:?}", type_id);
                            //states.get::<ArchiveSelectionState>(id);
                            states
                                .get_mut::<SnapshotSelectionState>(id)
                                .change_archive();
                        })
                        .id("archive_selector")
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
    fn update(&mut self, _registry: &mut Registry, ctx: &mut Context) {
        let _x = ctx.child("archive_selector");
        if self.change_archive {
            println!("new archive: {}: {:?}", "whatever", ctx.entity().type_id());
            self.change_archive = false;
        }
    }
}
