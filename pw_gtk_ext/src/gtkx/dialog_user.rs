// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use glib::Cast;
use gtk::prelude::WidgetExtManual;
use gtk::{ButtonExt, EntryExt, FileChooserExt, WidgetExt};
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::process;

use crate::gdk::prelude::IsA;
use crate::glib::markup_escape_text;
use crate::gtk::{BoxExt};
use crate::gtkx::entry::PathCompletion;
use crate::sourceview::prelude::{DialogExt, GtkWindowExt};

pub trait TopGtkWindow {
    fn get_toplevel_gtk_window(&self) -> Option<gtk::Window>;
}

macro_rules! implement_tgw_for_widget {
    ( $f:ident ) => {
        impl TopGtkWindow for gtk::$f {
            fn get_toplevel_gtk_window(&self) -> Option<gtk::Window> {
                if let Some(widget) = self.get_toplevel() {
                    if widget.is_toplevel() {
                        if let Ok(window) = widget.dynamic_cast::<gtk::Window>() {
                            return Some(window);
                        }
                    }
                };
                None
            }
        }
    };
}

implement_tgw_for_widget!(Bin);
implement_tgw_for_widget!(Box);
implement_tgw_for_widget!(Container);
implement_tgw_for_widget!(DrawingArea);
implement_tgw_for_widget!(Entry);
implement_tgw_for_widget!(EventBox);
implement_tgw_for_widget!(Frame);
implement_tgw_for_widget!(Grid);
implement_tgw_for_widget!(Layout);
implement_tgw_for_widget!(ListBox);
implement_tgw_for_widget!(Notebook);
implement_tgw_for_widget!(Paned);
implement_tgw_for_widget!(ScrolledWindow);
implement_tgw_for_widget!(Stack);
implement_tgw_for_widget!(TextView);
implement_tgw_for_widget!(TreeView);
implement_tgw_for_widget!(Widget);
implement_tgw_for_widget!(Window);
implement_tgw_for_widget!(ApplicationWindow);
implement_tgw_for_widget!(Dialog);
implement_tgw_for_widget!(AboutDialog);
implement_tgw_for_widget!(AppChooserDialog);
implement_tgw_for_widget!(ColorChooserDialog);
implement_tgw_for_widget!(FileChooserDialog);
implement_tgw_for_widget!(FontChooserDialog);
implement_tgw_for_widget!(MessageDialog);
implement_tgw_for_widget!(RecentChooserDialog);

pub fn parent_none() -> Option<&'static gtk::Window> {
    let none: Option<&gtk::Window> = None;
    none
}

pub static CANCEL_OK_BUTTONS: &[(&str, gtk::ResponseType)] = &[
    ("Cancel", gtk::ResponseType::Cancel),
    ("Ok", gtk::ResponseType::Ok),
];

fn markup_cmd_fail(cmd: &str) -> String {
    format!(
        "{}: <span foreground=\"red\">FAILED</span>",
        markup_escape_text(cmd)
    )
}

fn markup_cmd_warn(cmd: &str) -> String {
    format!(
        "{}: <span foreground=\"orange\">WARNED</span>",
        markup_escape_text(cmd)
    )
}

fn markup_cmd_ok(cmd: &str) -> String {
    format!(
        "{}: <span foreground=\"green\">OK</span>",
        markup_escape_text(cmd)
    )
}

fn markup_output(output: &process::Output) -> Option<String> {
    if !output.stdout.is_empty() {
        let stdout = markup_escape_text(&String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            let stderr = markup_escape_text(&String::from_utf8_lossy(&output.stderr));
            Some(format!(
                "{}\n<span foreground=\"red\">{}</span>",
                stdout, stderr
            ))
        } else {
            Some(stdout.to_string())
        }
    } else if !output.stderr.is_empty() {
        let stderr = markup_escape_text(&String::from_utf8_lossy(&output.stderr));
        Some(format!("<span foreground=\"red\">{}</span>", stderr))
    } else {
        None
    }
}

pub trait DialogUser: TopGtkWindow {
    const CLOSE_BUTTONS: [(&'static str, gtk::ResponseType); 1] =
        [("Close", gtk::ResponseType::Close)];
    const CANCEL_OK_BUTTONS: [(&'static str, gtk::ResponseType); 2] = [
        ("Cancel", gtk::ResponseType::Cancel),
        ("Ok", gtk::ResponseType::Ok),
    ];

    fn set_transient_for_and_icon_on<W: GtkWindowExt>(&self, window: &W) {
        if let Some(tlw) = self.get_toplevel_gtk_window() {
            window.set_transient_for(Some(&tlw));
            if let Some(ref icon) = tlw.get_icon() {
                window.set_icon(Some(icon));
            }
        };
    }

    fn new_colour_chooser_dialog_builder(&self) -> gtk::ColorChooserDialogBuilder {
        let mut dialog_builder = gtk::ColorChooserDialogBuilder::new();
        if let Some(tlw) = self.get_toplevel_gtk_window() {
            if let Some(icon) = tlw.get_icon() {
                dialog_builder = dialog_builder.icon(&icon);
            }
            dialog_builder = dialog_builder.parent(&tlw);
        };

        dialog_builder
    }

    fn new_dialog_builder(&self) -> gtk::DialogBuilder {
        let mut dialog_builder = gtk::DialogBuilder::new();
        if let Some(tlw) = self.get_toplevel_gtk_window() {
            if let Some(icon) = tlw.get_icon() {
                dialog_builder = dialog_builder.icon(&icon);
            }
            dialog_builder = dialog_builder.parent(&tlw);
        };

        dialog_builder
    }

    fn new_file_chooser_dialog_builder(&self) -> gtk::FileChooserDialogBuilder {
        let mut dialog_builder = gtk::FileChooserDialogBuilder::new();
        if let Some(tlw) = self.get_toplevel_gtk_window() {
            if let Some(icon) = tlw.get_icon() {
                dialog_builder = dialog_builder.icon(&icon);
            }
            dialog_builder = dialog_builder.parent(&tlw);
        };

        dialog_builder
    }

    fn new_font_chooser_dialog_builder(&self) -> gtk::FontChooserDialogBuilder {
        let mut dialog_builder = gtk::FontChooserDialogBuilder::new();
        if let Some(tlw) = self.get_toplevel_gtk_window() {
            if let Some(icon) = tlw.get_icon() {
                dialog_builder = dialog_builder.icon(&icon);
            }
            dialog_builder = dialog_builder.parent(&tlw);
        };

        dialog_builder
    }

    fn new_message_dialog_builder(&self) -> gtk::MessageDialogBuilder {
        let mut dialog_builder = gtk::MessageDialogBuilder::new();
        if let Some(tlw) = self.get_toplevel_gtk_window() {
            if let Some(icon) = tlw.get_icon() {
                dialog_builder = dialog_builder.icon(&icon);
            }
            dialog_builder = dialog_builder.parent(&tlw);
        };

        dialog_builder
    }

    fn new_recent_chooser_dialog_builder(&self) -> gtk::RecentChooserDialogBuilder {
        let mut dialog_builder = gtk::RecentChooserDialogBuilder::new();
        if let Some(tlw) = self.get_toplevel_gtk_window() {
            if let Some(icon) = tlw.get_icon() {
                dialog_builder = dialog_builder.icon(&icon);
            }
            dialog_builder = dialog_builder.parent(&tlw);
        };

        dialog_builder
    }

    fn message_and_explanation(&self, msg: &str, expln: Option<&str>, msg_type: gtk::MessageType) {
        let mut builder = self.new_message_dialog_builder();
        if let Some(expln) = expln {
            builder = builder.secondary_text(expln);
        };
        let dialog = builder
            .text(msg)
            .message_type(msg_type)
            .buttons(gtk::ButtonsType::Close)
            .window_position(gtk::WindowPosition::Mouse)
            .build();
        dialog.run();
        dialog.close();
    }

    fn inform_user(&self, msg: &str, expln: Option<&str>) {
        self.message_and_explanation(msg, expln, gtk::MessageType::Info)
    }

    fn warn_user(&self, msg: &str, expln: Option<&str>) {
        self.message_and_explanation(msg, expln, gtk::MessageType::Warning)
    }

    fn report_error<E: Error>(&self, msg: &str, error: &E) {
        let mut expln = error.to_string();
        if let Some(source) = error.source() {
            expln += &format!("\nCaused by: {}.", source);
        };
        self.message_and_explanation(msg, Some(&expln), gtk::MessageType::Error)
    }

    fn report_command_result(&self, cmd: &str, result: &io::Result<process::Output>) {
        match result {
            Ok(output) => {
                let (msg_type, markup) = if !output.status.success() {
                    (gtk::MessageType::Error, markup_cmd_fail(cmd))
                } else if !output.stderr.is_empty() {
                    (gtk::MessageType::Warning, markup_cmd_warn(cmd))
                } else {
                    (gtk::MessageType::Info, markup_cmd_ok(cmd))
                };
                let mut builder = self.new_message_dialog_builder();
                if let Some(markup) = markup_output(output) {
                    builder = builder
                        .secondary_use_markup(true)
                        .secondary_text(markup.as_str());
                }
                let dialog = builder
                    .message_type(msg_type)
                    .buttons(gtk::ButtonsType::Close)
                    .text(&markup)
                    .build();
                dialog.run();
                dialog.close();
            }
            Err(err) => {
                let msg = format!("{}: blew up!", cmd);
                self.report_error(&msg, err)
            }
        }
    }

    fn report_any_command_problems(&self, cmd: &str, result: &io::Result<process::Output>) {
        match result {
            Ok(output) => {
                if output.status.success() && output.stderr.is_empty() {
                    // Nothing to report
                    return;
                }
                let (msg_type, markup) = if !output.status.success() {
                    (gtk::MessageType::Error, markup_cmd_fail(cmd))
                } else if !output.stderr.is_empty() {
                    (gtk::MessageType::Warning, markup_cmd_warn(cmd))
                } else {
                    (gtk::MessageType::Info, markup_cmd_ok(cmd))
                };
                let mut builder = self.new_message_dialog_builder();
                if let Some(markup) = markup_output(output) {
                    builder = builder
                        .secondary_use_markup(true)
                        .secondary_text(markup.as_str());
                }
                let dialog = builder
                    .message_type(msg_type)
                    .buttons(gtk::ButtonsType::Close)
                    .text(&markup)
                    .build();
                dialog.run();
                dialog.close();
            }
            Err(err) => {
                let msg = format!("{}: blew up!", cmd);
                self.report_error(&msg, err)
            }
        }
    }

    fn ask_question(
        &self,
        question: &str,
        expln: Option<&str>,
        buttons: &[(&'static str, gtk::ResponseType)],
    ) -> gtk::ResponseType {
        let mut builder = self.new_message_dialog_builder();
        if let Some(expln) = expln {
            builder = builder.secondary_text(expln);
        };
        let dialog = builder
            .message_type(gtk::MessageType::Question)
            .text(question)
            .build();
        for button in buttons {
            dialog.add_button(button.0, button.1);
        }
        let response = dialog.run();
        dialog.hide();
        response
    }

    fn ask_confirm_action(&self, msg: &str, expln: Option<&str>) -> bool {
        self.ask_question(msg, expln, &Self::CANCEL_OK_BUTTONS) == gtk::ResponseType::Ok
    }

    fn ask_string_cancel_or_ok(&self, question: &str) -> (gtk::ResponseType, Option<String>) {
        let dialog = self.new_dialog_builder().destroy_with_parent(true).build();
        dialog.set_default_response(gtk::ResponseType::Ok);
        let h_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        h_box.pack_start(&gtk::Label::new(Some(question)), false, false, 2);
        let entry = gtk::Entry::new();
        h_box.pack_start(&entry, true, true, 2);
        dialog.get_content_area().pack_start(&h_box, true, true, 0);
        dialog.show_all();
        entry.set_activates_default(true);
        let response = dialog.run();
        dialog.hide();
        if response == gtk::ResponseType::Ok {
            let gtext = entry.get_text();
            (response, Some(gtext.to_string()))
        } else {
            (response, None)
        }
    }

    fn new_file_chooser_dialog(
        &self,
        o_title: Option<&str>,
        action: gtk::FileChooserAction,
    ) -> gtk::FileChooserDialog {
        let dialog = gtk::FileChooserDialog::new(o_title, parent_none(), action);
        self.set_transient_for_and_icon_on(&dialog);
        dialog
    }

    fn present_widget_cancel_or_ok<W: IsA<gtk::Widget>>(&self, widget: &W) -> gtk::ResponseType {
        let dialog = self
            .new_dialog_builder()
            .window_position(gtk::WindowPosition::Mouse)
            .build();
        dialog
            .get_content_area()
            .pack_start(widget, false, false, 0);
        for button in &Self::CANCEL_OK_BUTTONS {
            dialog.add_button(button.0, button.1);
        }
        dialog.show_all();
        let response = dialog.run();
        dialog.close();
        crate::yield_to_pending_events!();
        response
    }

    fn browse_path(
        &self,
        prompt: Option<&str>,
        suggestion: Option<&str>,
        action: gtk::FileChooserAction,
        absolute: bool,
    ) -> Option<PathBuf> {
        let mut builder = self.new_file_chooser_dialog_builder();
        if let Some(prompt) = prompt {
            builder = builder.title(prompt);
        }
        let dialog = builder.action(action).build();
        //let dialog = gtk::FileChooserDialog::new(o_prompt, dialog_parent, action);
        for button in CANCEL_OK_BUTTONS {
            dialog.add_button(button.0, button.1);
        }
        dialog.set_default_response(gtk::ResponseType::Ok);
        if let Some(suggestion) = suggestion {
            dialog.set_filename(suggestion);
        };
        if dialog.run() == gtk::ResponseType::Ok {
            if let Some(file_path) = dialog.get_filename() {
                dialog.hide();
                if absolute {
                    match path_utilities::absolute_pathbuf(&file_path) {
                        Some(pathbuf) => Some(pathbuf),
                        None => Some(file_path),
                    }
                } else {
                    Some(path_utilities::relative_pathbuf_or_mine(&file_path))
                }
            } else {
                dialog.hide();
                None
            }
        } else {
            dialog.hide();
            None
        }
    }

    fn ask_path(
        &self,
        prompt: Option<&str>,
        suggestion: Option<&str>,
        action: gtk::FileChooserAction,
    ) -> Option<PathBuf> {
        let dialog = self.new_dialog_builder().destroy_with_parent(true).build();
        for button in Self::CANCEL_OK_BUTTONS.iter() {
            dialog.add_button(button.0, button.1);
        }
        dialog.connect_close(|d| unsafe { d.destroy() });
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 1);
        dialog.get_content_area().pack_start(&hbox, false, false, 0);

        let prompt_label = if prompt.is_some() {
            gtk::Label::new(prompt)
        } else {
            gtk::Label::new(Some("File Path:"))
        };
        hbox.pack_start(&prompt_label, false, false, 0);

        let entry = gtk::Entry::new();
        match action {
            gtk::FileChooserAction::Open | gtk::FileChooserAction::Save => {
                entry.enable_file_path_completion();
            }
            gtk::FileChooserAction::SelectFolder | gtk::FileChooserAction::CreateFolder => {
                entry.enable_dir_path_completion();
            }
            _ => panic!("Must specify a (useful) action"),
        };
        entry.set_activates_default(true);
        entry.set_width_chars(32);
        if let Some(suggestion) = suggestion {
            entry.set_text(suggestion)
        };
        hbox.pack_start(&entry, true, true, 0);

        let button = gtk::Button::with_label("Browse");
        hbox.pack_start(&button, false, false, 0);
        hbox.show_all();
        let b_prompt = if let Some(prompt_text) = prompt {
            format!("Select {}", prompt_text)
        } else {
            "Select Path:".to_string()
        };
        let entry_c = entry.clone();
        let dialog_c = dialog.clone();
        button.connect_clicked(move |_| {
            // NB: following gymnastics need to satisfy lifetime  checks
            //let text = &entry_c.get_text().unwrap_or("".to_string());
            //let suggestion: Option<&str> = if text.len() > 0 { Some(text) } else { None };
            let suggestion_str = String::from(entry_c.get_text());
            let suggestion: Option<&str> = Some(&suggestion_str);
            if let Some(path) =
                //browse_path(Some(&dialog_c), Some(&b_prompt), suggestion, action, false)
                dialog_c.browse_path(Some(&b_prompt), suggestion, action, false)
            {
                let text = path_utilities::path_to_string(&path);
                entry_c.set_text(&text);
            }
        });

        dialog.set_default_response(gtk::ResponseType::Ok);
        if dialog.run() == gtk::ResponseType::Ok {
            let text = String::from(entry.get_text());
            unsafe { dialog.destroy() };
            Some(PathBuf::from(&text))
        } else {
            unsafe { dialog.destroy() };
            None
        }
    }

    fn select_dir(
        &self,
        o_prompt: Option<&str>,
        o_suggestion: Option<&str>,
        existing: bool,
        absolute: bool,
    ) -> Option<PathBuf> {
        if existing {
            self.browse_path(
                o_prompt,
                o_suggestion,
                gtk::FileChooserAction::SelectFolder,
                absolute,
            )
        } else {
            self.browse_path(
                o_prompt,
                o_suggestion,
                gtk::FileChooserAction::CreateFolder,
                absolute,
            )
        }
    }

    fn select_file(
        &self,
        o_prompt: Option<&str>,
        o_suggestion: Option<&str>,
        existing: bool,
        absolute: bool,
    ) -> Option<PathBuf> {
        if existing {
            self.browse_path(
                o_prompt,
                o_suggestion,
                gtk::FileChooserAction::Open,
                absolute,
            )
        } else {
            self.browse_path(
                o_prompt,
                o_suggestion,
                gtk::FileChooserAction::Save,
                absolute,
            )
        }
    }

    fn ask_dir_path(
        &self,
        o_prompt: Option<&str>,
        o_suggestion: Option<&str>,
        existing: bool,
    ) -> Option<PathBuf> {
        if existing {
            self.ask_path(o_prompt, o_suggestion, gtk::FileChooserAction::SelectFolder)
        } else {
            self.ask_path(o_prompt, o_suggestion, gtk::FileChooserAction::CreateFolder)
        }
    }

    fn ask_file_path(
        &self,
        o_prompt: Option<&str>,
        o_suggestion: Option<&str>,
        existing: bool,
    ) -> Option<PathBuf> {
        if existing {
            self.ask_path(o_prompt, o_suggestion, gtk::FileChooserAction::Open)
        } else {
            self.ask_path(o_prompt, o_suggestion, gtk::FileChooserAction::Save)
        }
    }

    fn browse_file_path(
        &self,
        o_prompt: Option<&str>,
        o_suggestion: Option<&str>,
        existing: bool,
        absolute: bool,
    ) -> Option<PathBuf> {
        if existing {
            self.browse_path(
                o_prompt,
                o_suggestion,
                gtk::FileChooserAction::Open,
                absolute,
            )
        } else {
            self.browse_path(
                o_prompt,
                o_suggestion,
                gtk::FileChooserAction::Save,
                absolute,
            )
        }
    }
}

impl DialogUser for gtk::Bin {}
impl DialogUser for gtk::DrawingArea {}
impl DialogUser for gtk::EventBox {}
impl DialogUser for gtk::Frame {}
impl DialogUser for gtk::Notebook {}
impl DialogUser for gtk::ScrolledWindow {}
impl DialogUser for gtk::TextView {}
impl DialogUser for gtk::TreeView {}
impl DialogUser for gtk::Window {}
impl DialogUser for gtk::ApplicationWindow {}
impl DialogUser for gtk::Dialog {}
impl DialogUser for gtk::AboutDialog {}
impl DialogUser for gtk::AppChooserDialog {}
impl DialogUser for gtk::ColorChooserDialog {}
impl DialogUser for gtk::FileChooserDialog {}
impl DialogUser for gtk::FontChooserDialog {}
impl DialogUser for gtk::MessageDialog {}
impl DialogUser for gtk::RecentChooserDialog {}
