//! Vial's view of the macros: a byte buffer in Vial's own macro encoding.
//!
//! Vial reads the whole buffer, edits it, and writes it back from offset 0 with
//! every macro `0x00`-terminated and only the bytes it has, so a save arrives as
//! a stream of segments. Each terminator commits the segment before it to the
//! next slot, skipping a slot whose bytes still equal what the keyboard renders
//! for it, so only macros the user changed reach the flash. The view is lossy:
//! an op Vial cannot spell is left out when rendering.

use core::cell::RefCell;

use rmk_types::action::{Action, KeyAction};
use rmk_types::keyboard_macros::{Macro, MacroOp};
use rmk_types::keycode::{HidKeyCode, from_ascii};

use super::keycode_convert::{from_via_keycode, to_via_keycode};
use crate::host::context::KeyboardContext;
use crate::{MACRO_MAX_NUM, MACRO_MAX_SIZE};

/// The buffer size reported to Vial. vial-gui checks a save only against this
/// total and every macro costs its terminator, so one macro gets at most
/// `MACRO_MAX_SIZE` bytes; a byte parses to at most one op, so every save fits.
pub(crate) const MACRO_SPACE_SIZE: usize = MACRO_MAX_NUM + MACRO_MAX_SIZE;

/// The longest delay one Vial delay op holds: two 1-based bytes.
const VIAL_DELAY_MAX: u16 = 254 * 255 + 254;

pub(crate) struct MacroView {
    buf: [u8; MACRO_SPACE_SIZE],
    /// Where the segment Vial is still sending starts, and the slot it lands in.
    seg: u16,
    idx: u8,
    /// The first slot the last full read could not fit: Vial shows it and the
    /// rest as empty, so an empty macro saved there means unchanged.
    hidden_from: u8,
}

impl MacroView {
    pub(crate) const fn new() -> Self {
        Self {
            buf: [0; MACRO_SPACE_SIZE],
            seg: 0,
            idx: 0,
            hidden_from: MACRO_MAX_NUM as u8,
        }
    }
}

/// Fill `out` with the view from `offset`. Offset 0 renders every macro first,
/// the way Vial starts each read.
pub(crate) async fn read(ctx: &KeyboardContext<'_>, view: &RefCell<MacroView>, offset: usize, out: &mut [u8]) {
    if offset == 0 {
        let mut cursor = 0;
        view.borrow_mut().hidden_from = MACRO_MAX_NUM as u8;
        for idx in 0..MACRO_MAX_NUM as u8 {
            let macro_ops = ctx.keymap.read_macro(idx).await.unwrap_or_default();
            let mut view = view.borrow_mut();
            // A macro that does not fit ends the view: Vial sees it and the rest as empty.
            if cursor + render(&macro_ops).count() >= view.buf.len() {
                warn!(
                    "Vial shows macros from {} on as empty, raise macro_max_size to show them",
                    idx
                );
                view.hidden_from = idx;
                break;
            }
            for byte in render(&macro_ops) {
                view.buf[cursor] = byte;
                cursor += 1;
            }
            view.buf[cursor] = 0;
            cursor += 1;
        }
        view.borrow_mut().buf[cursor..].fill(0);
    }
    let view = view.borrow();
    let end = (offset + out.len()).min(view.buf.len());
    out.fill(0);
    if offset < end {
        out[..end - offset].copy_from_slice(&view.buf[offset..end]);
    }
}

/// Take `data`, written at `offset`, and commit every macro it terminates.
pub(crate) async fn write(ctx: &KeyboardContext<'_>, view: &RefCell<MacroView>, offset: usize, data: &[u8]) {
    let end = {
        let mut view = view.borrow_mut();
        if offset == 0 {
            view.seg = 0;
            view.idx = 0;
        }
        let end = (offset + data.len()).min(view.buf.len());
        if offset < end {
            view.buf[offset..end].copy_from_slice(&data[..end - offset]);
        }
        end
    };
    loop {
        // A resent packet holds no terminator past `seg`, so it commits nothing twice.
        let segment = {
            let view = view.borrow();
            let start = view.seg as usize;
            if view.idx as usize >= MACRO_MAX_NUM || start >= end {
                return;
            }
            view.buf[start..end]
                .iter()
                .position(|&byte| byte == 0)
                .map(|len| (start, start + len, view.idx, view.hidden_from))
        };
        let Some((start, stop, idx, hidden_from)) = segment else {
            return;
        };
        if idx < hidden_from || stop > start {
            commit(ctx, view, idx, start, stop).await;
        }
        let mut view = view.borrow_mut();
        view.seg = stop as u16 + 1;
        view.idx += 1;
    }
}

/// Commit `buf[start..stop]` to macro `idx`, unless the bytes still equal what
/// this keyboard renders for the slot, or parse to what it already holds.
pub(crate) async fn commit(ctx: &KeyboardContext<'_>, view: &RefCell<MacroView>, idx: u8, start: usize, stop: usize) {
    // Scoped so neither macro outlives the check into the write's await.
    {
        let current = ctx.keymap.read_macro(idx).await;
        let view = view.borrow();
        let bytes = &view.buf[start..stop];
        let macro_ops = parse(bytes);
        if let Ok(current) = current
            && (macro_ops.as_ref() == Ok(&current) || render(&current).eq(bytes.iter().copied()))
        {
            return;
        }
        if macro_ops.is_err() {
            warn!("Macro {} from Vial is malformed or too long, ignored", idx);
            return;
        }
    }
    #[cfg(feature = "storage")]
    if crate::storage::store_macro(idx, |slot| {
        *slot = parse(&view.borrow().buf[start..stop]).unwrap_or_default();
    })
    .await
    .is_err()
    {
        error!("Failed to save macro {}", idx);
    }
    #[cfg(not(feature = "storage"))]
    warn!("Macro {} not saved: macros are read-only without storage", idx);
}

/// Vial's bytes for `ops`, without the terminator.
fn render(ops: &[MacroOp]) -> impl Iterator<Item = u8> + '_ {
    ops.iter().flat_map(|op| {
        let mut out = heapless::Vec::new();
        match *op {
            MacroOp::Tap(action) => render_key(&mut out, 1, action),
            MacroOp::Press(action) => render_key(&mut out, 2, action),
            MacroOp::Release(action) => render_key(&mut out, 3, action),
            MacroOp::Delay(mut ms) => loop {
                let chunk = ms.min(VIAL_DELAY_MAX);
                let _ = out.extend_from_slice(&[0x01, 0x04, (chunk % 255) as u8 + 1, (chunk / 255) as u8 + 1]);
                ms -= chunk;
                if ms == 0 {
                    break;
                }
            },
            MacroOp::Char(c) => {
                if c > 0x01 {
                    let _ = out.push(c);
                }
            }
            MacroOp::PauseForRelease => {}
        }
        out
    })
}

/// `01 kind kc` for a keycode Vial keeps in one byte, `01 kind+4 lo hi` for the
/// rest; nothing for an action Vial has no keycode for.
fn render_key(out: &mut heapless::Vec<u8, 8>, kind: u8, action: Action) {
    let keycode = to_via_keycode(KeyAction::Single(action));
    if keycode == 0 {
        return;
    }
    if keycode < 0x100 {
        let _ = out.extend_from_slice(&[0x01, kind, keycode as u8]);
    } else {
        // Vial escapes a zero low byte so the payload never holds a terminator.
        let word = if keycode & 0xFF == 0 {
            0xFF00 | (keycode >> 8)
        } else {
            keycode
        };
        let [lo, hi] = word.to_le_bytes();
        let _ = out.extend_from_slice(&[0x01, kind + 4, lo, hi]);
    }
}

/// The macro in `bytes`, one Vial-terminated segment without its terminator. A
/// byte that types nothing and a keycode with no action here are dropped;
/// `Err` for bytes that are not the encoding, or more ops than a macro holds.
fn parse(bytes: &[u8]) -> Result<Macro, ()> {
    let mut ops = heapless::Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let op = if bytes[i] == 0x01 {
            let at = |offset: usize| bytes.get(i + offset).copied().ok_or(());
            let (op, len) = match at(1)? {
                kind @ 1..=3 => (key_op(kind, at(2)? as u16), 3),
                4 => {
                    let ms = (at(2)?.max(1) - 1) as u16 + (at(3)?.max(1) - 1) as u16 * 255;
                    (Some(MacroOp::Delay(ms)), 4)
                }
                kind @ 5..=7 => {
                    let raw = u16::from_le_bytes([at(2)?, at(3)?]);
                    let keycode = if raw & 0xFF00 == 0xFF00 { (raw & 0xFF) << 8 } else { raw };
                    (key_op(kind - 4, keycode), 4)
                }
                _ => return Err(()),
            };
            i += len;
            op
        } else {
            let c = bytes[i];
            i += 1;
            (from_ascii(c).0 != HidKeyCode::No).then_some(MacroOp::Char(c))
        };
        if let Some(op) = op {
            ops.push(op).map_err(|_| ())?;
        }
    }
    Ok(Macro(ops))
}

/// `None` for a keycode with no action here.
fn key_op(kind: u8, keycode: u16) -> Option<MacroOp> {
    match from_via_keycode(keycode).to_action() {
        Action::No => None,
        action => Some(match kind {
            1 => MacroOp::Tap(action),
            2 => MacroOp::Press(action),
            _ => MacroOp::Release(action),
        }),
    }
}

#[cfg(test)]
mod tests {
    use rmk_types::keycode::KeyCode;
    use rmk_types::modifier::ModifierCombination;

    use super::*;

    fn bytes(ops: &[MacroOp]) -> std::vec::Vec<u8> {
        render(ops).collect()
    }

    #[test]
    fn every_op_shape_renders_and_parses_back() {
        let ops = [
            MacroOp::Tap(Action::Key(KeyCode::Hid(HidKeyCode::A))),
            MacroOp::Press(Action::Key(KeyCode::Hid(HidKeyCode::LShift))),
            MacroOp::Release(Action::Key(KeyCode::Hid(HidKeyCode::LShift))),
            MacroOp::Delay(100),
            MacroOp::Char(b'x'),
            MacroOp::Tap(Action::KeyWithModifier(HidKeyCode::A, ModifierCombination::LCTRL)),
            // 0x7E00: the zero low byte is escaped.
            MacroOp::Tap(Action::User(0)),
            MacroOp::Press(Action::PersistentDefaultLayer(1)),
        ];
        let rendered = bytes(&ops);
        assert_eq!(
            rendered,
            [
                0x01, 0x01, 0x04, // tap A
                0x01, 0x02, 0xE1, // press LShift
                0x01, 0x03, 0xE1, // release LShift
                0x01, 0x04, 0x65, 0x01, // 100 ms
                b'x', // a character is its own byte
                0x01, 0x05, 0x04, 0x01, // WM(A, LCtrl) = 0x0104, little-endian
                0x01, 0x05, 0x7E, 0xFF, // User(0) = 0x7E00 escaped
                0x01, 0x06, 0xE1, 0x52, // PDF(1) = 0x52E1
            ]
        );
        assert_eq!(parse(&rendered).unwrap().as_ref(), &ops);
    }

    #[test]
    fn delays_split_at_the_encoding_limit_and_round_trip() {
        for ms in [0, 1, 254, 255, 65024, 65025, u16::MAX] {
            let rendered = bytes(&[MacroOp::Delay(ms)]);
            let parsed = parse(&rendered).unwrap();
            let total: u32 = parsed
                .iter()
                .map(|op| match op {
                    MacroOp::Delay(ms) => *ms as u32,
                    _ => panic!("not a delay"),
                })
                .sum();
            assert_eq!(total, ms as u32, "{ms}ms");
            assert!(parsed.len() <= 2);
        }
    }

    #[test]
    fn ops_vial_cannot_spell_are_left_out() {
        let ops = [
            MacroOp::PauseForRelease,
            MacroOp::Char(0x00),
            MacroOp::Char(0x01),
            MacroOp::Tap(Action::Key(KeyCode::Hid(HidKeyCode::B))),
        ];
        assert_eq!(bytes(&ops), [0x01, 0x01, 0x05]);
    }

    #[test]
    fn parse_drops_what_it_cannot_run_and_rejects_bad_bytes() {
        // A macro trigger is kept; an unmapped keycode and a control character are dropped.
        assert_eq!(
            parse(&[0x01, 0x05, 0x02, 0x77, 0x01, 0x01, 0x00, 0x07, b'a'])
                .unwrap()
                .as_ref(),
            &[MacroOp::Tap(Action::TriggerMacro(2)), MacroOp::Char(b'a')]
        );
        assert!(parse(&[0x01, 0x09, 0x00]).is_err(), "unknown kind");
        assert!(parse(&[0x01, 0x01]).is_err(), "truncated");
        assert!(parse(&[0x01, 0x04, 0x01]).is_err(), "truncated delay");
        let too_long = vec![b'a'; crate::MACRO_MAX_SIZE + 1];
        assert!(parse(&too_long).is_err());
    }
}
