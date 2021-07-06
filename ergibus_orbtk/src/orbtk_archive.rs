// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use ergibus_lib::archive;
use orbtk::prelude::*;

type ArchiveNames = Vec<String>;

widget!(ArchiveSelectionView<ArchiveSelectionState> { archive_names: ArchiveNames, selected_index: i32 });

impl Template for ArchiveSelectionView {
    fn template(self, id: Entity, ctx: &mut BuildContext) -> Self {
        let archive_names = archive::get_archive_names();
        let count = archive_names.len();

        self.archive_names(archive_names).selected_index(0).child(
            ComboBox::new()
                .count(count)
                .items_builder(move |bc, index| {
                    let text =
                        ArchiveSelectionView::archive_names_ref(&bc.get_widget(id))[index].clone();
                    TextBlock::new().v_align("center").text(text).build(bc)
                })
                .on_changed("selected_index", move |states, _| {
                    states.get_mut::<ArchiveSelectionState>(id).change_archive();
                })
                .selected_index(id)
                .build(ctx),
        )
    }
}

#[derive(Debug, Default, AsAny)]
struct ArchiveSelectionState {
    change_archive: bool,
    _change_available_archives: bool,
}

impl ArchiveSelectionState {
    fn change_archive(&mut self) {
        self.change_archive = true
    }

    fn change_available_archives(&mut self) {
        self._change_available_archives = true
    }
}

impl State for ArchiveSelectionState {
    fn update(&mut self, registry: &mut Registry, ctx: &mut Context) {
        if self.change_archive {
            let index = *ArchiveSelectionView::selected_index_ref(&ctx.widget()) as usize;
            let selected_archive =
                ArchiveSelectionView::archive_names_ref(&ctx.widget())[index].clone();

            println!("selected_archive: {}", selected_archive.as_str());

            self.change_archive = false;
        }
    }
}
