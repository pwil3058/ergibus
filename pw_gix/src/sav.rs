// Copyright 2020 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
//! Provide mechanisms to control the sensitivity and/or visibility
//! of groups of widgets dependent on a widget and/or an application's
//! current state.
//! Up to 128 conditions can be used to describe a state.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ops::{Add, BitAnd, BitOr, Not},
};

use gtk::prelude::*;

/// Set of boolean flags representing conditions that define a state
#[derive(Debug, Default, PartialOrd, PartialEq, Ord, Eq, Clone, Copy)]
pub struct Condns(pub u128);

pub const DONT_CARE: Condns = Condns(0);

impl Condns {
    /// Returns `true` if `self`'s flags are a subset of `other`' flags.
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.0 & other.0 == self.0
    }

    /// Returns `true` if `self`'s flags are a superset of `other`'s flags.
    pub fn is_superset_of(&self, other: &Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl From<u128> for Condns {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl Not for Condns {
    type Output = Self;

    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl BitAnd for Condns {
    type Output = Self;

    fn bitand(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl BitOr for Condns {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Add<&Change> for Condns {
    type Output = Self;

    fn add(self, change: &Change) -> Self {
        change.1 | (self & !change.0)
    }
}

/// An ordered pair of `Condns` specifying a change to be made to a set of conditions.
/// The first member of the pair specifies which conditions should be changed and the second
/// specifies the values that they should be given.
#[derive(Debug, Default, PartialOrd, PartialEq, Ord, Eq, Clone, Copy)]
pub struct Change(pub Condns, pub Condns);

impl Change {
    #[inline]
    pub fn changed_condns(&self) -> Condns {
        self.0
    }

    #[inline]
    pub fn new_values(&self) -> Condns {
        self.1
    }

    pub fn is_valid(&self) -> bool {
        self.changed_condns().is_superset_of(&self.new_values())
    }
}

/// Interesting conditions for a TreeSelection that are useful for
/// tailoring pop up menus.
pub const SELN_NONE: Condns = Condns(1); // << 0;
pub const SELN_MADE: Condns = Condns(1 << 1);
pub const SELN_UNIQUE: Condns = Condns(1 << 2);
pub const SELN_PAIR: Condns = Condns(1 << 3);
pub const SELN_MADE_OR_HOVER_OK: Condns = Condns(1 << 4);
pub const SELN_UNIQUE_OR_HOVER_OK: Condns = Condns(1 << 5);
pub const SELN_CONDITIONS: Condns = Condns((1 << 6) - 1);
/// Conditions for mouse hovering over a row in a TreeView or an area of interest in a DrawingArea
pub const HOVER_OK: Condns = Condns(1 << 6);
pub const HOVER_NOT_OK: Condns = Condns(1 << 7);
pub const HOVER_CONDITIONS: Condns = Condns(HOVER_OK.0 | HOVER_NOT_OK.0);
pub const NEXT_FLAG: u128 = 1 << 8;

pub fn hover_change(hover_ok: bool) -> Change {
    if hover_ok {
        Change(HOVER_CONDITIONS, HOVER_OK)
    } else {
        Change(HOVER_CONDITIONS, HOVER_NOT_OK)
    }
}

/// A trait that we can use to add a function to existing objects to
/// determine the state of the subset of conditions that they are responsible
/// for monitoring.
pub trait ConditionSource {
    fn conditions_subset(&self) -> Change;
    fn conditions_subset_with_hover_ok(&self, hover_ok: bool) -> Change;
}

impl ConditionSource for gtk::TreeSelection {
    fn conditions_subset(&self) -> Change {
        match self.count_selected_rows() {
            0 => Change(SELN_CONDITIONS, SELN_NONE),
            1 => Change(SELN_CONDITIONS, SELN_MADE | SELN_UNIQUE),
            2 => Change(SELN_CONDITIONS, SELN_MADE | SELN_PAIR),
            _ => Change(SELN_CONDITIONS, SELN_MADE),
        }
    }

    fn conditions_subset_with_hover_ok(&self, hover_ok: bool) -> Change {
        let rel_condns = SELN_CONDITIONS | HOVER_CONDITIONS;
        if hover_ok {
            match self.count_selected_rows() {
                0 => Change(rel_condns, SELN_NONE | HOVER_OK),
                1 => Change(
                    rel_condns,
                    SELN_MADE | SELN_UNIQUE | HOVER_OK | SELN_UNIQUE_OR_HOVER_OK,
                ),
                2 => Change(
                    rel_condns,
                    SELN_MADE | SELN_PAIR | HOVER_OK | SELN_MADE_OR_HOVER_OK,
                ),
                _ => Change(rel_condns, SELN_MADE | HOVER_OK | SELN_MADE_OR_HOVER_OK),
            }
        } else {
            match self.count_selected_rows() {
                0 => Change(rel_condns, SELN_NONE | HOVER_NOT_OK),
                1 => Change(rel_condns, SELN_MADE | SELN_UNIQUE | HOVER_NOT_OK),
                2 => Change(rel_condns, SELN_MADE | SELN_PAIR | HOVER_NOT_OK),
                _ => Change(rel_condns, SELN_MADE | HOVER_NOT_OK),
            }
        }
    }
}

pub trait ApplyChange {
    fn apply_changed_condns(&self, change: &Change);
}

/// Conditions for a `Widget` to be sensitive, visible and/or both.
#[derive(Debug, Clone, Copy)]
pub enum Policy {
    Sensitivity(Condns),
    Visibility(Condns),
    Both(Condns, Condns),
}

#[derive(Debug, Default)]
pub struct Enforcer {
    widget_policy: RefCell<HashMap<gtk::Widget, Policy>>,
    current_condns: Cell<Condns>,
}

impl Enforcer {
    pub fn add_widget<W: IsA<gtk::Widget>>(&self, w: &W, policy: Policy) {
        let widget = w.clone().upcast::<gtk::Widget>();
        match &policy {
            Policy::Sensitivity(condns) => {
                widget.set_sensitive(self.current_condns.get().is_superset_of(condns))
            }
            Policy::Visibility(condns) => {
                widget.set_visible(self.current_condns.get().is_superset_of(condns))
            }
            Policy::Both(s_condns, v_condns) => {
                widget.set_sensitive(self.current_condns.get().is_superset_of(s_condns));
                widget.set_visible(self.current_condns.get().is_superset_of(v_condns))
            }
        }
        let result = self.widget_policy.borrow_mut().insert(widget, policy);
        debug_assert!(result.is_none());
    }

    pub fn remove_widget<W: IsA<gtk::Widget>>(&self, w: &W) -> Result<(), &'static str> {
        let widget = w.clone().upcast::<gtk::Widget>();
        if self.widget_policy.borrow_mut().remove(&widget).is_none() {
            return Err("Widget not found");
        };
        Ok(())
    }
}

impl ApplyChange for Enforcer {
    fn apply_changed_condns(&self, change: &Change) {
        debug_assert!(change.is_valid());
        let new_condns = self.current_condns.get() + change;
        for (widget, policy) in self.widget_policy.borrow().iter() {
            match policy {
                Policy::Sensitivity(condns) => {
                    widget.set_sensitive(new_condns.is_superset_of(condns))
                }
                Policy::Visibility(condns) => widget.set_visible(new_condns.is_superset_of(condns)),
                Policy::Both(s_condns, v_condns) => {
                    widget.set_sensitive(new_condns.is_superset_of(s_condns));
                    widget.set_visible(new_condns.is_superset_of(v_condns))
                }
            }
        }
        self.current_condns.set(new_condns);
    }
}

#[derive(Debug, Default)]
pub struct EnforcerBuilder {
    initial_condns: Condns,
}

impl EnforcerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initial_condns(&mut self, initial_condns: Condns) -> &mut Self {
        self.initial_condns = initial_condns;
        self
    }

    pub fn build(&self) -> Enforcer {
        let enforcer = Enforcer::default();
        enforcer.current_condns.set(self.initial_condns);
        enforcer
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn into() {
        assert_eq!(Condns(45), 45.into());
    }

    #[test]
    fn subset_of() {
        assert!(Condns(0b101010).is_subset_of(&Condns(0b101010)));
        assert!(Condns(0b101010).is_subset_of(&Condns(0b101110)));
        assert!(!Condns(0b1101010).is_subset_of(&Condns(0b101110)));
    }

    #[test]
    fn superset_of() {
        assert!(!Condns(0b101010).is_superset_of(&Condns(0b101110)));
        assert!(!Condns(0b1101010).is_superset_of(&Condns(0b101110)));
        assert!(Condns(0b1101010).is_superset_of(&Condns(0b101010)));
        assert!(Condns(0b1101010).is_superset_of(&Condns(0b1101010)));
    }
}
