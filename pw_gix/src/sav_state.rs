// Copyright 2019 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

//! Provide mechanisms to control the sensitivity and/or visibility
//! of groups of widgets dependent on a widget and/or an application's
//! current state.
//! Up to 64 conditions can be used to describe a state.

use std::cell::{Cell, RefCell};
use std::clone::Clone;
use std::collections::HashMap;
use std::ops::BitOr;
use std::rc::Rc;

use gtk::{TreeSelection, TreeSelectionExt, WidgetExt};

/// A struct that enables the state of a subset of the conditions to
/// be specified without effecting the other conditions.
#[derive(Debug, Default, Clone, Copy)]
pub struct MaskedCondns {
    pub condns: u64,
    pub mask: u64,
}

impl MaskedCondns {
    pub fn is_consistent(&self) -> bool {
        self.condns & !self.mask == 0
    }
}

impl BitOr for MaskedCondns {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        MaskedCondns {
            condns: self.condns | rhs.condns,
            mask: self.mask | rhs.mask,
        }
    }
}

/// A trait that we can use to add a function to existing objects to
/// determine their states,
pub trait MaskedCondnProvider {
    fn get_masked_conditions(&self) -> MaskedCondns;
    fn get_masked_conditions_with_hover_ok(&self, hover_ok: bool) -> MaskedCondns;
}

pub const SAV_DONT_CARE: u64 = 0;
/// Interesting conditions for a TreeSelection that are useful for
/// tailoring pop up menus.
pub const SAV_SELN_NONE: u64 = 1; // << 0;
pub const SAV_SELN_MADE: u64 = 1 << 1;
pub const SAV_SELN_UNIQUE: u64 = 1 << 2;
pub const SAV_SELN_PAIR: u64 = 1 << 3;
pub const SAV_SELN_MADE_OR_HOVER_OK: u64 = 1 << 4;
pub const SAV_SELN_UNIQUE_OR_HOVER_OK: u64 = 1 << 5;
pub const SAV_SELN_MASK: u64 = (1 << 6) - 1;
pub const SAV_HOVER_OK: u64 = 1 << 6;
pub const SAV_HOVER_NOT_OK: u64 = 1 << 7;
pub const SAV_HOVER_MASK: u64 = SAV_HOVER_OK | SAV_HOVER_NOT_OK;
pub const SAV_NEXT_CONDN: u64 = 1 << 8;

pub fn hover_masked_conditions(hover_ok: bool) -> MaskedCondns {
    if hover_ok {
        MaskedCondns {
            condns: SAV_HOVER_OK,
            mask: SAV_HOVER_MASK,
        }
    } else {
        MaskedCondns {
            condns: SAV_HOVER_NOT_OK,
            mask: SAV_HOVER_MASK,
        }
    }
}

/// Implementation of MaskedCondnProvider for TreeSelection
impl MaskedCondnProvider for TreeSelection {
    fn get_masked_conditions(&self) -> MaskedCondns {
        match self.count_selected_rows() {
            0 => MaskedCondns {
                condns: SAV_SELN_NONE,
                mask: SAV_SELN_MASK,
            },
            1 => MaskedCondns {
                condns: SAV_SELN_MADE + SAV_SELN_UNIQUE,
                mask: SAV_SELN_MASK,
            },
            2 => MaskedCondns {
                condns: SAV_SELN_MADE + SAV_SELN_PAIR,
                mask: SAV_SELN_MASK,
            },
            _ => MaskedCondns {
                condns: SAV_SELN_MADE,
                mask: SAV_SELN_MASK,
            },
        }
    }

    fn get_masked_conditions_with_hover_ok(&self, hover_ok: bool) -> MaskedCondns {
        let mut mc = self.get_masked_conditions();
        if hover_ok && (mc.condns & SAV_SELN_NONE) != 0 {
            mc.condns |= SAV_SELN_UNIQUE_OR_HOVER_OK + SAV_SELN_MADE_OR_HOVER_OK
        } else {
            if (mc.condns & SAV_SELN_UNIQUE) != 0 {
                mc.condns |= SAV_SELN_UNIQUE_OR_HOVER_OK
            }
            if (mc.condns & SAV_SELN_MADE) != 0 {
                mc.condns |= SAV_SELN_MADE_OR_HOVER_OK
            }
        }
        mc | hover_masked_conditions(hover_ok)
    }
}

pub type NumberedChangeCallback = (u64, Box<dyn Fn(MaskedCondns)>);

#[derive(Default)]
pub struct ChangedCondnsNotifier {
    // TODO: remove mutexes: Gtk is single threaded
    callbacks: RefCell<Vec<NumberedChangeCallback>>,
    next_token: Cell<u64>,
    current_condns: Cell<u64>,
}

impl ChangedCondnsNotifier {
    pub fn new(initial_condns: u64) -> Rc<Self> {
        let ccn = Self::default();
        ccn.current_condns.set(initial_condns);
        Rc::new(ccn)
    }

    pub fn current_condns(&self) -> u64 {
        self.current_condns.get()
    }

    pub fn register_callback(&self, callback: Box<dyn Fn(MaskedCondns)>) -> u64 {
        let token = self.next_token.get();
        self.next_token.set(token + 1);

        self.callbacks.borrow_mut().push((token, callback));

        token
    }

    pub fn deregister_callback(&self, token: u64) {
        let position = self.callbacks.borrow().iter().position(|x| x.0 == token);
        if let Some(position) = position {
            let _ = self.callbacks.borrow_mut().remove(position);
        }
    }

    pub fn notify_changed_condns(&self, masked_condns: MaskedCondns) {
        for (_, callback) in self.callbacks.borrow().iter() {
            callback(masked_condns)
        }
        self.current_condns
            .set(masked_condns.condns | (self.current_condns.get() & !masked_condns.mask));
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum WidgetStatesControlled {
    #[default]
    Sensitivity,
    Visibility,
    Both,
}

use self::WidgetStatesControlled::*;

#[derive(Debug)]
struct ConditionalWidgetGroup<W>
where
    W: WidgetExt + Clone + PartialEq,
{
    widget_states_controlled: WidgetStatesControlled,
    widgets: HashMap<String, W>,
    is_on: bool,
}

impl<W> ConditionalWidgetGroup<W>
where
    W: WidgetExt + Clone + PartialEq,
{
    fn new(wsc: WidgetStatesControlled) -> Self {
        Self {
            widget_states_controlled: wsc,
            widgets: HashMap::new(),
            is_on: false,
        }
    }

    fn len(&self) -> usize {
        self.widgets.len()
    }

    fn contains_name(&self, name: &str) -> bool {
        self.widgets.contains_key(name)
    }

    fn contains_widget(&self, widget: &W) -> bool {
        for value in self.widgets.values() {
            if value == widget {
                return true;
            }
        }
        false
    }

    fn add_widget(&mut self, name: &str, widget: W) {
        match self.widget_states_controlled {
            Sensitivity => widget.set_sensitive(self.is_on),
            Visibility => widget.set_visible(self.is_on),
            Both => {
                widget.set_sensitive(self.is_on);
                widget.set_visible(self.is_on);
            }
        }
        self.widgets.insert(name.to_string(), widget);
    }

    fn set_state(&mut self, on: bool) {
        match self.widget_states_controlled {
            Sensitivity => {
                for widget in self.widgets.values() {
                    widget.set_sensitive(on);
                }
            }
            Visibility => {
                for widget in self.widgets.values() {
                    widget.set_visible(on);
                }
            }
            Both => {
                for widget in self.widgets.values() {
                    widget.set_sensitive(on);
                    widget.set_visible(on);
                }
            }
        }
        self.is_on = on
    }
}

/// Groups of widgets whose sensitivity and/or visibility is determined
/// by the current conditions
// TODO: get a better name than ConditionalWidgetGroups
// TODO: make a dynamic trait version
pub struct ConditionalWidgetGroups<W>
where
    W: WidgetExt + Clone + PartialEq,
{
    widget_states_controlled: WidgetStatesControlled,
    groups: RefCell<HashMap<u64, ConditionalWidgetGroup<W>>>,
    current_condns: Cell<u64>,
    change_notifier: Rc<ChangedCondnsNotifier>,
    selection: Option<TreeSelection>,
}

impl<W> ConditionalWidgetGroups<W>
where
    W: WidgetExt + Clone + PartialEq,
{
    pub fn new(
        wsc: WidgetStatesControlled,
        selection: Option<&TreeSelection>,
        change_notifier: Option<&Rc<ChangedCondnsNotifier>>,
    ) -> Rc<Self> {
        let change_notifier = if let Some(change_notifier) = change_notifier {
            Rc::clone(change_notifier)
        } else {
            ChangedCondnsNotifier::new(0)
        };
        let initial_condns = change_notifier.current_condns();
        let cwg = Rc::new(ConditionalWidgetGroups::<W> {
            widget_states_controlled: wsc,
            groups: RefCell::new(HashMap::new()),
            current_condns: Cell::new(initial_condns),
            change_notifier,
            selection: selection.cloned(),
        });
        if let Some(selection) = selection {
            cwg.update_condns(selection.get_masked_conditions());
            let cwg_clone = Rc::clone(&cwg);
            selection
                .connect_changed(move |seln| cwg_clone.update_condns(seln.get_masked_conditions()));
        }
        let cwg_clone = Rc::clone(&cwg);
        cwg.change_notifier
            .register_callback(Box::new(move |condns| cwg_clone.update_condns(condns)));
        cwg
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        for group in self.groups.borrow().values() {
            len += group.len()
        }
        len
    }

    pub fn current_condns(&self) -> u64 {
        self.current_condns.get()
    }

    pub fn change_notifier(&self) -> &Rc<ChangedCondnsNotifier> {
        &self.change_notifier
    }

    fn contains_name(&self, name: &str) -> bool {
        for group in self.groups.borrow().values() {
            if group.contains_name(name) {
                return true;
            }
        }
        false
    }

    fn contains_widget(&self, widget: &W) -> bool {
        for group in self.groups.borrow().values() {
            if group.contains_widget(widget) {
                return true;
            }
        }
        false
    }

    pub fn add_widget(&self, name: &str, widget: &W, condns: u64) {
        assert!(!self.contains_widget(widget));
        assert!(!self.contains_name(name));
        let mut groups = self.groups.borrow_mut();
        if let Some(group) = groups.get_mut(&condns) {
            group.add_widget(name, widget.clone());
            return;
        }
        let mut group = ConditionalWidgetGroup::<W>::new(self.widget_states_controlled);
        group.set_state((condns & self.current_condns.get()) == condns);
        group.add_widget(name, widget.clone());
        groups.insert(condns, group);
    }

    pub fn get_widget(&self, name: &str) -> Option<W> {
        let groups = self.groups.borrow();
        for group in groups.values() {
            if let Some(widget) = group.widgets.get(name) {
                return Some(widget.clone());
            }
        }
        None
    }

    pub fn update_condns(&self, changed_condns: MaskedCondns) {
        assert!(changed_condns.is_consistent());
        let new_condns = changed_condns.condns | (self.current_condns.get() & !changed_condns.mask);
        for (key_condns, group) in self.groups.borrow_mut().iter_mut() {
            if changed_condns.mask & key_condns != 0 {
                group.set_state((key_condns & new_condns) == *key_condns);
            };
        }
        self.current_condns.set(new_condns)
    }

    pub fn update_hover_condns(&self, hover_ok: bool) {
        let new_condns = if let Some(selection) = &self.selection {
            selection.get_masked_conditions_with_hover_ok(hover_ok)
        } else {
            hover_masked_conditions(hover_ok)
        };
        self.update_condns(new_condns);
    }
}

#[derive(Debug)]
struct ConditionalWidgetHashMap<K, W>
where
    W: WidgetExt + Clone + PartialEq,
    K: Eq + std::hash::Hash + std::fmt::Debug,
{
    widget_states_controlled: WidgetStatesControlled,
    widgets: HashMap<K, W>,
    is_on: bool,
}

impl<K, W> Default for ConditionalWidgetHashMap<K, W>
where
    W: WidgetExt + Clone + PartialEq,
    K: Eq + std::hash::Hash + std::fmt::Debug,
{
    fn default() -> Self {
        Self {
            widget_states_controlled: WidgetStatesControlled::default(),
            is_on: false,
            widgets: HashMap::new(),
        }
    }
}

impl<K, W> ConditionalWidgetHashMap<K, W>
where
    W: WidgetExt + Clone + PartialEq,
    K: Eq + std::hash::Hash + std::fmt::Debug,
{
    fn new(widget_states_controlled: WidgetStatesControlled) -> Self {
        ConditionalWidgetHashMap::<K, W> {
            widget_states_controlled,
            ..Default::default()
        }
    }

    fn len(&self) -> usize {
        self.widgets.len()
    }

    fn contains_widget(&self, widget: &W) -> bool {
        for value in self.widgets.values() {
            if value == widget {
                return true;
            }
        }
        false
    }

    fn insert(&mut self, key: K, widget: W) {
        debug_assert!(!self.widgets.contains_key(&key));
        match self.widget_states_controlled {
            Sensitivity => widget.set_sensitive(self.is_on),
            Visibility => widget.set_visible(self.is_on),
            Both => {
                widget.set_sensitive(self.is_on);
                widget.set_visible(self.is_on);
            }
        }
        self.widgets.insert(key, widget);
    }

    fn set_state(&mut self, on: bool) {
        match self.widget_states_controlled {
            Sensitivity => {
                for widget in self.widgets.values() {
                    widget.set_sensitive(on);
                }
            }
            Visibility => {
                for widget in self.widgets.values() {
                    widget.set_visible(on);
                }
            }
            Both => {
                for widget in self.widgets.values() {
                    widget.set_sensitive(on);
                    widget.set_visible(on);
                }
            }
        }
        self.is_on = on
    }
}

#[derive(Default)]
pub struct ConditionalWidgets<K, W>
where
    W: WidgetExt + Clone + PartialEq,
    K: Eq + std::hash::Hash + std::fmt::Debug,
{
    widget_states_controlled: WidgetStatesControlled,
    groups: RefCell<HashMap<u64, ConditionalWidgetHashMap<K, W>>>,
    current_condns: Cell<u64>,
    change_notifier: Rc<ChangedCondnsNotifier>,
    selection: Option<TreeSelection>,
}

impl<K, W> ConditionalWidgets<K, W>
where
    W: WidgetExt + Clone + PartialEq,
    K: Eq + std::hash::Hash + std::fmt::Debug,
{
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        for group in self.groups.borrow().values() {
            len += group.len()
        }
        len
    }

    fn contains_key(&self, key: &K) -> bool {
        for group in self.groups.borrow().values() {
            if group.widgets.contains_key(key) {
                return true;
            }
        }
        false
    }

    fn contains_widget(&self, widget: &W) -> bool {
        for group in self.groups.borrow().values() {
            if group.contains_widget(widget) {
                return true;
            }
        }
        false
    }

    pub fn add_widget(&self, key: K, widget: &W, condns: u64) {
        debug_assert!(!self.contains_widget(widget));
        debug_assert!(!self.contains_key(&key));
        let mut groups = self.groups.borrow_mut();
        if let Some(group) = groups.get_mut(&condns) {
            group.insert(key, widget.clone());
            return;
        }
        let mut group = ConditionalWidgetHashMap::<K, W>::new(self.widget_states_controlled);
        group.set_state((condns & self.current_condns.get()) == condns);
        group.insert(key, widget.clone());
        groups.insert(condns, group);
    }

    pub fn get_widget<Q: ?Sized>(&self, key: &Q) -> Option<W>
    where
        K: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + Eq,
    {
        let groups = self.groups.borrow();
        for group in groups.values() {
            if let Some(widget) = group.widgets.get(key) {
                return Some(widget.clone());
            }
        }
        None
    }

    pub fn update_condns(&self, changed_condns: MaskedCondns) {
        debug_assert!(changed_condns.is_consistent());
        let new_condns = changed_condns.condns | (self.current_condns.get() & !changed_condns.mask);
        for (key_condns, group) in self.groups.borrow_mut().iter_mut() {
            if changed_condns.mask & key_condns != 0 {
                group.set_state((key_condns & new_condns) == *key_condns);
            };
        }
        self.current_condns.set(new_condns)
    }

    pub fn update_hover_condns(&self, hover_ok: bool) {
        let new_condns = if let Some(selection) = &self.selection {
            selection.get_masked_conditions_with_hover_ok(hover_ok)
        } else {
            hover_masked_conditions(hover_ok)
        };
        self.update_condns(new_condns);
    }
}

pub struct ConditionalWidgetsBuilder {
    widget_states_controlled: WidgetStatesControlled,
    change_notifier: Rc<ChangedCondnsNotifier>,
    selection: Option<TreeSelection>,
}

impl Default for ConditionalWidgetsBuilder {
    fn default() -> Self {
        Self {
            widget_states_controlled: WidgetStatesControlled::default(),
            change_notifier: ChangedCondnsNotifier::new(0),
            selection: None,
        }
    }
}

impl ConditionalWidgetsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn widget_states_controlled(
        &mut self,
        widget_states_controlled: WidgetStatesControlled,
    ) -> &mut Self {
        self.widget_states_controlled = widget_states_controlled;
        self
    }

    pub fn change_notifier(&mut self, change_notifier: &Rc<ChangedCondnsNotifier>) -> &mut Self {
        self.change_notifier = Rc::clone(change_notifier);
        self
    }

    pub fn selection(&mut self, selection: &TreeSelection) -> &mut Self {
        self.selection = Some(selection.clone());
        self
    }

    pub fn build<K, W>(&self) -> Rc<ConditionalWidgets<K, W>>
    where
        W: WidgetExt + Clone + PartialEq,
        K: Eq + std::hash::Hash + std::fmt::Debug + 'static,
    {
        let change_notifier = Rc::clone(&self.change_notifier);
        let initial_condns = change_notifier.current_condns();
        let selection = self.selection.clone();
        let cwg = Rc::new(ConditionalWidgets::<K, W> {
            widget_states_controlled: self.widget_states_controlled,
            groups: RefCell::new(HashMap::new()),
            current_condns: Cell::new(initial_condns),
            change_notifier,
            selection,
        });
        if let Some(selection) = &cwg.selection {
            cwg.update_condns(selection.get_masked_conditions());
            let cwg_clone = Rc::clone(&cwg);
            selection
                .connect_changed(move |seln| cwg_clone.update_condns(seln.get_masked_conditions()));
        }
        let cwg_clone = Rc::clone(&cwg);
        cwg.change_notifier
            .register_callback(Box::new(move |condns| cwg_clone.update_condns(condns)));
        cwg
    }
}
