//! Via/Vial host exchanges and their live-keyboard integration tests.

use rmk::config::BehaviorConfig;
use rmk::test_support::{test_block_on, to_via_keycode};
use rmk::types::action::{Action, EncoderAction, KeyAction};
use rmk::types::constants::{MACRO_MAX_NUM, MACRO_MAX_SIZE};
use rmk::types::keyboard_macros::MacroOp;
use rmk::types::keycode::{HidKeyCode, KeyCode};
use rmk::{k, macros, text};
use rmk_types::protocol::vial::{
    SettingKey, VIA_PROTOCOL_VERSION, VIAL_EP_SIZE as REPORT, ViaCommand, VialCommand, VialDynamic,
};

use crate::simulator::SimKeyboard;

fn via(cmd: ViaCommand) -> [u8; REPORT] {
    let mut data = [0; REPORT];
    data[0] = cmd as u8;
    data
}

fn vial(cmd: VialCommand) -> [u8; REPORT] {
    let mut data = via(ViaCommand::Vial);
    data[1] = cmd as u8;
    data
}

fn dynamic(op: VialDynamic, index: u8) -> [u8; REPORT] {
    let mut data = vial(VialCommand::DynamicEntryOp);
    data[2..4].copy_from_slice(&[op as u8, index]);
    data
}

impl SimKeyboard {
    fn echo(&mut self, request: [u8; REPORT]) {
        self.host_exchange(request, request);
    }

    fn echo_with_status(&mut self, request: [u8; REPORT]) {
        let mut expected = request;
        expected[0] = 0;
        self.host_exchange(request, expected);
    }

    fn set_behavior(&mut self, setting: SettingKey, value: u16) {
        let mut request = vial(VialCommand::SetBehaviorSetting);
        request[2..4].copy_from_slice(&(setting as u16).to_le_bytes());
        request[4..6].copy_from_slice(&value.to_le_bytes());
        self.echo(request)
    }

    fn set_combo<const N: usize>(&mut self, index: u8, actions: [KeyAction; N], output: KeyAction) {
        let mut request = dynamic(VialDynamic::DynamicVialComboSet, index);
        const MAX: usize = rmk::test_support::COMBO_MAX_LENGTH;
        assert!(N <= MAX);
        for (idx, action) in actions.into_iter().enumerate() {
            let start = 4 + idx * 2;
            request[start..start + 2].copy_from_slice(&to_via_keycode(action).to_le_bytes());
        }
        request[4 + MAX * 2..6 + MAX * 2].copy_from_slice(&to_via_keycode(output).to_le_bytes());
        self.echo_with_status(request)
    }
}

#[test]
fn protocol_version_round_trips() {
    test_block_on(async {
        let mut keyboard = SimKeyboard::builder([[[k!(A)]]]).build().await;
        let request = via(ViaCommand::GetProtocolVersion);
        let mut reply = request;
        reply[1..3].copy_from_slice(&VIA_PROTOCOL_VERSION.to_be_bytes());
        keyboard.host_exchange(request, reply);
        keyboard.run().await;
    });
}

#[test]
fn keymap_write_changes_the_key() {
    test_block_on(async {
        let mut keyboard = SimKeyboard::builder([[[k!(A)]]]).build().await;
        let mut request = via(ViaCommand::DynamicKeymapSetKeyCode);
        request[1..4].copy_from_slice(&[0, 0, 0]);
        request[4..6].copy_from_slice(&to_via_keycode(k!(B)).to_be_bytes());
        keyboard.echo(request);
        keyboard
            .tap(0, 0, 10)
            .expect_keys([HidKeyCode::B])
            .expect_keys([])
            .run()
            .await;
    });
}

#[test]
fn encoder_write_changes_the_knob() {
    test_block_on(async {
        let action = EncoderAction::new(k!(C), k!(D));
        let mut keyboard = SimKeyboard::builder([[[k!(A)]]])
            .encoders([[EncoderAction::new(k!(A), k!(B))]])
            .build()
            .await;
        for (direction, key) in [(1, action.clockwise), (0, action.counter_clockwise)] {
            let mut request = vial(VialCommand::SetEncoder);
            request[2..5].copy_from_slice(&[0, 0, direction]);
            request[5..7].copy_from_slice(&to_via_keycode(key).to_be_bytes());
            keyboard.echo(request);
        }
        keyboard
            .rotary_cw(0)
            .expect_keys([HidKeyCode::C])
            .expect_keys([])
            .rotary_ccw(0)
            .expect_keys([HidKeyCode::D])
            .expect_keys([])
            .run()
            .await;
    });
}

#[test]
fn rejects_out_of_range_and_unknown_requests() {
    test_block_on(async {
        let mut keyboard = SimKeyboard::builder([[[k!(A)]]])
            .encoders([[EncoderAction::new(k!(A), k!(B))]])
            .build()
            .await;
        let mut request = vial(VialCommand::GetEncoder);
        request[2..4].copy_from_slice(&[0, 99]);
        keyboard.host_exchange(request, [0; REPORT]);
        keyboard.host_exchange(dynamic(VialDynamic::Unhandled, 0), [0; REPORT]);
        keyboard.run().await;
    });
}

#[test]
fn combo_and_behavior_writes_change_the_chord() {
    test_block_on(async {
        let mut keyboard = SimKeyboard::builder([[[k!(A), k!(B)]]]).build().await;
        keyboard.set_combo(0, [k!(A), k!(B)], k!(C));
        keyboard.set_behavior(SettingKey::ComboTimeout, 80);
        keyboard
            .press(0, 0)
            .expect_no_report(60)
            .expect_keys([HidKeyCode::A])
            .release(0, 0)
            .expect_keys([])
            .delay(20)
            .press(0, 0)
            .delay(10)
            .press(0, 1)
            .expect_keys([HidKeyCode::C])
            .release(0, 0)
            .release(0, 1)
            .expect_keys([])
            .run()
            .await;
    });
}

#[test]
fn tap_capslock_interval_reads_back_its_own_value() {
    test_block_on(async {
        let mut keyboard = SimKeyboard::builder([[[k!(A)]]]).build().await;
        keyboard.set_behavior(SettingKey::TapInterval, 180);
        keyboard.set_behavior(SettingKey::TapCapslockInterval, 240);
        let mut request = vial(VialCommand::GetBehaviorSetting);
        request[2..4].copy_from_slice(&(SettingKey::TapCapslockInterval as u16).to_le_bytes());
        let mut reply = [0xFF; REPORT];
        reply[0] = 0;
        reply[1..3].copy_from_slice(&240u16.to_le_bytes());
        keyboard.host_exchange(request, reply);
        keyboard.run().await;
    });
}

#[test]
fn morse_write_changes_the_tap() {
    test_block_on(async {
        let mut keyboard = SimKeyboard::builder([[[rmk::td!(0)]]]).build().await;
        let mut request = dynamic(VialDynamic::DynamicVialMorseSet, 0);
        for (idx, action) in [k!(A), k!(B), k!(C), k!(D)].into_iter().enumerate() {
            let start = 4 + idx * 2;
            request[start..start + 2].copy_from_slice(&to_via_keycode(action).to_le_bytes());
        }
        request[12..14].copy_from_slice(&80u16.to_le_bytes());
        keyboard.echo_with_status(request);
        keyboard
            .delay(150)
            .tap(0, 0, 20)
            .expect_keys([HidKeyCode::A])
            .expect_keys([])
            .run()
            .await;
    });
}

/// Macro 0 is `tap A` then a pause Vial cannot spell; macro 1 types `hi`.
const MACROS: &[&[MacroOp]] = &[
    &[
        MacroOp::Tap(Action::Key(KeyCode::Hid(HidKeyCode::A))),
        MacroOp::PauseForRelease,
    ],
    &text!("hi"),
];

/// A two-key keyboard, `MACRO(0)` and `MACRO(1)`, with [`MACROS`] compiled in.
fn macro_keyboard() -> crate::simulator::SimKeyboardBuilder<1, 2, 1, 0> {
    SimKeyboard::builder([[[macros!(0), macros!(1)]]]).behavior_config(BehaviorConfig {
        keyboard_macros: MACROS,
        ..Default::default()
    })
}

/// A `DynamicKeymapMacroGetBuffer`/`SetBuffer` report for `size` bytes at `offset`.
fn macro_buffer(cmd: ViaCommand, offset: u16, payload: &[u8]) -> [u8; REPORT] {
    let mut request = via(cmd);
    request[1..3].copy_from_slice(&offset.to_be_bytes());
    request[3] = payload.len() as u8;
    request[4..4 + payload.len()].copy_from_slice(payload);
    request
}

/// vial-gui reads the macros as one buffer in Vial's own encoding: every macro
/// rendered and `0x00`-terminated, an op Vial cannot spell left out, and the
/// rest of the view zero.
#[test]
fn macro_buffer_renders_the_macros() {
    test_block_on(async {
        let mut keyboard = macro_keyboard().build().await;
        let request = macro_buffer(ViaCommand::DynamicKeymapMacroGetBuffer, 0, &[0; 28]);
        let reply = macro_buffer(
            ViaCommand::DynamicKeymapMacroGetBuffer,
            0,
            &[
                0x01, 0x01, 0x04, 0x00, // tap A, the pause left out
                b'h', b'i', 0x00, // hi
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        keyboard.host_exchange(request, reply);
        keyboard.run().await;
    });
}

/// Save [`MACROS`] the way vial-gui does, with slot 0 changed to `tap B`: every
/// macro from offset 0, each `0x00`-terminated, without padding, the last
/// packet sent twice when `resend_last`. The packets are 8 bytes rather than
/// vial-gui's 28 so that the resent one lands past offset 0.
#[cfg(feature = "storage")]
async fn save_from_vial_gui(resend_last: bool) -> (SimKeyboard, crate::simulator::Flash) {
    let flash = crate::simulator::Flash::new();
    let mut keyboard = macro_keyboard().build_with_flash(flash.clone()).await;
    // Slot 0 becomes `tap B`, slot 1 is sent back as read, the rest are empty.
    let mut data = vec![0x01, 0x01, 0x05, 0x00, b'h', b'i', 0x00];
    data.resize(7 + MACRO_MAX_NUM - 2, 0);
    let packets: Vec<_> = data
        .chunks(8)
        .enumerate()
        .map(|(i, chunk)| macro_buffer(ViaCommand::DynamicKeymapMacroSetBuffer, i as u16 * 8, chunk))
        .collect();
    for packet in &packets {
        keyboard.echo(*packet);
    }
    if resend_last {
        keyboard.echo(packets[1]);
    }
    keyboard.run().await;
    (keyboard, flash)
}

/// Only the slot the user changed reaches the flash: a save writes as much as
/// one `SetMacro`, a resent packet writes nothing, and after a restart the
/// saved macro still wins.
#[cfg(feature = "storage")]
#[test]
fn macro_save_from_vial_gui_writes_only_the_changed_slot() {
    test_block_on(async {
        let (_, resent) = save_from_vial_gui(true).await;
        let one_write = {
            let flash = crate::simulator::Flash::new();
            let mut keyboard = macro_keyboard().build_with_flash(flash.clone()).await;
            keyboard.echo(macro_buffer(
                ViaCommand::DynamicKeymapMacroSetBuffer,
                0,
                &[0x01, 0x01, 0x05, 0x00],
            ));
            keyboard.run().await;
            flash.writes()
        };
        let flash = {
            let (mut keyboard, flash) = save_from_vial_gui(false).await;
            assert_eq!(flash.writes(), one_write, "only slot 0 is written");
            assert_eq!(resent.writes(), flash.writes(), "a resent packet writes nothing");
            keyboard
                .tap(0, 0, 10)
                .expect_keys([HidKeyCode::B])
                .expect_keys([])
                .tap(0, 1, 10)
                .expect_keys([HidKeyCode::H])
                .expect_keys([])
                .expect_keys([HidKeyCode::I])
                .expect_keys([])
                .expect_keys([])
                .run()
                .await;
            flash
        };
        let mut keyboard = macro_keyboard().build_with_flash(flash).await;
        keyboard
            .tap(0, 0, 10)
            .expect_keys([HidKeyCode::B])
            .expect_keys([])
            .run()
            .await;
    });
}

/// A reset clears the slots that render to something and leaves the empty ones
/// alone, so a second reset has nothing left to write.
#[cfg(feature = "storage")]
#[test]
fn macro_reset_clears_the_slots_that_have_macros() {
    test_block_on(async {
        let flash = crate::simulator::Flash::new();
        let mut keyboard = macro_keyboard().build_with_flash(flash.clone()).await;
        keyboard.echo(via(ViaCommand::DynamicKeymapMacroReset));
        keyboard.tap(0, 0, 10).run().await;
        let cleared = flash.writes();
        assert!(cleared > 0, "the two macros are cleared");
        keyboard.echo(via(ViaCommand::DynamicKeymapMacroReset));
        keyboard.run().await;
        assert_eq!(flash.writes(), cleared, "nothing left to clear");
    });
}

#[cfg(feature = "storage")]
#[test]
fn behavior_write_survives_restart() {
    test_block_on(async {
        let flash = crate::simulator::Flash::new();
        {
            let mut keyboard = SimKeyboard::builder([[[k!(A), k!(B)]]])
                .build_with_flash(flash.clone())
                .await;
            keyboard.set_behavior(SettingKey::ComboTimeout, 80);
            keyboard.set_combo(0, [k!(A), k!(B)], k!(C));
            keyboard.run().await;
        }
        let mut keyboard = SimKeyboard::builder([[[k!(A), k!(B)]]]).build_with_flash(flash).await;
        keyboard
            .press(0, 0)
            .expect_no_report(60)
            .expect_keys([HidKeyCode::A])
            .release(0, 0)
            .expect_keys([])
            .run()
            .await;
    });
}

/// Save `data` from offset 0 in 28-byte packets, the way vial-gui does.
fn save_macro_buffer(keyboard: &mut SimKeyboard, data: &[u8]) {
    for (i, chunk) in data.chunks(28).enumerate() {
        keyboard.echo(macro_buffer(
            ViaCommand::DynamicKeymapMacroSetBuffer,
            i as u16 * 28,
            chunk,
        ));
    }
}

/// Vial is told the buffer holds `MACRO_MAX_NUM + MACRO_MAX_SIZE` bytes, and a
/// save that spends all of it on one macro of single-byte ops is kept whole.
#[cfg(feature = "storage")]
#[test]
fn the_longest_macro_vial_can_save_is_kept() {
    const SPACE: usize = MACRO_MAX_NUM + MACRO_MAX_SIZE;
    test_block_on(async {
        let flash = crate::simulator::Flash::new();
        let mut keyboard = macro_keyboard().build_with_flash(flash).await;
        let mut size = via(ViaCommand::DynamicKeymapMacroGetBufferSize);
        size[1..3].copy_from_slice(&(SPACE as u16).to_be_bytes());
        keyboard.host_exchange(via(ViaCommand::DynamicKeymapMacroGetBufferSize), size);
        let mut data = vec![b'a'; MACRO_MAX_SIZE];
        data.resize(SPACE, 0);
        save_macro_buffer(&mut keyboard, &data);
        keyboard.tap(0, 0, 10);
        for _ in 0..MACRO_MAX_SIZE {
            keyboard.expect_keys([HidKeyCode::A]).expect_keys([]);
        }
        keyboard.expect_keys([]).run().await;
    });
}

/// Three macros of `TAPS` taps do not fit the view: slot 2 is shown empty. A
/// save that sends it back empty leaves it alone; one that fills it writes it.
#[cfg(feature = "storage")]
#[test]
fn a_slot_vial_could_not_show_changes_only_when_filled() {
    // Two macros of `3 * TAPS + 1` bytes fit the view, a third does not.
    const TAPS: usize = (MACRO_MAX_NUM + MACRO_MAX_SIZE).div_ceil(9);
    test_block_on(async {
        const TAP_A: MacroOp = MacroOp::Tap(Action::Key(KeyCode::Hid(HidKeyCode::A)));
        const BIG: &[MacroOp] = &[TAP_A; TAPS];
        let flash = crate::simulator::Flash::new();
        let mut keyboard = SimKeyboard::builder([[[macros!(0), macros!(1), macros!(2)]]])
            .behavior_config(BehaviorConfig {
                keyboard_macros: &[BIG; 3],
                ..Default::default()
            })
            .build_with_flash(flash.clone())
            .await;
        keyboard.run().await;
        let before = flash.writes();
        // vial-gui reads the view, then saves it back: slots 0 and 1 as rendered,
        // slot 2 and the rest empty because the view could not fit them.
        let mut first_page = [0x01, 0x01, 0x04].repeat(10);
        first_page.truncate(28);
        keyboard.host_exchange(
            macro_buffer(ViaCommand::DynamicKeymapMacroGetBuffer, 0, &[0; 28]),
            macro_buffer(ViaCommand::DynamicKeymapMacroGetBuffer, 0, &first_page),
        );
        let mut shown = Vec::new();
        for _ in 0..2 {
            shown.extend([0x01, 0x01, 0x04].repeat(TAPS));
            shown.push(0);
        }
        let mut data = shown.clone();
        data.resize(shown.len() + MACRO_MAX_NUM - 2, 0);
        save_macro_buffer(&mut keyboard, &data);
        keyboard.run().await;
        assert_eq!(flash.writes(), before, "nothing changed, so nothing is written");
        keyboard.tap(0, 2, 10);
        for _ in 0..TAPS {
            keyboard.expect_keys([HidKeyCode::A]).expect_keys([]);
        }
        keyboard.run().await;

        // Slot 2 becomes `tap B`.
        let mut data = shown;
        data.extend([0x01, 0x01, 0x05, 0x00]);
        data.resize(data.len() + MACRO_MAX_NUM - 3, 0);
        save_macro_buffer(&mut keyboard, &data);
        keyboard
            .tap(0, 2, 10)
            .expect_keys([HidKeyCode::B])
            .expect_keys([])
            .run()
            .await;
    });
}
