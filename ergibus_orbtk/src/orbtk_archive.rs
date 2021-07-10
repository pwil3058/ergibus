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
                    states
                        .get_mut::<ArchiveSelectionState>(id)
                        .change_selected_archive();
                })
                .selected_index(id)
                .build(ctx),
        )
    }
}

#[derive(Debug, Default, AsAny)]
pub struct ArchiveSelectionState {
    change_selected_archive: bool,
    selected_archive: Option<String>,
}

impl ArchiveSelectionState {
    fn change_selected_archive(&mut self) {
        self.change_selected_archive = true
    }
}

impl State for ArchiveSelectionState {
    fn init(&mut self, _registry: &mut Registry, ctx: &mut Context) {
        self.selected_archive = match ArchiveSelectionView::archive_names_ref(&ctx.widget()).len() {
            0 => None,
            len_archive_names => {
                let index = *ArchiveSelectionView::selected_index_ref(&ctx.widget()) as usize;
                debug_assert!(index < len_archive_names);
                Some(ArchiveSelectionView::archive_names_ref(&ctx.widget())[index as usize].clone())
            }
        };
        println!("INIT: {:?}", self.selected_archive);
    }

    fn update(&mut self, _registry: &mut Registry, ctx: &mut Context) {
        if self.change_selected_archive {
            let index = *ArchiveSelectionView::selected_index_ref(&ctx.widget()) as usize;
            self.selected_archive =
                Some(ArchiveSelectionView::archive_names_ref(&ctx.widget())[index].clone());

            println!("UPDATE: {:?}", self.selected_archive);

            self.change_selected_archive = false;
        }
    }
}
