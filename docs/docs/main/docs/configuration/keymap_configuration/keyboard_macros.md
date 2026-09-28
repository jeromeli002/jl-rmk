# Keyboard Macros

A keyboard macro is a list of steps that runs when you press its key: tap keys, hold and release them, type text, wait, or wait for the key to be released.

Macros come from two places:

- **Default macros** are defined in `keyboard.toml` or in Rust and built into the firmware.
- **Dynamic macros** are saved by a host tool, [Rynk](../../features/rynk) or Vial. A dynamic macro replaces the default macro with the same number.

## Operations

| Operation | `keyboard.toml` | Rust | What it does |
| --- | --- | --- | --- |
| Tap | `{ operation = "tap", keycode = "A" }` | `MacroOp::Tap(action)` | Press and release the key |
| Press | `{ operation = "down", keycode = "LShift" }` | `MacroOp::Press(action)` | Press and hold the key until an `up` step releases it or you press the key yourself, even after the macro ends |
| Release | `{ operation = "up", keycode = "LShift" }` | `MacroOp::Release(action)` | Release the key |
| Delay | `{ operation = "delay", duration = "100ms" }` | `MacroOp::Delay(100)` | Wait, at most 65535 ms |
| Text | `{ operation = "text", text = "Hello" }` | `text!("Hello")` | Type ASCII text; modifiers you hold don't change it |
| Pause for release | `{ operation = "pause_for_release" }` | `MacroOp::PauseForRelease` | Run the steps after it when the macro key is released. Without it, the whole macro runs on press |

`keycode` takes a keycode or a single action such as `WM(A, LCtrl)`, `MO(1)` or `Macro(n)`. Tap-hold and tap-dance keys (`MT`, `LT`, `TH`, `TT`, `TD`) are not accepted.

## Limits

- A macro has at most `macro_max_size` operations (default 96); each text character counts as one.
- A macro has at most one pause for release.
- Text is ASCII only. For other characters, see [Special characters and unicode](./special_characters_and_unicode).
- There are at most `macro_max_num` macros (default 32).

Set `macro_max_size` and `macro_max_num` in the [`[rmk]`](../rmk_config#behavior-configuration) section. A default macro that breaks a limit fails the build, and a host tool can't save one.

## Defining macros

Macros are numbered from 0 in the order they are defined.

### In `keyboard.toml`

See the [`[behavior.macro]`](../behavior#macro) section.

### In Rust

Set `BehaviorConfig::keyboard_macros` to a `const` table, and check it with `validate_default_macros` so a broken limit fails the build:

```rust
use rmk::config::BehaviorConfig;
use rmk::text;
use rmk::types::action::Action;
use rmk::types::keyboard_macros::{MacroOp, validate_default_macros};
use rmk::types::keycode::{HidKeyCode, KeyCode};
use rmk::types::modifier::ModifierCombination;

const LSHIFT: Action = Action::Key(KeyCode::Hid(HidKeyCode::LShift));

const MACROS: &[&[MacroOp]] = &[
    // Macro 0 types "Hello"
    &text!("Hello"),
    // Macro 1 types "W", waits, then sends Ctrl+C
    &[
        MacroOp::Press(LSHIFT),
        MacroOp::Tap(Action::Key(KeyCode::Hid(HidKeyCode::W))),
        MacroOp::Release(LSHIFT),
        MacroOp::Delay(100),
        MacroOp::Tap(Action::KeyWithModifier(HidKeyCode::C, ModifierCombination::LCTRL)),
    ],
];
const _: () = assert!(validate_default_macros(MACROS));

let behavior_config = BehaviorConfig {
    keyboard_macros: MACROS,
    ..Default::default()
};
```

`text!("...")` expands to one `MacroOp::Char` per character. To mix text with other steps in one macro, write each character as `MacroOp::Char(b'a')`.

## Triggering a macro

Trigger macro `n` with `Macro(n)` in `keyboard.toml` or `macros!(n)` in Rust. A number with no macro does nothing.

The trigger is an ordinary action, `Action::TriggerMacro(n)`, so it works anywhere an action does, such as a tap-hold key:

```rust
// Trigger macro 1 when tapped, activate layer 1 when held
// (the third field selects the morse profile; u8::MAX = default profile)
KeyAction::TapHold(Action::TriggerMacro(1), Action::LayerOn(1), u8::MAX)
```

Macros run one at a time: a macro triggered while another is running waits its turn. A `Macro(n)` step works the same way, so macro `n` runs after the current macro, not in its place. Don't let a macro trigger itself, directly or through another macro: it repeats until the keyboard restarts.

## Editing macros from a host

Rynk and Vial save edited macros to flash. Without the `storage` feature, macros can't be edited.

Vial's macro memory is `macro_max_num + macro_max_size` bytes (default 128). A key step takes 3 or 4 bytes, a delay 4, a text character 1, and each macro 1 more. Vial can't show a pause for release, and it shows the macros in order until the memory is used; the macros after that appear empty. You can write a new macro into one of them, but to see or edit its steps, raise `macro_max_size` or use Rynk. Saving in Vial only writes the macros you changed, so the others keep the steps Vial can't show.

## Tips

### Type words with chords

Combos can trigger macros, so pressing a few keys together types a whole word. Here `T`+`Y` types `type`, and `G` right after turns it into `typing`. The combos only work on layer 1, so rolling over `T` and `Y` while typing doesn't fire them:

```rust
use rmk::keyboard::combo::{Combo, ComboConfig};

const MACROS: &[&[MacroOp]] = &[
    &text!("type"),
    &[
        MacroOp::Tap(Action::Key(KeyCode::Hid(HidKeyCode::Backspace))),
        MacroOp::Char(b'i'),
        MacroOp::Char(b'n'),
        MacroOp::Char(b'g'),
    ],
];
const _: () = assert!(validate_default_macros(MACROS));

// `combos` is a `[Option<Combo>; COMBO_MAX_NUM]` array; unused slots stay `None`.
let mut combo_config = CombosConfig::default();
combo_config.combos[0] = Some(Combo::new(ComboConfig::new([k!(T), k!(Y)], macros!(0), Some(1))));
combo_config.combos[1] = Some(Combo::new(ComboConfig::new([k!(G)], macros!(1), Some(1))));
```

### Capitalize with Shift

Text ignores the modifiers you hold, but a tap doesn't. To type `qu` normally and `Qu` with Shift held, tap the first letter and type the rest:

```rust
const MACROS: &[&[MacroOp]] = &[&[
    MacroOp::Tap(Action::Key(KeyCode::Hid(HidKeyCode::Q))),
    MacroOp::Char(b'u'),
]];
```
