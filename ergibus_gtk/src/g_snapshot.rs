use std::cell::RefCell;
use std::rc::Rc;

use pw_gtk_ext::{
    gtk::{self, prelude::*},
    wrapper::*,
    UNEXPECTED,
};

use ergibus_lib::{snapshot, EResult};

use crate::icons;
use ergibus_lib::content::Mutability;
use ergibus_lib::fs_objects::{DirectoryData, ExtractionStats, FileSystemObject, Name};
use ergibus_lib::snapshot::SnapshotPersistentData;
use pw_gtk_ext::glib::{Type, Value};
use pw_gtk_ext::gtk::ButtonBuilder;
use pw_gtk_ext::gtkx::list_store::{ListRowOps, ListViewSpec, WrappedListStore};
use pw_gtk_ext::gtkx::menu::MenuItemSpec;
use pw_gtk_ext::gtkx::tree_view::{TreeViewWithPopup, TreeViewWithPopupBuilder};
use pw_gtk_ext::sav_state::SAV_SELN_MADE;
use std::path::{Path, PathBuf};

#[derive(PWO)]
pub struct CurrentDirectoryManagerCore {
    h_box: gtk::Box,
    button: gtk::Button,
    label: gtk::Label,
}

#[derive(PWO, WClone)]
pub struct CurrentDirectoryManager(Rc<CurrentDirectoryManagerCore>);

impl CurrentDirectoryManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let h_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        let button = ButtonBuilder::new()
            .tooltip_text("Change directory up one level")
            .image(&icons::up_dir::sized_image_or(16).upcast::<gtk::Widget>())
            .sensitive(false)
            .build();
        let label = gtk::LabelBuilder::new()
            .halign(gtk::Align::Start)
            .xalign(0.0)
            .build();
        h_box.pack_start(&button, false, false, 0);
        h_box.pack_start(&label, true, true, 0);
        let cdm = Self(Rc::new(CurrentDirectoryManagerCore {
            h_box,
            button,
            label,
        }));
        cdm.set_curr_dir_path(path);

        cdm
    }

    pub fn set_curr_dir_path<P: AsRef<Path>>(&self, path: P) {
        self.0
            .label
            .set_text(&format!("{}", path.as_ref().display()))
    }

    pub fn set_sensitive(&self, sensitive: bool) {
        self.0.button.set_sensitive(sensitive)
    }

    pub fn connect_button_clicked<F: Fn(&gtk::Button) + 'static>(&self, f: F) {
        self.0.button.connect_clicked(f);
    }
}

#[derive(PWO)]
pub struct SnapshotManagerCore {
    v_box: gtk::Box,
    list_view: TreeViewWithPopup,
    list_store: WrappedListStore<SnapshotManagerSpec>,
    snapshot: SnapshotPersistentData,
    current_directory_manager: CurrentDirectoryManager,
    curr_dir_path: RefCell<PathBuf>,
}

#[derive(PWO, WClone, Wrapper)]
pub struct SnapshotManager(Rc<SnapshotManagerCore>);

#[derive(Default)]
struct SnapshotManagerSpec;

impl ListViewSpec for SnapshotManagerSpec {
    fn column_types() -> Vec<Type> {
        vec![Type::U32, Type::String]
    }

    fn columns() -> Vec<gtk::TreeViewColumn> {
        let col = gtk::TreeViewColumnBuilder::new()
            .title("Name")
            .expand(false)
            .resizable(false)
            .build();

        let cell = gtk::CellRendererTextBuilder::new()
            .editable(false)
            .xalign(0.0)
            .build();

        col.pack_start(&cell, false);
        col.add_attribute(&cell, "text", 1);
        vec![col]
    }
}

impl SnapshotManager {
    pub fn new(archive_name: &str, snapshot_name: &str) -> EResult<Self> {
        let snapshot = snapshot::get_named_snapshot(archive_name, snapshot_name)?;
        let base_dir_path = snapshot.base_dir_path().to_path_buf();
        let current_directory_manager = CurrentDirectoryManager::new(&base_dir_path);
        let v_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Vertical)
            .build();
        v_box.pack_start(&current_directory_manager.pwo(), false, false, 0);
        let list_store = WrappedListStore::<SnapshotManagerSpec>::new();
        let list_view = TreeViewWithPopupBuilder::new()
            .enable_grid_lines(gtk::TreeViewGridLines::Horizontal)
            .width_request(640)
            .selection_mode(gtk::SelectionMode::Multiple)
            .menu_item((
                "extract_to",
                MenuItemSpec(
                    "Extract To",
                    None,
                    Some("Extract selected items to nominated directory."),
                ),
                SAV_SELN_MADE,
            ))
            .build(&list_store);
        let scrolled_window = gtk::ScrolledWindow::new(
            Option::<&gtk::Adjustment>::None,
            Option::<&gtk::Adjustment>::None,
        );
        scrolled_window.add(&list_view.pwo());
        v_box.pack_start(&scrolled_window, true, true, 0);
        v_box.show_all();
        let snapshot_manager = Self(Rc::new(SnapshotManagerCore {
            v_box,
            list_view,
            list_store,
            snapshot,
            curr_dir_path: RefCell::new(base_dir_path.clone()),
            current_directory_manager,
        }));
        snapshot_manager.set_curr_dir_path(&base_dir_path);
        snapshot_manager.repopulate();

        let snapshot_manager_clone = snapshot_manager.clone();
        snapshot_manager.0.list_view.connect_double_click(move |v| {
            snapshot_manager_clone.process_double_click(v);
        });

        let snapshot_manager_clone = snapshot_manager.clone();
        snapshot_manager
            .0
            .current_directory_manager
            .connect_button_clicked(move |_| snapshot_manager_clone.change_dir_to_parent());

        let snapshot_manager_clone = snapshot_manager.clone();
        snapshot_manager
            .0
            .list_view
            .connect_popup_menu_item("extract_to", move |_, selection| {
                snapshot_manager_clone.extract_to(&selection)
            });

        Ok(snapshot_manager)
    }

    pub fn repopulate(&self) {
        let curr_dir_path = self.0.curr_dir_path.borrow();
        let curr_dir = self
            .0
            .snapshot
            .find_subdir(&*curr_dir_path)
            .expect(UNEXPECTED);
        let rows: Vec<Vec<Value>> = curr_dir
            .contents()
            .enumerate()
            .map(|(u, s)| vec![(u as u32).to_value(), s.name().to_string_lossy().to_value()])
            .collect();
        self.0.list_store.repopulate_with(&rows);
    }

    fn curr_dir(&self) -> &DirectoryData {
        let curr_dir_path = self.0.curr_dir_path.borrow();
        self.0
            .snapshot
            .find_subdir(&*curr_dir_path)
            .expect(UNEXPECTED)
    }

    fn set_curr_dir_path<P: AsRef<Path>>(&self, path: P) {
        let mut curr_dir_path = self.0.curr_dir_path.borrow_mut();
        *curr_dir_path = path.as_ref().to_path_buf();
        self.0.current_directory_manager.set_curr_dir_path(path);
        self.0
            .current_directory_manager
            .set_sensitive(*curr_dir_path != self.0.snapshot.root_dir_path());
    }

    fn change_dir_to_parent(&self) {
        let curr_dir_path = self.0.curr_dir_path.borrow().to_owned();
        if let Some(parent_dir_path) = curr_dir_path.parent() {
            self.set_curr_dir_path(parent_dir_path);
            self.repopulate();
        }
    }

    fn process_double_click(&self, value: &Value) {
        let index = value.get_some::<u32>().expect(UNEXPECTED) as usize;
        let curr_dir = self.curr_dir();
        match curr_dir[index] {
            FileSystemObject::Directory(ref dir_data) => {
                self.set_curr_dir_path(dir_data.path());
                self.repopulate();
            }
            _ => (),
        }
    }

    fn extract_to(&self, values: &[Value]) {
        let extraction_options = ExtractionOptions::new();
        if self.present_widget_cancel_or_ok(&extraction_options.pwo()) == gtk::ResponseType::Ok {
            if let Some(target_dir_path) = extraction_options.target_dir_path() {
                let overwrite = extraction_options.overwrite();
                let content_mgmt_key = self.0.snapshot.content_mgmt_key();
                let curr_dir = self.curr_dir();
                let mut extraction_stats = ExtractionStats::default();
                for index in values
                    .iter()
                    .map(|v| v.get_some::<u32>().expect(UNEXPECTED) as usize)
                {
                    match &curr_dir[index] {
                        FileSystemObject::Directory(dir_data) => {
                            match dir_data.copy_to(
                                &target_dir_path.join(dir_data.name()),
                                content_mgmt_key,
                                overwrite,
                            ) {
                                Ok(stats) => extraction_stats += stats,
                                Err(err) => self.report_error("error", &err),
                            }
                        }
                        FileSystemObject::File(file_data) => {
                            match content_mgmt_key.open_content_manager(Mutability::Immutable) {
                                Ok(content_mgr) => match file_data.copy_contents_to(
                                    &target_dir_path.join(file_data.name()),
                                    &content_mgr,
                                    overwrite,
                                ) {
                                    Ok(bytes) => {
                                        extraction_stats.file_count += 1;
                                        extraction_stats.bytes_count += bytes;
                                    }
                                    Err(err) => self.report_error("error", &err),
                                },
                                Err(err) => self.report_error("error", &err),
                            }
                        }
                        FileSystemObject::SymLink(link_data, is_dir) => {
                            match link_data
                                .copy_link_as(&target_dir_path.join(link_data.name()), overwrite)
                            {
                                Ok(_) => {
                                    if *is_dir {
                                        extraction_stats.dir_sym_link_count += 1
                                    } else {
                                        extraction_stats.file_sym_link_count += 1
                                    }
                                }
                                Err(err) => self.report_error("error", &err),
                            }
                        }
                    }
                }
                self.inform_user(
                    "Extraction complete.",
                    Some(&format_for_inform(&extraction_stats)),
                );
            }
        }
    }
}

fn format_for_inform(extraction_stats: &ExtractionStats) -> String {
    format!("{:16} Directories\n{:16} Files\n{:16} Bytes\n{:16} Directory Sym Links\n{:16} File Sym Links\n",
            extraction_stats.dir_count,
            extraction_stats.file_count,
            extraction_stats.bytes_count,
            extraction_stats.dir_sym_link_count,
            extraction_stats.file_sym_link_count
    )
}

#[derive(PWO)]
struct ExtractionOptionsCore {
    v_box: gtk::Box,
    overwrite: gtk::CheckButton,
    file_chooser_button: gtk::FileChooserButton,
}

#[derive(PWO, WClone)]
struct ExtractionOptions(Rc<ExtractionOptionsCore>);

impl ExtractionOptions {
    fn new() -> Self {
        let v_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let overwrite = gtk::CheckButtonBuilder::new()
            .label("overwrite")
            .tooltip_text("Overwrite existing files?")
            .active(false)
            .build();
        v_box.pack_start(&overwrite, false, false, 0);
        let file_chooser_button = gtk::FileChooserButtonBuilder::new()
            .create_folders(true)
            .action(gtk::FileChooserAction::SelectFolder)
            .build();
        let h_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        h_box.pack_start(&gtk::Label::new(Some("Target Directory:")), false, false, 0);
        h_box.pack_start(&file_chooser_button, true, true, 0);
        v_box.pack_start(&h_box, false, false, 0);
        v_box.show_all();
        Self(Rc::new(ExtractionOptionsCore {
            v_box,
            overwrite,
            file_chooser_button,
        }))
    }

    fn overwrite(&self) -> bool {
        self.0.overwrite.get_active()
    }

    fn target_dir_path(&self) -> Option<PathBuf> {
        self.0.file_chooser_button.get_filename()
    }
}
