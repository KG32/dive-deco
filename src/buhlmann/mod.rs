mod buhlmann_config;
mod buhlmann_model;
mod compartment;
mod zhl_values;

pub use buhlmann_config::BuhlmannConfig;
pub use buhlmann_model::{BuehlmannModel, BuhlmannModel};
pub use compartment::{Compartment, Supersaturation};

// Add aliases with alternative spelling
pub type BuehlmannConfig = BuhlmannConfig;
