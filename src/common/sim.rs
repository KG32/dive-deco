pub trait Sim {
    fn fork(&self) -> Self;
    fn is_sim(&self) -> bool;
}
