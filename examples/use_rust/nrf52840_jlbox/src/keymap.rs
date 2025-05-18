use rmk::action::{EncoderAction, KeyAction};
use rmk::{a, encoder, k, layer, mo};
pub(crate) const COL: usize = 5;
pub(crate) const ROW: usize = 5;
pub(crate) const NUM_LAYER: usize = 8;
pub(crate) const NUM_ENCODER: usize = 5;

#[rustfmt::skip]
pub const fn get_default_keymap() -> [[[KeyAction; COL]; ROW]; NUM_LAYER] {
    [
        layer!([
            [mo!(1), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)]
        ]),
        layer!([
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)]
        ]),
        layer!([
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)]
        ]),
        layer!([
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)]
        ]),
        layer!([
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)]
        ]),
        layer!([
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)]
        ]),
        layer!([
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)]
        ]),
        layer!([
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)],
            [a!(No), a!(No), a!(No), a!(No), a!(No)]
        ]),
    ]
}

pub const fn get_default_encoder_map() -> [[EncoderAction; NUM_ENCODER]; NUM_LAYER] {
    [
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
        [
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
            encoder!(k!(KbVolumeUp), k!(KbVolumeDown)),
        ],
    ]
}
