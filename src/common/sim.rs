#[derive(Clone, Debug, PartialEq)]
pub enum SimType {
    Calc,
    Recovery
}

pub trait Sim {
    fn fork(&self, sim_type: Option<SimType>) -> Self;
    fn is_sim(&self) -> bool;
}
