// Copyright 2018 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gdk;
use gtk;
use gtk::prelude::*;

use crate::{sav_state::*, wrapper::*};

#[derive(Debug, Clone)]
pub struct MenuItemSpec {
    label: String,
    image: Option<gtk::Image>,
    tooltip_text: Option<String>,
}

impl From<(&str, Option<gtk::Image>, Option<&str>)> for MenuItemSpec {
    fn from(tuple_: (&str, Option<gtk::Image>, Option<&str>)) -> Self {
        Self {
            label: tuple_.0.to_string(),
            image: tuple_.1,
            tooltip_text: tuple_.2.map(|text| text.to_string()),
        }
    }
}

impl From<&MenuItemSpec> for gtk::MenuItem {
    fn from(menu_item_spec: &MenuItemSpec) -> Self {
        let item = gtk::MenuItem::new();
        let h_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        if let Some(image) = &menu_item_spec.image {
            h_box.pack_start(image, false, false, 0);
        }
        let label = gtk::Label::new(Some(&menu_item_spec.label));
        label.set_xalign(0.0);
        h_box.pack_start(&label, true, true, 0);
        item.add(&h_box);
        item.set_tooltip_text(if let Some(string) = &menu_item_spec.tooltip_text {
            Some(string)
        } else {
            None
        });

        item
    }
}

#[derive(PWO)]
pub struct ManagedMenu {
    menu: gtk::Menu,
    items: Rc<ConditionalWidgets<&'static str, gtk::MenuItem>>,
}

impl ManagedMenu {
    pub fn menu(&self) -> gtk::Menu {
        self.menu.clone()
    }

    pub fn menu_item(&self, name: &'static str) -> Option<gtk::MenuItem> {
        self.items.get_widget(&name)
    }

    pub fn append_menu_item(&self, name: &'static str, item: &gtk::MenuItem, condns: u64) {
        self.items.add_widget(name, item, condns);
        self.menu.append(item);
        self.menu.show_all();
    }

    pub fn insert_menu_item(
        &self,
        name: &'static str,
        item: &gtk::MenuItem,
        condns: u64,
        position: i32,
    ) {
        self.items.add_widget(name, item, condns);
        self.menu.insert(item, position);
        self.menu.show_all();
    }

    pub fn prepend_menu_item(&self, name: &'static str, item: &gtk::MenuItem, condns: u64) {
        self.items.add_widget(name, item, condns);
        self.menu.prepend(item);
        self.menu.show_all();
    }

    pub fn append_item(
        &self,
        name: &'static str,
        menu_item_spec: &MenuItemSpec,
        condns: u64,
    ) -> gtk::MenuItem {
        let item = menu_item_spec.into();
        self.append_menu_item(name, &item, condns);

        item
    }

    pub fn insert_item(
        &self,
        name: &'static str,
        menu_item_spec: &MenuItemSpec,
        condns: u64,
        position: i32,
    ) -> gtk::MenuItem {
        let item = menu_item_spec.into();
        self.insert_menu_item(name, &item, condns, position);

        item
    }

    pub fn prepend_item(
        &self,
        name: &'static str,
        menu_item_spec: &MenuItemSpec,
        condns: u64,
    ) -> gtk::MenuItem {
        let item = menu_item_spec.into();
        self.prepend_menu_item(name, &item, condns);

        item
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
        if self.items.len() > 0 {
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

    pub fn change_notifier(&mut self, change_notifier: &Rc<ChangedCondnsNotifier>) -> &mut Self {
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
            mm.append_item(name, menu_item_spec, *condns);
        }
        mm.menu.show_all();

        mm
    }
}

#[derive(PWO)]
pub struct WrappedMenu {
    menu: gtk::Menu,
    items: RefCell<HashMap<String, gtk::MenuItem>>,
}

impl WrappedMenu {
    pub fn menu(&self) -> gtk::Menu {
        self.menu.clone()
    }

    pub fn menu_item(&self, name: &str) -> Option<gtk::MenuItem> {
        self.items.borrow().get(name).cloned()
    }

    pub fn append_menu_item(&self, name: &str, item: &gtk::MenuItem) {
        if self
            .items
            .borrow_mut()
            .insert(name.to_string(), item.clone())
            .is_some()
        {
            panic!("Duplicate popup menu item name: {}", name);
        };
        self.menu.append(item);
        self.menu.show_all();
    }

    pub fn insert_menu_item(&self, name: &str, item: &gtk::MenuItem, position: i32) {
        if self
            .items
            .borrow_mut()
            .insert(name.to_string(), item.clone())
            .is_some()
        {
            panic!("Duplicate popup menu item name: {}", name);
        };
        self.menu.insert(item, position);
        self.menu.show_all();
    }

    pub fn prepend_menu_item(&self, name: &str, item: &gtk::MenuItem) {
        if self
            .items
            .borrow_mut()
            .insert(name.to_string(), item.clone())
            .is_some()
        {
            panic!("Duplicate popup menu item name: {}", name);
        };
        self.menu.prepend(item);
        self.menu.show_all();
    }

    pub fn append_item(&self, name: &str, menu_item_spec: &MenuItemSpec) -> gtk::MenuItem {
        let item = menu_item_spec.into();
        self.append_menu_item(name, &item);

        item
    }

    pub fn insert_item(
        &self,
        name: &str,
        menu_item_spec: &MenuItemSpec,
        position: i32,
    ) -> gtk::MenuItem {
        let item = menu_item_spec.into();
        self.insert_menu_item(name, &item, position);

        item
    }

    pub fn prepend_item(&self, name: &str, menu_item_spec: &MenuItemSpec) -> gtk::MenuItem {
        let item = menu_item_spec.into();
        self.prepend_menu_item(name, &item);

        item
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

    pub fn connect_item_activate<F: Fn(&gtk::MenuItem) + 'static>(&self, name: &str, f: F) {
        if let Some(item) = self.items.borrow().get(name) {
            item.connect_activate(f);
        } else {
            panic!("Unknown popup menu item name: {}", name);
        }
    }

    pub fn set_sensitivities(&self, sensitivity: bool, names: &[&str]) {
        for name in names.iter() {
            if let Some(item) = self.items.borrow().get(*name) {
                item.set_sensitive(sensitivity);
            } else {
                panic!("Unknown popup menu item name: {}", name);
            }
        }
    }

    pub fn set_visibilities(&self, visibility: bool, names: &[&str]) {
        for name in names.iter() {
            if let Some(item) = self.items.borrow().get(*name) {
                item.set_visible(visibility);
            } else {
                panic!("Unknown popup menu item name: {}", name);
            }
        }
    }

    pub fn popup_at_event(&self, event: &gdk::EventButton) {
        if self.items.borrow().len() > 0 {
            self.menu.popup_easy(event.get_button(), event.get_time());
        }
    }
}

#[derive(Default)]
pub struct WrappedMenuBuilder {
    items: Vec<(&'static str, MenuItemSpec)>,
}

impl WrappedMenuBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(&mut self, items: &[(&'static str, MenuItemSpec)]) -> &Self {
        self.items = items.to_vec();
        self
    }

    pub fn build(self) -> WrappedMenu {
        let pm = WrappedMenu {
            menu: gtk::Menu::new(),
            items: RefCell::new(HashMap::<String, gtk::MenuItem>::new()),
        };
        for (name, menu_item_spec) in self.items.iter() {
            pm.append_item(name, menu_item_spec);
        }
        pm.menu.show_all();

        pm
    }
}

pub struct SplitManagedMenu {
    menu: gtk::Menu,
    sensitivity: Rc<ConditionalWidgetGroups<gtk::MenuItem>>,
    visibility: Rc<ConditionalWidgetGroups<gtk::MenuItem>>,
}

impl SplitManagedMenu {
    pub fn menu(&self) -> gtk::Menu {
        self.menu.clone()
    }

    pub fn menu_item(&self, name: &str) -> Option<gtk::MenuItem> {
        self.sensitivity.get_widget(name)
    }

    pub fn append_menu_item(
        &self,
        name: &str,
        item: &gtk::MenuItem,
        sensitivity_condns: u64,
        visibility_condns: u64,
    ) {
        self.sensitivity.add_widget(name, item, sensitivity_condns);
        self.visibility.add_widget(name, item, visibility_condns);
        self.menu.append(item);
        self.menu.show_all();
    }

    pub fn insert_menu_item(
        &self,
        name: &str,
        item: &gtk::MenuItem,
        sensitivity_condns: u64,
        visibility_condns: u64,
        position: i32,
    ) {
        self.sensitivity.add_widget(name, item, sensitivity_condns);
        self.visibility.add_widget(name, item, visibility_condns);
        self.menu.insert(item, position);
        self.menu.show_all();
    }

    pub fn prepend_menu_item(
        &self,
        name: &str,
        item: &gtk::MenuItem,
        sensitivity_condns: u64,
        visibility_condns: u64,
    ) {
        self.sensitivity.add_widget(name, item, sensitivity_condns);
        self.visibility.add_widget(name, item, visibility_condns);
        self.menu.prepend(item);
        self.menu.show_all();
    }

    pub fn append_item(
        &self,
        name: &str,
        menu_item_spec: &MenuItemSpec,
        sensitivity_condns: u64,
        visibility_condns: u64,
    ) -> gtk::MenuItem {
        let item = menu_item_spec.into();
        self.append_menu_item(name, &item, sensitivity_condns, visibility_condns);

        item
    }

    pub fn insert_item(
        &self,
        name: &str,
        menu_item_spec: &MenuItemSpec,
        sensitivity_condns: u64,
        visibility_condns: u64,
        position: i32,
    ) -> gtk::MenuItem {
        let item = menu_item_spec.into();
        self.insert_menu_item(name, &item, sensitivity_condns, visibility_condns, position);

        item
    }

    pub fn prepend_item(
        &self,
        name: &str,
        menu_item_spec: &MenuItemSpec,
        sensitivity_condns: u64,
        visibility_condns: u64,
    ) -> gtk::MenuItem {
        let item = menu_item_spec.into();
        self.prepend_menu_item(name, &item, sensitivity_condns, visibility_condns);

        item
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
        self.sensitivity.update_condns(changed_condns);
        self.visibility.update_condns(changed_condns);
    }

    pub fn update_hover_condns(&self, hover_ok: bool) {
        self.sensitivity.update_hover_condns(hover_ok);
        self.visibility.update_hover_condns(hover_ok);
    }

    pub fn popup_at_event(&self, event: &gdk::EventButton) {
        if self.sensitivity.len() > 0 {
            self.menu.popup_easy(event.get_button(), event.get_time());
        }
    }
}

#[derive(Default)]
pub struct SplitManagedMenuBuilder {
    selection: Option<gtk::TreeSelection>,
    change_notifier: Option<Rc<ChangedCondnsNotifier>>,
    items: Vec<(&'static str, MenuItemSpec, u64, u64)>,
}

impl SplitManagedMenuBuilder {
    pub fn build(&self) -> SplitManagedMenu {
        let menu = gtk::MenuBuilder::new().build();
        let sensitivity = ConditionalWidgetGroups::<gtk::MenuItem>::new(
            WidgetStatesControlled::Sensitivity,
            if let Some(selection) = &self.selection {
                Some(selection)
            } else {
                None
            },
            if let Some(change_notifier) = &self.change_notifier {
                Some(change_notifier)
            } else {
                None
            },
        );
        let visibility = ConditionalWidgetGroups::<gtk::MenuItem>::new(
            WidgetStatesControlled::Visibility,
            if let Some(selection) = &self.selection {
                Some(selection)
            } else {
                None
            },
            if let Some(change_notifier) = &self.change_notifier {
                Some(change_notifier)
            } else {
                None
            },
        );
        let smm = SplitManagedMenu {
            menu,
            sensitivity,
            visibility,
        };
        for (name, menu_item_spec, sensitivity_condns, visibility_condns) in self.items.iter() {
            smm.append_item(
                name,
                menu_item_spec,
                *sensitivity_condns,
                *visibility_condns,
            );
        }
        smm.menu.show_all();

        smm
    }
}
