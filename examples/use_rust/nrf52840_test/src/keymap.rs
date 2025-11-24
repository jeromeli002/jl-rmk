use rmk::types::action::{EncoderAction, KeyAction};
use rmk::{a, encoder, k, layer, mo};
pub(crate) const COL: usize = 2;
pub(crate) const ROW: usize = 2;
pub(crate) const NUM_LAYER: usize = 8;
pub(crate) const NUM_ENCODER: usize = 2;

#[rustfmt::skip]
pub const fn get_default_keymap() -> [[[KeyAction; COL]; ROW]; NUM_LAYER] {
    [
        layer!([
            [k!(Grave), k!(Kc1)],
            [k!(LCtrl), k!(LGui)]
        ]),
        layer!([
            [k!(Grave), k!(Kc1)],
            [k!(LCtrl), k!(LGui)]
        ]),
        layer!([
            [k!(Grave), k!(Kc1)],
            [k!(LCtrl), k!(LGui)]
        ]),
        layer!([
            [k!(Grave), k!(Kc1)],
            [k!(LCtrl), k!(LGui)]
        ]),
        layer!([
           [k!(Grave), k!(Kc1)],
            [k!(LCtrl), k!(LGui)]
        ]),
        layer!([
           [k!(Grave), k!(Kc1)],
            [k!(LCtrl), k!(LGui)]
        ]),
        layer!([
            [k!(Grave), k!(Kc1)],
            [k!(LCtrl), k!(LGui)]
        ]),
        layer!([
           [k!(Grave), k!(Kc1)],
            [k!(LCtrl), k!(LGui)]
        ]),
    ]
}

pub const fn get_default_encoder_map() -> [[EncoderAction; NUM_ENCODER]; NUM_LAYER] {
    [
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
    ]
}
