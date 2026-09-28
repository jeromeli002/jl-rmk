//! Macro handlers: one whole macro per request.

use rmk_types::keyboard_macros::{Macro, validate_macro};
use rmk_types::protocol::rynk::command::{GetMacro, SetMacro};
use rmk_types::protocol::rynk::{RynkError, SetMacroRequest};

use super::super::RynkService;
use super::Handle;
use crate::MACRO_MAX_NUM;

impl Handle<GetMacro> for RynkService<'_> {
    async fn handle(&self, idx: u8) -> Result<Macro, RynkError> {
        if idx as usize >= MACRO_MAX_NUM {
            return Err(RynkError::Invalid);
        }
        self.ctx
            .keymap
            .read_macro(idx)
            .await
            .map_err(|()| RynkError::StorageFault)
    }
}

impl Handle<SetMacro> for RynkService<'_> {
    async fn handle(&self, r: SetMacroRequest) -> Result<(), RynkError> {
        // A macro with too many ops never decodes: `serve` answers `Malformed` first.
        if r.index as usize >= MACRO_MAX_NUM || !validate_macro(&r.macro_ops) {
            return Err(RynkError::Invalid);
        }
        #[cfg(feature = "storage")]
        {
            crate::storage::store_macro(r.index, |slot| *slot = r.macro_ops)
                .await
                .map_err(|()| RynkError::StorageFault)
        }
        #[cfg(not(feature = "storage"))]
        {
            Err(RynkError::Unimplemented)
        }
    }
}
