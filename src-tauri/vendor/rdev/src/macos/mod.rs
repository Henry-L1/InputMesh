mod common;
mod display;
#[cfg(feature = "unstable_grab")]
mod grab;
mod keyboard;
mod keycodes;
mod listen;
mod simulate;

pub(crate) const SYNTHETIC_EVENT_MARKER: i64 = 0x494E_5055_544D_4553;

pub use crate::macos::display::display_size;
#[cfg(feature = "unstable_grab")]
pub use crate::macos::grab::grab;
pub use crate::macos::keyboard::Keyboard;
pub use crate::macos::listen::listen;
pub use crate::macos::simulate::simulate;
