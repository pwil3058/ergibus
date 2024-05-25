// Copyright 2018 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gdk;
use gtk;
use gtk::prelude::*;

use crate::sav_state::*;

pub struct WrappedMenu {
    menu: gtk::Menu,
    items: RefCell<HashMap<String, gtk::MenuItem>>,
}

impl WrappedMenu {
    pub fn new(items: &[(&str, &str, &str)]) -> WrappedMenu {
        let pm = WrappedMenu {
            menu: gtk::Menu::new(),
            items: RefCell::new(HashMap::<String, gtk::MenuItem>::new()),
        };
        for &(name, label_text, tooltip_text) in items.iter() {
            pm.append_item(name, label_text, tooltip_text);
        }
        pm.menu.show_all();

        pm
    }

    pub fn len(&self) -> usize {
        self.items.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

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

    fn new_item(&self, label_text: &str, tooltip_text: &str) -> gtk::MenuItem {
        let item = gtk::MenuItem::with_label(label_text);
        item.set_tooltip_text(Some(tooltip_text));

        item
    }

    pub fn append_item(&self, name: &str, label_text: &str, tooltip_text: &str) -> gtk::MenuItem {
        let item = self.new_item(label_text, tooltip_text);
        self.append_menu_item(name, &item);

        item
    }

    pub fn insert_item(
        &self,
        name: &str,
        label_text: &str,
        tooltip_text: &str,
        position: i32,
    ) -> gtk::MenuItem {
        let item = self.new_item(label_text, tooltip_text);
        self.insert_menu_item(name, &item, position);

        item
    }

    pub fn prepend_item(&self, name: &str, label_text: &str, tooltip_text: &str) -> gtk::MenuItem {
        let item = self.new_item(label_text, tooltip_text);
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
        if !self.is_empty() {
            self.menu.popup_easy(event.get_button(), event.get_time());
        }
    }
}

#[derive(Default)]
pub struct WrappedMenuBuilder<'a> {
    items: &'a [(&'a str, &'a str, &'a str)],
}

impl<'a> WrappedMenuBuilder<'a> {
    pub fn new() -> Self {
        Self { items: &[] }
    }

    pub fn items(&mut self, items: &'a [(&'a str, &'a str, &'a str)]) -> &Self {
        self.items = items;
        self
    }

    pub fn build(self) -> WrappedMenu {
        let pm = WrappedMenu {
            menu: gtk::Menu::new(),
            items: RefCell::new(HashMap::<String, gtk::MenuItem>::new()),
        };
        for &(name, label_text, tooltip_text) in self.items.iter() {
            pm.append_item(name, label_text, tooltip_text);
        }
        pm.menu.show_all();

        pm
    }
}

#[derive(Debug, Clone)]
pub struct MenuItemSpec {
    name: String,
    label: String,
    image: Option<gtk::Image>,
    tooltip: String,
    condns: u64,
}

impl MenuItemSpec {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn image(&self) -> Option<&gtk::Image> {
        if let Some(ref image) = self.image {
            Some(image)
        } else {
            None
        }
    }

    pub fn tooltip(&self) -> &str {
        &self.tooltip
    }

    pub fn condns(&self) -> u64 {
        self.condns
    }
}

impl From<(&str, &str, Option<gtk::Image>, &str, u64)> for MenuItemSpec {
    fn from(tuple_: (&str, &str, Option<gtk::Image>, &str, u64)) -> Self {
        Self {
            name: tuple_.0.to_string(),
            label: tuple_.1.to_string(),
            image: tuple_.2,
            tooltip: tuple_.3.to_string(),
            condns: tuple_.4,
        }
    }
}

pub struct ManagedMenu {
    menu: gtk::Menu,
    items: Rc<ConditionalWidgetGroups<gtk::MenuItem>>,
}

impl ManagedMenu {
    pub fn new(
        wsc: WidgetStatesControlled,
        selection: Option<&gtk::TreeSelection>,
        change_notifier: Option<&Rc<ChangedCondnsNotifier>>,
        items: &[(&str, &str, Option<&gtk::Image>, &str, u64)],
    ) -> Self {
        let pm = Self {
            menu: gtk::Menu::new(),
            items: ConditionalWidgetGroups::<gtk::MenuItem>::new(wsc, selection, change_notifier),
        };
        for &(name, label_text, image, tooltip_text, condns) in items.iter() {
            pm.append_item(name, label_text, image, tooltip_text, condns);
        }
        pm.menu.show_all();

        pm
    }

    pub fn menu(&self) -> gtk::Menu {
        self.menu.clone()
    }

    pub fn menu_item(&self, name: &str) -> Option<gtk::MenuItem> {
        self.items.get_widget(name)
    }

    fn append_menu_item(&self, name: &str, item: &gtk::MenuItem, condns: u64) {
        self.items.add_widget(name, item, condns);
        self.menu.append(item);
        self.menu.show_all();
    }

    fn insert_menu_item(&self, name: &str, item: &gtk::MenuItem, condns: u64, position: i32) {
        self.items.add_widget(name, item, condns);
        self.menu.insert(item, position);
        self.menu.show_all();
    }

    fn prepend_menu_item(&self, name: &str, item: &gtk::MenuItem, condns: u64) {
        self.items.add_widget(name, item, condns);
        self.menu.prepend(item);
        self.menu.show_all();
    }

    fn new_item(
        &self,
        label_text: &str,
        image: Option<&gtk::Image>,
        tooltip_text: &str,
    ) -> gtk::MenuItem {
        let item = gtk::MenuItem::new();
        let h_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        if let Some(image) = image {
            h_box.pack_start(image, false, false, 0);
        }
        let label = gtk::Label::new(Some(label_text));
        label.set_xalign(0.0);
        h_box.pack_start(&label, true, true, 0);
        item.add(&h_box);
        item.set_tooltip_text(Some(tooltip_text));

        item
    }

    pub fn append_item(
        &self,
        name: &str,
        label_text: &str,
        image: Option<&gtk::Image>,
        tooltip_text: &str,
        condns: u64,
    ) -> gtk::MenuItem {
        let item = self.new_item(label_text, image, tooltip_text);
        self.append_menu_item(name, &item, condns);

        item
    }

    pub fn insert_item(
        &self,
        name: &str,
        label_text: &str,
        image: Option<&gtk::Image>,
        tooltip_text: &str,
        condns: u64,
        position: i32,
    ) -> gtk::MenuItem {
        let item = self.new_item(label_text, image, tooltip_text);
        self.insert_menu_item(name, &item, condns, position);

        item
    }

    pub fn prepend_item(
        &self,
        name: &str,
        label_text: &str,
        image: Option<&gtk::Image>,
        tooltip_text: &str,
        condns: u64,
    ) -> gtk::MenuItem {
        let item = self.new_item(label_text, image, tooltip_text);
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
pub struct ManagedMenuBuilder<'a, 'b, 'c> {
    wsc: WidgetStatesControlled,
    selection: Option<&'a gtk::TreeSelection>,
    change_notifier: Option<&'b Rc<ChangedCondnsNotifier>>,
    items: &'c [(&'c str, &'c str, Option<&'c gtk::Image>, &'c str, u64)],
}

impl<'a, 'b, 'c> ManagedMenuBuilder<'a, 'b, 'c> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn states_controlled(&mut self, wsc: WidgetStatesControlled) -> &Self {
        self.wsc = wsc;
        self
    }

    pub fn selection(&mut self, selection: &'a gtk::TreeSelection) -> &Self {
        self.selection = Some(selection);
        self
    }

    pub fn change_notifier(&mut self, change_notifier: &'b Rc<ChangedCondnsNotifier>) -> &Self {
        self.change_notifier = Some(change_notifier);
        self
    }

    pub fn items(
        &mut self,
        items: &'c [(&'c str, &'c str, Option<&'c gtk::Image>, &'c str, u64)],
    ) -> &Self {
        self.items = items;
        self
    }

    pub fn build(self) -> ManagedMenu {
        let menu = gtk::MenuBuilder::new().build();
        let items = ConditionalWidgetGroups::<gtk::MenuItem>::new(
            self.wsc,
            self.selection,
            self.change_notifier,
        );
        let mm = ManagedMenu { menu, items };
        for &(name, label_text, image, tooltip_text, condns) in self.items.iter() {
            mm.append_item(name, label_text, image, tooltip_text, condns);
        }
        mm.menu.show_all();

        mm
    }
}

pub struct DualManagedMenu {
    menu: gtk::Menu,
    sensitivity: Rc<ConditionalWidgetGroups<gtk::MenuItem>>,
    visibility: Rc<ConditionalWidgetGroups<gtk::MenuItem>>,
}

pub type DMMenuSpec<'c> = (&'c str, &'c str, Option<&'c gtk::Image>, &'c str, u64, u64);

impl DualManagedMenu {
    pub fn new(
        selection: Option<&gtk::TreeSelection>,
        change_notifier: Option<&Rc<ChangedCondnsNotifier>>,
        items: &[DMMenuSpec],
    ) -> Self {
        let pm = Self {
            menu: gtk::Menu::new(),
            sensitivity: ConditionalWidgetGroups::<gtk::MenuItem>::new(
                WidgetStatesControlled::Sensitivity,
                selection,
                change_notifier,
            ),
            visibility: ConditionalWidgetGroups::<gtk::MenuItem>::new(
                WidgetStatesControlled::Visibility,
                selection,
                change_notifier,
            ),
        };
        for &(name, label_text, image, tooltip_text, sensitivity_condns, visibility_condns) in
            items.iter()
        {
            pm.append_item(
                name,
                label_text,
                image,
                tooltip_text,
                sensitivity_condns,
                visibility_condns,
            );
        }
        pm.menu.show_all();

        pm
    }

    pub fn menu(&self) -> gtk::Menu {
        self.menu.clone()
    }

    pub fn menu_item(&self, name: &str) -> Option<gtk::MenuItem> {
        self.sensitivity.get_widget(name)
    }

    fn append_menu_item(
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

    fn insert_menu_item(
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

    fn prepend_menu_item(
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

    fn new_item(
        &self,
        label_text: &str,
        image: Option<&gtk::Image>,
        tooltip_text: &str,
    ) -> gtk::MenuItem {
        let item = gtk::MenuItem::new();
        if let Some(image) = image {
            let h_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            h_box.pack_start(image, false, false, 0);
            h_box.pack_start(&gtk::Label::new(Some(label_text)), false, false, 0);
            item.add(&h_box);
        } else {
            item.set_label(label_text)
        }
        item.set_tooltip_text(Some(tooltip_text));

        item
    }

    pub fn append_item(
        &self,
        name: &str,
        label_text: &str,
        image: Option<&gtk::Image>,
        tooltip_text: &str,
        sensitivity_condns: u64,
        visibility_condns: u64,
    ) -> gtk::MenuItem {
        let item = self.new_item(label_text, image, tooltip_text);
        self.append_menu_item(name, &item, sensitivity_condns, visibility_condns);

        item
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_item(
        &self,
        name: &str,
        label_text: &str,
        image: Option<&gtk::Image>,
        tooltip_text: &str,
        sensitivity_condns: u64,
        visibility_condns: u64,
        position: i32,
    ) -> gtk::MenuItem {
        let item = self.new_item(label_text, image, tooltip_text);
        self.insert_menu_item(name, &item, sensitivity_condns, visibility_condns, position);

        item
    }

    pub fn prepend_item(
        &self,
        name: &str,
        label_text: &str,
        image: Option<&gtk::Image>,
        tooltip_text: &str,
        sensitivity_condns: u64,
        visibility_condns: u64,
    ) -> gtk::MenuItem {
        let item = self.new_item(label_text, image, tooltip_text);
        self.prepend_menu_item(name, &item, sensitivity_condns, visibility_condns);

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

pub struct DualManagedMenuBuilder<'a, 'b, 'c> {
    wsc: WidgetStatesControlled,
    selection: Option<&'a gtk::TreeSelection>,
    change_notifier: Option<&'b Rc<ChangedCondnsNotifier>>,
    items: &'c [DMMenuSpec<'c>],
}

impl<'a, 'b, 'c> Default for DualManagedMenuBuilder<'a, 'b, 'c> {
    fn default() -> Self {
        Self {
            wsc: WidgetStatesControlled::Sensitivity,
            selection: None,
            change_notifier: None,
            items: &[],
        }
    }
}

impl<'a, 'b, 'c> DualManagedMenuBuilder<'a, 'b, 'c> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn states_controlled(&mut self, wsc: WidgetStatesControlled) -> &Self {
        self.wsc = wsc;
        self
    }

    pub fn selection(&mut self, selection: &'a gtk::TreeSelection) -> &Self {
        self.selection = Some(selection);
        self
    }

    pub fn change_notifier(&mut self, change_notifier: &'b Rc<ChangedCondnsNotifier>) -> &Self {
        self.change_notifier = Some(change_notifier);
        self
    }

    pub fn items(&mut self, items: &'c [DMMenuSpec<'c>]) -> &Self {
        self.items = items;
        self
    }

    pub fn build(self) -> DualManagedMenu {
        let menu = gtk::MenuBuilder::new().build();
        let sensitivity = ConditionalWidgetGroups::<gtk::MenuItem>::new(
            self.wsc,
            self.selection,
            self.change_notifier,
        );
        let visibility = ConditionalWidgetGroups::<gtk::MenuItem>::new(
            self.wsc,
            self.selection,
            self.change_notifier,
        );
        let mm = DualManagedMenu {
            menu,
            sensitivity,
            visibility,
        };
        for &(name, label_text, image, tooltip_text, sensitivity_condns, visibility_condns) in
            self.items.iter()
        {
            mm.append_item(
                name,
                label_text,
                image,
                tooltip_text,
                sensitivity_condns,
                visibility_condns,
            );
        }
        mm.menu.show_all();

        mm
    }
}
