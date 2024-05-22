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

pub use pw_gtk_ext_derive::*;

#[derive(Debug)]
pub enum Error {
    DuplicateKey,
    DuplicateWidget,
    NotFound,
    InconsistentMaskCondns,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            DuplicateKey => f.write_str("Duplicate key"),
            DuplicateWidget => f.write_str("Duplicate widget"),
            NotFound => f.write_str("Not found"),
            InconsistentMaskCondns => f.write_str("Inconsistent mask/conditions"),
        }
    }
}

impl std::error::Error for Error {}

/// A struct that enables the state of a subset of the conditions to
/// be specified without effecting the other conditions.
#[derive(Debug, Default, Clone, Copy)]
pub struct MaskedCondns {
    pub condns: u64,
    pub mask: u64,
}

impl MaskedCondns {
    /// Check whether `MaskedCondns` is internally consistent.
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
pub const SAV_SELN_NONE_BUT_HOVER_OK: u64 = 1 << 6;
pub const SAV_SELN_MASK: u64 = (1 << 7) - 1;
pub const SAV_HOVER_OK: u64 = 1 << 7;
pub const SAV_HOVER_NOT_OK: u64 = 1 << 8;
pub const SAV_HOVER_MASK: u64 = SAV_HOVER_OK | SAV_HOVER_NOT_OK;
pub const SAV_NEXT_CONDN: u64 = 1 << 9;

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
            mc.condns |=
                SAV_SELN_UNIQUE_OR_HOVER_OK + SAV_SELN_MADE_OR_HOVER_OK + SAV_SELN_NONE_BUT_HOVER_OK
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
pub struct ChangedCondnsNotifierCore {
    callbacks: RefCell<Vec<NumberedChangeCallback>>,
    next_token: Cell<u64>,
    current_condns: Cell<u64>,
}

#[derive(Default, WClone)]
pub struct ChangedCondnsNotifier(Rc<ChangedCondnsNotifierCore>);

impl ChangedCondnsNotifier {
    pub fn new(initial_condns: u64) -> Self {
        let ccn = ChangedCondnsNotifierCore::default();
        ccn.current_condns.set(initial_condns);
        Self(Rc::new(ccn))
    }

    pub fn current_condns(&self) -> u64 {
        self.0.current_condns.get()
    }

    pub fn register_callback(&self, callback: Box<dyn Fn(MaskedCondns)>) -> u64 {
        let token = self.0.next_token.get();
        self.0.next_token.set(token + 1);

        self.0.callbacks.borrow_mut().push((token, callback));

        token
    }

    pub fn deregister_callback(&self, token: u64) {
        let position = self.0.callbacks.borrow().iter().position(|x| x.0 == token);
        if let Some(position) = position {
            let _ = self.0.callbacks.borrow_mut().remove(position);
        }
    }

    pub fn notify_changed_condns(&self, masked_condns: MaskedCondns) {
        debug_assert!(masked_condns.is_consistent());
        for (_, callback) in self.0.callbacks.borrow().iter() {
            callback(masked_condns)
        }
        self.0
            .current_condns
            .set(masked_condns.condns | (self.0.current_condns.get() & !masked_condns.mask));
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

#[derive(Debug, Default)]
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
    fn len(&self) -> usize {
        self.widgets.len()
    }

    fn add_widget(&mut self, name: &str, widget: W) -> Result<(), Error> {
        if self.widgets.contains_key(name) {
            Err(Error::DuplicateKey)
        } else if self.widgets.values().any(|value| value == &widget) {
            Err(Error::DuplicateWidget)
        } else {
            match self.widget_states_controlled {
                Sensitivity => widget.set_sensitive(self.is_on),
                Visibility => widget.set_visible(self.is_on),
                Both => {
                    widget.set_sensitive(self.is_on);
                    widget.set_visible(self.is_on);
                }
            }
            if self.widgets.insert(name.to_string(), widget).is_none() {
                Ok(())
            } else {
                Err(Error::DuplicateKey)
            }
        }
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

#[derive(Debug, Default)]
struct ConditionalWidgetGroupBuilder {
    widget_states_controlled: WidgetStatesControlled,
}

impl ConditionalWidgetGroupBuilder {
    fn new(widget_states_controlled: WidgetStatesControlled) -> Self {
        Self {
            widget_states_controlled,
        }
    }

    fn build<W>(&self) -> ConditionalWidgetGroup<W>
    where
        W: WidgetExt + Clone + PartialEq,
    {
        ConditionalWidgetGroup::<W> {
            widget_states_controlled: self.widget_states_controlled,
            widgets: HashMap::new(),
            is_on: false,
        }
    }
}

/// Groups of widgets whose sensitivity and/or visibility is determined
/// by the current conditions
// TODO: make a dynamic trait version
#[derive(Default)]
pub struct ConditionalWidgetGroupsCore<W>
where
    W: WidgetExt + Clone + PartialEq,
{
    conditional_widget_group_builder: ConditionalWidgetGroupBuilder,
    groups: RefCell<HashMap<u64, ConditionalWidgetGroup<W>>>,
    current_condns: Cell<u64>,
    change_notifier: ChangedCondnsNotifier,
    selection: Option<TreeSelection>,
}

#[derive(Default, WClone)]
pub struct ConditionalWidgetGroups<W>(Rc<ConditionalWidgetGroupsCore<W>>)
where
    W: WidgetExt + Clone + PartialEq;

#[derive(Default)]
pub struct ConditionalWidgetGroupsBuilder {
    widget_states_controlled: WidgetStatesControlled,
    selection: Option<TreeSelection>,
    change_notifier: Option<ChangedCondnsNotifier>,
}

impl ConditionalWidgetGroupsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn widget_states_controlled(&mut self, wsc: WidgetStatesControlled) -> &mut Self {
        self.widget_states_controlled = wsc;
        self
    }

    pub fn selection(&mut self, selection: &TreeSelection) -> &mut Self {
        self.selection = Some(selection.clone());
        self
    }

    pub fn change_notifier(&mut self, change_notifier: &ChangedCondnsNotifier) -> &mut Self {
        self.change_notifier = Some(change_notifier.clone());
        self
    }

    pub fn build<W>(&self) -> ConditionalWidgetGroups<W>
    where
        W: WidgetExt + Clone + PartialEq,
    {
        let change_notifier = if let Some(change_notifier) = &self.change_notifier {
            change_notifier.clone()
        } else {
            ChangedCondnsNotifier::new(0)
        };
        let initial_condns = change_notifier.current_condns();
        let selection = self.selection.as_ref().cloned();
        let cwg = ConditionalWidgetGroups(Rc::new(ConditionalWidgetGroupsCore::<W> {
            conditional_widget_group_builder: ConditionalWidgetGroupBuilder::new(
                self.widget_states_controlled,
            ),
            groups: RefCell::new(HashMap::new()),
            current_condns: Cell::new(initial_condns),
            change_notifier,
            selection,
        }));
        if let Some(selection) = &self.selection {
            cwg.update_condns(selection.get_masked_conditions());
            let cwg_clone = cwg.clone();
            selection
                .connect_changed(move |seln| cwg_clone.update_condns(seln.get_masked_conditions()));
        }
        let cwg_clone = cwg.clone();
        cwg.0
            .change_notifier
            .register_callback(Box::new(move |condns| cwg_clone.update_condns(condns)));
        cwg
    }
}

impl<W> ConditionalWidgetGroups<W>
where
    W: WidgetExt + Clone + PartialEq,
{
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        for group in self.0.groups.borrow().values() {
            len += group.len()
        }
        len
    }

    pub fn current_condns(&self) -> u64 {
        self.0.current_condns.get()
    }

    pub fn change_notifier(&self) -> &ChangedCondnsNotifier {
        &self.0.change_notifier
    }

    pub fn add_widget(&self, name: &str, widget: &W, condns: u64) -> Result<(), Error> {
        let mut groups = self.0.groups.borrow_mut();
        if let Some(group) = groups.get_mut(&condns) {
            group.add_widget(name, widget.clone())?;
            return Ok(());
        }
        let mut group = self.0.conditional_widget_group_builder.build::<W>();
        group.set_state((condns & self.0.current_condns.get()) == condns);
        group.add_widget(name, widget.clone())?;
        groups.insert(condns, group);
        Ok(())
    }

    pub fn get_widget(&self, name: &str) -> Result<W, Error> {
        let groups = self.0.groups.borrow();
        for group in groups.values() {
            if let Some(widget) = group.widgets.get(name) {
                return Ok(widget.clone());
            }
        }
        Err(Error::NotFound)
    }

    pub fn update_condns(&self, changed_condns: MaskedCondns) {
        assert!(changed_condns.is_consistent());
        let new_condns =
            changed_condns.condns | (self.0.current_condns.get() & !changed_condns.mask);
        for (key_condns, group) in self.0.groups.borrow_mut().iter_mut() {
            if changed_condns.mask & key_condns != 0 {
                group.set_state((key_condns & new_condns) == *key_condns);
            };
        }
        self.0.current_condns.set(new_condns)
    }

    pub fn update_hover_condns(&self, hover_ok: bool) {
        let new_condns = if let Some(selection) = &self.0.selection {
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
        Self {
            widget_states_controlled,
            ..std::default::Default::default()
        }
        //let mut cwhm = Self::default();
        //cwhm.widget_states_controlled = widget_states_controlled;
        //cwhm
    }

    fn len(&self) -> usize {
        self.widgets.len()
    }

    fn insert(&mut self, key: K, widget: W) -> Result<(), Error> {
        if self.widgets.contains_key(&key) {
            Err(Error::DuplicateKey)
        } else if self.widgets.values().any(|value| value == &widget) {
            Err(Error::DuplicateWidget)
        } else {
            match self.widget_states_controlled {
                Sensitivity => widget.set_sensitive(self.is_on),
                Visibility => widget.set_visible(self.is_on),
                Both => {
                    widget.set_sensitive(self.is_on);
                    widget.set_visible(self.is_on);
                }
            }
            if self.widgets.insert(key, widget).is_none() {
                Ok(())
            } else {
                Err(Error::DuplicateKey)
            }
        }
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
pub struct ConditionalWidgetsCore<K, W>
where
    W: WidgetExt + Clone + PartialEq,
    K: Eq + std::hash::Hash + std::fmt::Debug,
{
    widget_states_controlled: WidgetStatesControlled,
    groups: RefCell<HashMap<u64, ConditionalWidgetHashMap<K, W>>>,
    current_condns: Cell<u64>,
    change_notifier: ChangedCondnsNotifier,
    selection: Option<TreeSelection>,
}

#[derive(Default, WClone)]
pub struct ConditionalWidgets<K, W>(Rc<ConditionalWidgetsCore<K, W>>)
where
    W: WidgetExt + Clone + PartialEq,
    K: Eq + std::hash::Hash + std::fmt::Debug;

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
        for group in self.0.groups.borrow().values() {
            len += group.len()
        }
        len
    }

    pub fn add_widget(&self, key: K, widget: &W, condns: u64) -> Result<(), Error> {
        let mut groups = self.0.groups.borrow_mut();
        if let Some(group) = groups.get_mut(&condns) {
            group.insert(key, widget.clone())?;
            return Ok(());
        }
        let mut group = ConditionalWidgetHashMap::<K, W>::new(self.0.widget_states_controlled);
        group.set_state((condns & self.0.current_condns.get()) == condns);
        group.insert(key, widget.clone())?;
        groups.insert(condns, group);
        Ok(())
    }

    pub fn get_widget<Q: ?Sized>(&self, key: &Q) -> Result<W, Error>
    where
        K: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + Eq,
    {
        let groups = self.0.groups.borrow();
        for group in groups.values() {
            if let Some(widget) = group.widgets.get(key) {
                return Ok(widget.clone());
            }
        }
        Err(Error::NotFound)
    }

    pub fn update_condns(&self, changed_condns: MaskedCondns) {
        debug_assert!(changed_condns.is_consistent());
        let new_condns =
            changed_condns.condns | (self.0.current_condns.get() & !changed_condns.mask);
        for (key_condns, group) in self.0.groups.borrow_mut().iter_mut() {
            if changed_condns.mask & key_condns != 0 {
                group.set_state((key_condns & new_condns) == *key_condns);
            };
        }
        self.0.current_condns.set(new_condns)
    }

    pub fn update_hover_condns(&self, hover_ok: bool) {
        let new_condns = if let Some(selection) = &self.0.selection {
            selection.get_masked_conditions_with_hover_ok(hover_ok)
        } else {
            hover_masked_conditions(hover_ok)
        };
        self.update_condns(new_condns);
    }
}

pub struct ConditionalWidgetsBuilder {
    widget_states_controlled: WidgetStatesControlled,
    change_notifier: ChangedCondnsNotifier,
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

    pub fn change_notifier(&mut self, change_notifier: &ChangedCondnsNotifier) -> &mut Self {
        self.change_notifier = change_notifier.clone();
        self
    }

    pub fn selection(&mut self, selection: &TreeSelection) -> &mut Self {
        self.selection = Some(selection.clone());
        self
    }

    pub fn build<K, W>(&self) -> ConditionalWidgets<K, W>
    where
        W: WidgetExt + Clone + PartialEq,
        K: Eq + std::hash::Hash + std::fmt::Debug + 'static,
    {
        let change_notifier = self.change_notifier.clone();
        let initial_condns = change_notifier.current_condns();
        let selection = self.selection.clone();
        let cwg = ConditionalWidgets(Rc::new(ConditionalWidgetsCore::<K, W> {
            widget_states_controlled: self.widget_states_controlled,
            groups: RefCell::new(HashMap::new()),
            current_condns: Cell::new(initial_condns),
            change_notifier,
            selection,
        }));
        if let Some(selection) = &cwg.0.selection {
            cwg.update_condns(selection.get_masked_conditions());
            let cwg_clone = cwg.clone();
            selection
                .connect_changed(move |seln| cwg_clone.update_condns(seln.get_masked_conditions()));
        }
        let cwg_clone = cwg.clone();
        cwg.0
            .change_notifier
            .register_callback(Box::new(move |condns| cwg_clone.update_condns(condns)));
        cwg
    }
}
