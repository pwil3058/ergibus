// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::gtk::prelude::*;

use crate::{
    sav_state::{
        self, ChangedCondnsNotifier, ConditionalWidgets, ConditionalWidgetsBuilder, MaskedCondns,
        WidgetStatesControlled,
    },
    wrapper::*,
};

#[derive(Debug, Clone)]
pub struct MenuItemSpec(
    pub &'static str,
    pub Option<gtk::Image>,
    pub Option<&'static str>,
);

impl From<(&'static str, Option<gtk::Image>, Option<&'static str>)> for MenuItemSpec {
    fn from(tuple_: (&'static str, Option<gtk::Image>, Option<&'static str>)) -> Self {
        Self(tuple_.0, tuple_.1, tuple_.2)
    }
}

impl From<&MenuItemSpec> for gtk::MenuItem {
    fn from(menu_item_spec: &MenuItemSpec) -> Self {
        let h_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        if let Some(image) = &menu_item_spec.1 {
            h_box.pack_start(image, false, false, 0);
        }
        let label = gtk::LabelBuilder::new()
            .label(menu_item_spec.0)
            .xalign(0.0)
            .build();
        h_box.pack_start(&label, true, true, 0);
        let item = gtk::MenuItemBuilder::new().child(&h_box).build();
        item.set_tooltip_text(menu_item_spec.2);

        item
    }
}

#[derive(PWO)]
pub struct ManagedMenu {
    menu: gtk::Menu,
    items: ConditionalWidgets<&'static str, gtk::MenuItem>,
}

impl ManagedMenu {
    pub fn menu(&self) -> gtk::Menu {
        self.menu.clone()
    }

    pub fn menu_item(&self, name: &'static str) -> Result<gtk::MenuItem, sav_state::Error> {
        self.items.get_widget(&name)
    }

    pub fn append_menu_item(
        &self,
        name: &'static str,
        item: &gtk::MenuItem,
        condns: u64,
    ) -> Result<(), sav_state::Error> {
        self.items.add_widget(name, item, condns)?;
        self.menu.append(item);
        self.menu.show_all();
        Ok(())
    }

    pub fn insert_menu_item(
        &self,
        name: &'static str,
        item: &gtk::MenuItem,
        condns: u64,
        position: i32,
    ) -> Result<(), sav_state::Error> {
        self.items.add_widget(name, item, condns)?;
        self.menu.insert(item, position);
        self.menu.show_all();
        Ok(())
    }

    pub fn prepend_menu_item(
        &self,
        name: &'static str,
        item: &gtk::MenuItem,
        condns: u64,
    ) -> Result<(), sav_state::Error> {
        self.items.add_widget(name, item, condns)?;
        self.menu.prepend(item);
        self.menu.show_all();
        Ok(())
    }

    pub fn append_item(
        &self,
        name: &'static str,
        menu_item_spec: &MenuItemSpec,
        condns: u64,
    ) -> Result<gtk::MenuItem, sav_state::Error> {
        let item = menu_item_spec.into();
        self.append_menu_item(name, &item, condns)?;

        Ok(item)
    }

    pub fn insert_item(
        &self,
        name: &'static str,
        menu_item_spec: &MenuItemSpec,
        condns: u64,
        position: i32,
    ) -> Result<gtk::MenuItem, sav_state::Error> {
        let item = menu_item_spec.into();
        self.insert_menu_item(name, &item, condns, position)?;

        Ok(item)
    }

    pub fn prepend_item(
        &self,
        name: &'static str,
        menu_item_spec: &MenuItemSpec,
        condns: u64,
    ) -> Result<gtk::MenuItem, sav_state::Error> {
        let item = menu_item_spec.into();
        self.prepend_menu_item(name, &item, condns)?;

        Ok(item)
    }

    pub fn append_separator(&self) {
        self.menu.append(&gtk::SeparatorMenuItem::new());
        self.menu.show_all();
    }

    pub fn insert_separator(&self, position: i32) {
        self.menu.insert(&gtk::SeparatorMenuItem::new(), position);
        self.menu.show_all();
    }

    pub fn prepend_separator(&self) {
        self.menu.prepend(&gtk::SeparatorMenuItem::new());
        self.menu.show_all();
    }

    pub fn update_condns(&self, changed_condns: MaskedCondns) {
        self.items.update_condns(changed_condns)
    }

    pub fn update_hover_condns(&self, hover_ok: bool) {
        self.items.update_hover_condns(hover_ok)
    }

    pub fn popup_at_event(&self, event: &gdk::EventButton) {
        if !self.items.is_empty() {
            self.menu.popup_easy(event.get_button(), event.get_time());
        }
    }
}

#[derive(Default)]
pub struct ManagedMenuBuilder {
    conditional_widgets_builder: ConditionalWidgetsBuilder,
    items: Vec<(&'static str, MenuItemSpec, u64)>,
}

impl ManagedMenuBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn widget_states_controlled(
        &mut self,
        widget_states_controlled: WidgetStatesControlled,
    ) -> &mut Self {
        self.conditional_widgets_builder
            .widget_states_controlled(widget_states_controlled);
        self
    }

    pub fn selection(&mut self, selection: &gtk::TreeSelection) -> &mut Self {
        self.conditional_widgets_builder.selection(selection);
        self
    }

    pub fn change_notifier(&mut self, change_notifier: &ChangedCondnsNotifier) -> &mut Self {
        self.conditional_widgets_builder
            .change_notifier(change_notifier);
        self
    }

    pub fn items(&mut self, items: &[(&'static str, MenuItemSpec, u64)]) -> &Self {
        self.items = items.to_vec();
        self
    }

    pub fn build(&self) -> ManagedMenu {
        let menu = gtk::MenuBuilder::new().build();
        let items = self
            .conditional_widgets_builder
            .build::<&'static str, gtk::MenuItem>();
        let mm = ManagedMenu { menu, items };
        for (name, menu_item_spec, condns) in self.items.iter() {
            if let Err(err) = mm.append_item(name, menu_item_spec, *condns) {
                panic!("Error adding item '{}' to menu: {}", name, err);
            };
        }
        mm.menu.show_all();

        mm
    }
}
