use crate::common::SupportedConfigType;

impl SupportedConfigType for BuehlmannConfig {}

impl Default for BuehlmannConfig {
    fn default() -> Self {
        Self {
            gf: (100, 100)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BuehlmannConfig {
    gf: (u8, u8)
}

