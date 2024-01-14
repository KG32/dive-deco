use crate::compartment::Compartment;
use crate::step::Step;
use crate::global_types::Depth;
use crate::zhl_16_values::{ZHLParams, zhl_16_values};

pub struct ZHLModel {
    compartments: Vec<Compartment>,
}

impl ZHLModel {
    pub fn new(zhl_values: Vec<ZHLParams>) -> ZHLModel {

        ZHLModel {
            compartments: vec![]
        }
    }

    pub fn step(&self, step: Step) -> () {

    }

    pub fn ceiling(&self) -> Depth {
        0.
    }

    fn create_compartments(&mut self, zhl_values: Vec<ZHLParams>) -> () {
        let mut compartments: Vec<Compartment> = vec![];
        let mut i = 0;
        for comp_values in zhl_values {
            let compartment = Compartment::new(i, comp_values);
            compartments.push(compartment);
            i += 1;
        }
        self.compartments = compartments;
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::gas::Gas;

    #[test]
    fn test_ceiling() {
        let model = ZHLModel::new(zhl_16_values().to_vec());
        let air = Gas::new(0.21);
        model.step(Step { depth: 40., time: 30., gas: air });
        // model.step(Step { depth: 30., time: 30., gas: air });
        let calculated_ceiling = model.ceiling();
    }
}
