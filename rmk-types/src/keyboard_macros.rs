//! Keyboard macros: a macro is a sequence of [`MacroOp`]s.
//!
//! Default macros are compiled into rodata as `&[&[MacroOp]]`; macros a host
//! writes travel and persist as a [`Macro`], one storage item per macro.

use core::ops::Deref;

use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::constants::{MACRO_MAX_NUM, MACRO_MAX_SIZE};

/// One step of a macro.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
pub enum MacroOp {
    /// Press and release the action.
    Tap(Action),
    /// Press the action and leave it pressed.
    Press(Action),
    /// Release the action.
    Release(Action),
    /// Wait this many milliseconds before the next op.
    Delay(u16),
    /// Type one ASCII character with its own shift state, ignoring held modifiers.
    Char(u8),
    /// The ops before this run when the macro key is pressed, the ops after it
    /// when the macro key is released.
    PauseForRelease,
}

/// A whole macro: at most `MACRO_MAX_SIZE` ops.
///
/// A newtype rather than a bare `heapless::Vec`, which has no `MaxSize`; the
/// wire bytes are the same.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[serde(transparent)]
pub struct Macro(pub heapless::Vec<MacroOp, MACRO_MAX_SIZE>);

impl Macro {
    /// `Err` when `ops` has more than `MACRO_MAX_SIZE` ops.
    pub fn from_slice(ops: &[MacroOp]) -> Result<Self, heapless::CapacityError> {
        heapless::Vec::from_slice(ops).map(Self)
    }
}

impl Deref for Macro {
    type Target = [MacroOp];

    fn deref(&self) -> &[MacroOp] {
        &self.0
    }
}

impl MaxSize for Macro {
    const POSTCARD_MAX_SIZE: usize = crate::heapless_vec_max_size::<MacroOp, MACRO_MAX_SIZE>();
}

/// Whether one macro can be stored and run: at most `MACRO_MAX_SIZE` ops, at
/// most one [`MacroOp::PauseForRelease`], and every [`MacroOp::Char`] ASCII.
pub const fn validate_macro(ops: &[MacroOp]) -> bool {
    if ops.len() > MACRO_MAX_SIZE {
        return false;
    }
    let mut pauses = 0;
    let mut i = 0;
    while i < ops.len() {
        match ops[i] {
            MacroOp::Char(c) if !c.is_ascii() => return false,
            MacroOp::PauseForRelease => {
                pauses += 1;
                if pauses > 1 {
                    return false;
                }
            }
            _ => {}
        }
        i += 1;
    }
    true
}

/// Whether a default macro table can be compiled in: at most `MACRO_MAX_NUM`
/// macros, each passing [`validate_macro`].
pub const fn validate_default_macros(macros: &[&[MacroOp]]) -> bool {
    if macros.len() > MACRO_MAX_NUM {
        return false;
    }
    let mut i = 0;
    while i < macros.len() {
        if !validate_macro(macros[i]) {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keycode::{HidKeyCode, KeyCode};
    use crate::modifier::ModifierCombination;

    const A: MacroOp = MacroOp::Tap(Action::Key(KeyCode::Hid(HidKeyCode::A)));

    #[test]
    fn every_rule_of_validate_macro() {
        assert!(validate_macro(&[]));
        assert!(validate_macro(&[
            MacroOp::Press(Action::KeyWithModifier(HidKeyCode::A, ModifierCombination::LCTRL)),
            MacroOp::Delay(u16::MAX),
            MacroOp::Char(b'z'),
            MacroOp::PauseForRelease,
            MacroOp::Release(Action::LayerOn(1)),
            MacroOp::Tap(Action::TriggerMacro(0)),
        ]));

        assert!(!validate_macro(&[
            MacroOp::PauseForRelease,
            A,
            MacroOp::PauseForRelease
        ]));
        assert!(!validate_macro(&[MacroOp::Char(0x80)]));
        assert!(!validate_macro(&[A; MACRO_MAX_SIZE + 1]));
        assert!(validate_macro(&[A; MACRO_MAX_SIZE]));
    }

    #[test]
    fn every_rule_of_validate_default_macros() {
        assert!(validate_default_macros(&[]));
        assert!(validate_default_macros(&[&[A], &[]]));
        assert!(!validate_default_macros(&[&[A], &[MacroOp::Char(0xff)]]));
        const EMPTY: &[MacroOp] = &[];
        assert!(!validate_default_macros(&[EMPTY; MACRO_MAX_NUM + 1]));
        assert!(validate_default_macros(&[EMPTY; MACRO_MAX_NUM]));
    }

    // The const fns are what `const _: () = assert!(..)` evaluates at compile time.
    const _: () = assert!(validate_default_macros(&[&[
        A,
        MacroOp::Char(b'a'),
        MacroOp::PauseForRelease
    ]]));

    #[test]
    fn macro_round_trips_through_postcard_within_its_bound() {
        let mut ops = heapless::Vec::new();
        while ops
            .push(MacroOp::Tap(Action::KeyWithModifier(
                HidKeyCode::RGui,
                ModifierCombination::RGUI,
            )))
            .is_ok()
        {}
        let full = Macro(ops);
        let mut buf = [0u8; 2 * Macro::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(&full, &mut buf).unwrap();
        assert!(bytes.len() <= Macro::POSTCARD_MAX_SIZE);
        assert_eq!(postcard::from_bytes::<Macro>(bytes).unwrap(), full);
        assert_eq!(MacroOp::POSTCARD_MAX_SIZE, 5, "one op is at most five wire bytes");

        // A bare op vector and its newtype share one encoding.
        let macro_ops =
            Macro::from_slice(&[A, MacroOp::Delay(300), MacroOp::Char(b'a'), MacroOp::PauseForRelease]).unwrap();
        let mut raw = [0u8; 32];
        let raw = postcard::to_slice(&macro_ops.0, &mut raw).unwrap();
        let mut wrapped = [0u8; 32];
        assert_eq!(postcard::to_slice(&macro_ops, &mut wrapped).unwrap(), raw);
        assert_eq!(raw, [4, 0, 1, 0, 4, 3, 172, 2, 4, 97, 5]);
    }
}
