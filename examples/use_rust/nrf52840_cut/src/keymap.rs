use rmk::action::{EncoderAction, KeyAction};
use rmk::{a, encoder, k, layer, mo};
pub(crate) const COL: usize = 4;
pub(crate) const ROW: usize = 4;
pub(crate) const NUM_LAYER: usize = 8;
pub(crate) const NUM_ENCODER: usize = 2;

#[rustfmt::skip]
pub const fn get_default_keymap() -> [[[KeyAction; COL]; ROW]; NUM_LAYER] {
    [
        layer!([
            [k!(Tab), k!(Q), k!(W), k!(E)],
            [k!(Escape), k!(A), k!(S), k!(D)],
            [k!(LShift), k!(Z), k!(X), k!(C)],
            [k!(LCtrl), k!(LGui), k!(LAlt), a!(No)]
        ]),
        layer!([
            [k!(Tab), k!(Q), k!(W), k!(E)],
            [k!(Escape), k!(A), k!(S), k!(D)],
            [k!(LShift), k!(Z), k!(X), k!(C)],
            [k!(LCtrl), k!(LGui), k!(LAlt), a!(No)]
        ]),
        layer!([
            [k!(Tab), k!(Q), k!(W), k!(E)],
            [k!(Escape), k!(A), k!(S), k!(D)],
            [k!(LShift), k!(Z), k!(X), k!(C)],
            [k!(LCtrl), k!(LGui), k!(LAlt), a!(No)]
        ]),
        layer!([
            [k!(Tab), k!(Q), k!(W), k!(E)],
            [k!(Escape), k!(A), k!(S), k!(D)],
            [k!(LShift), k!(Z), k!(X), k!(C)],
            [k!(LCtrl), k!(LGui), k!(LAlt), a!(No)]
        ]),
        layer!([
            [k!(Tab), k!(Q), k!(W), k!(E)],
            [k!(Escape), k!(A), k!(S), k!(D)],
            [k!(LShift), k!(Z), k!(X), k!(C)],
            [k!(LCtrl), k!(LGui), k!(LAlt), a!(No)]
        ]),
        layer!([
            [k!(Tab), k!(Q), k!(W), k!(E)],
            [k!(Escape), k!(A), k!(S), k!(D)],
            [k!(LShift), k!(Z), k!(X), k!(C)],
            [k!(LCtrl), k!(LGui), k!(LAlt), a!(No)]
        ]),
        layer!([
            [k!(Tab), k!(Q), k!(W), k!(E)],
            [k!(Escape), k!(A), k!(S), k!(D)],
            [k!(LShift), k!(Z), k!(X), k!(C)],
            [k!(LCtrl), k!(LGui), k!(LAlt), a!(No)]
        ]),
        layer!([
            [k!(Tab), k!(Q), k!(W), k!(E)],
            [k!(Escape), k!(A), k!(S), k!(D)],
            [k!(LShift), k!(Z), k!(X), k!(C)],
            [k!(LCtrl), k!(LGui), k!(LAlt), a!(No)]
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
