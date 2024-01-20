use crate::compartment::Compartment;
use crate::step::Step;
use crate::global_types::Depth;
use crate::zhl_16_values::ZHLParams;

pub struct ZHLModel {
    compartments: Vec<Compartment>,
}

impl ZHLModel {
    pub fn new(zhl_values: Vec<ZHLParams>) -> ZHLModel {
        let mut model = ZHLModel {
            compartments: vec![]
        };
        model.create_compartments(zhl_values);
        model
    }

    pub fn step(&mut self, step: &Step) -> () {
        self.recalculate_compartments(step);
    }

    pub fn ceiling(&self) -> Depth {
        let mut leading_cpt: &Compartment = &self.compartments[0];
        for compartment in &self.compartments {
            if compartment.min_tolerable_pressure > leading_cpt.min_tolerable_pressure {
                leading_cpt = compartment;
            }
        }
        let mut ceil = (leading_cpt.min_tolerable_pressure - 1.) * 10.;
        if ceil < 0. {
            ceil = 0.;
        }
        ceil
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

    fn recalculate_compartments(&mut self, step: &Step) -> () {
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate(step);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::gas::Gas;
    use crate::zhl_16_values::*;

    #[test]
    fn test_ceiling() {
        let mut model = ZHLModel::new(zhl_16_values().to_vec());
        let air = Gas::new(0.21);
        let step1 = Step { depth: &40., time: &30., gas: &air };
        let step2 = Step { depth: &30., time: &30., gas: &air };
        model.step(&step1);
        model.step(&step2);
        let calculated_ceiling = model.ceiling();
        assert_eq!(calculated_ceiling, 8.207313);
    }
}
