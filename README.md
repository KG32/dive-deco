# dive-deco

A dive decompression models library.

## Buehlmann ZH-L16C

The Bühlmann decompression set of parameters is an Haldanian mathematical model of the way in which inert gases enter and leave the human body as the ambient pressure changes. Versions are used to create Bühlmann decompression tables and in personal dive computers to compute no-decompression limits and decompression schedules for dives in real-time.[^1]

### Features

- step-by-step decompression model (ZH-L16C params version) calculations using depth, time and used gas (incl. helium mixes)
- GF (gradient factors) ascent profile conservatism
- current deco runtime / deco stop planner
  - decompression stages as a runtime based on current model state
  - TTS (time to surface)
- NDL (no-decompression limit)
- decompression ceiling
- supersaturation
  - GF99 (the raw percentage of the Bühlmann supersaturation at the current depth, i.e. super-saturation percent gradient)
  - GFsurf(the surfacing gradient factor, i.e. super-saturation percentage gradient relative to the surface)
- CNS (central nervous system toxicity)
- configurable model settings
  - gradient factors
  - surface pressure

### Planned features

- extended deco model config [metric/imperial units, water density and more] (currently metric and density assumed to be 1.03kg/l as salt water)
- travel steps optimization (linear ascent / descent steps using Schreiner equation instead of iterative Haldane equation)
- BuehlmannModel Default trait implementation
- other deco algorithms (VPM-B)
- other optimizations

### API

- [API documentation](https://docs.rs/dive-deco/latest/dive_deco/)

---

### Usage

#### Model initialization

##### Using default config

```rust
use dive_deco::{ BuehlmannConfig, BuehlmannModel, DecoModel };

fn main() {
    // model with default config (GF 100/100)
    let model = BuehlmannModel::default();
    println!("{:?}", model.config()); // BuehlmannConfig { gf: (100, 100) }
}
```

##### Using config builder

Current config options:

- `gradient_factors` - gradient factors settings (`[GFlow], [GFhigh])`default: `(100, 100)`)
- `surface_pressure` - atmospheric pressure at the surface at the time of model initialization and assumed constant throughout model's life

```rust
    // fluid-interface-like built config
    let config = BuehlmannConfig::new()
        .gradient_factors(30, 70)
        .surface_pressure(1013);
    let model = BuehlmannModel::new(config);
    println!("{:?}", model.config()); // BuehlmannConfig { gf: (30, 70) }
```

---

#### Updating model state

##### Step

A DecoModel trait method that represents a single model step as a datapoint.

- `.step(depth, time, gas)`
  - depth - current depth in msw
  - time - duration in seconds
  - gas - breathing mix used for the duration of this step

```rust
let depth = 20.;
let time = 1; // 1 second
let nitrox = Gas::new(0.32, 0.);
// register 1 second at 20 msw breathing nitrox 32
model.step(depth, time, &nitrox);
```

##### Step travel

A DecoModel trait method that represents a linear change of depth. It assumes a travel from depth A (current model state depth) to B (target_depth) with rate derived from change of depth and time.

- `.step_travel(target_depth, time, gas)`
  - target_depth - final depth at the end of the travel
  - time - duration of travel in seconds
  - gas: breathing mix using for the duration of this step

```rust
let target_depth = 30.;
let descent_time = 4 * 60; // 4 minutes as seconds
let nitrox = Gas::new(0.32, 0.);
// register a 4 minute descent to 30m using nitrox 32
model.step_travel(target_depth, time, &nitrox);
```

---

#### Decompression data / model state

##### Decompression stages (current deco runtime) + TTS

All decompression stages calculated to clear deco obligations and resurface in a most efficient way - a partial deco runtime from current model state to resurfacing.

- `deco() -> Vec<DecoStage>`

###### DecoStage

- stage_type (enum)
  - ```Ascent``` - linear ascent to shallowest depth possible, defined by deco stop depth (ceiling rounded using default 3m deco stop window) or surface if no deco obligation
  - ```DecoStop``` - a mandatory deco stop needed to desaturate enough to proceed to the next one
  - ```GasSwitch``` - a switch to another (most efficient) deco gas considering MOD and o2 content. Gas switch to another gas considered only if currently in decompression
- start_depth - depth at which deco stage started
- end_depth - depth at which deco stage ended
- duration - duration of deco stage in seconds
- tts - TTS (time to surface) is the the fastest a diver can resurface while completing all deco obligations and maintaining max ascent speed. TTS is based on deco stages total duration.

```rust
    let config = BuehlmannConfig::new().gradient_factors(30, 70);
    let mut model = BuehlmannModel::new(config);

    // bottom gas
    let air = Gas::air();
    // deco gases
    let ean_50 = Gas::new(0.5, 0.);
    let oxygen = Gas::new(1., 0.);
    let available_gas_mixes = vec![
        air,
        ean_50,
        oxygen,
    ];

    let bottom_depth = 40.;
    let bottom_time = 20 * 60; // 20 min

    // descent to 40m at a rate of 9min/min using air
    model.step_travel_with_rate(bottom_depth, 9., &available_gas_mixes[0]);

    // 20 min bottom time
    model.step(bottom_depth, bottom_time, &air);

    // calculate deco runtime providing available gasses
    let deco = model.deco(available_gas_mixes);
```

:warning: Current deco stops implementation consideres gas switches based on MOD only - don't use with hypoxic trimix mixes

##### NDL (no-decompression limit)

The NDL is a theoretical time obtained by calculating inert gas uptake and release in the body that determines a time interval a diver may theoretically spend at given depth without aquiring any decompression obligations (given constant depth and gas mix).

- `ndl()` - no-decompression limit for current model state in minutes, assuming constant depth and gas mix

```rust
use dive_deco::{DecoModel, BuehlmannModel, BuehlmannConfig, Gas};

fn main() {
    // initialize a Buehlmann ZHL-16C deco model with default config (GF 100/100)
    let config = BuehlmannConfig::default();
    let mut model = BuehlmannModel::new(config);

    let air = Gas::new(0.21, 0.);
    let depth = 30.;
    let bottom_time_minutes = 10;

    // a simulated instantaneous drop to 20m with a single step simulating 20 minutes bottom time using air
    model.step(depth, bottom_time_minutes * 60, &air);
    // model.step(....)
    // model.step(....)

    // current NDL (no-decompression limit)
    let current_ndl = model.ndl();
    println!("NDL: {} min", current_ndl); // output: NDL: 5 min
}
```

##### Decompression Ceiling

Minimum theoretical depth that can be reached at the moment without breaking the decompression obligation. In case of Buehlmann algorithm, a depth restricted by M-value given leading tissue saturation and gradient factors setting.

- `ceiling()` - current decompression ceiling in msw, given current model state and gradient factors settings

```rust
use dive_deco::{ BuehlmannConfig, BuehlmannModel, DecoModel, Gas };

fn main() {
    let mut model = BuehlmannModel::new(BuehlmannConfig::default());

    let nitrox_32 = Gas::new(0.32, 0.);

    // ceiling after 20 min at 20 meters using EAN32 - ceiling at 0m
    model.step(20., 20 * 60, &nitrox_32);
    println!("Ceiling: {}m", model.ceiling()); // Ceiling: 0m

    // ceiling after another 42 min at 30 meters using EAN32 - ceiling at 3m
    model.step(30., 42 * 60, &nitrox_32);
    println!("Ceiling: {},", model.ceiling()); // Ceiling: 3.004(..)m
}
```

##### Current tissues oversaturation (gradient factors)

Current tissue oversaturation as gradient factors.

- `supersaturation() -> Supersaturation { gf_99, gf_surf }` - supersaturation in % relative to M-value ()
  - gf_99 (f64) - GF99, current oversaturation relative to ambient pressure
  - gf_surf (f64) - Surface GF, current oversaturation relative to surface pressure

```rust
  // given model state after 120 seconds at 40 msw breathing air
  // (...)

  // on-gassing, gf99: 0%, surfGF: 71%
  let supersaturation = model.supersaturation(); // Supersaturation { gf_99: 0.0, gf_surf: 71.09852831834125 }
```

##### CNS (Central Nervous System Toxicity)

Current Central Nervous System Toxicity percentage (derived from NOAA limits).
Measure (%) of accumulated exposure to elevated oxygen partial pressure in relation to maximum allowed exposure time for given ranges.

- `cns()` - CNS %

```rust
  // given model
  // (...)
  let cns = model.cns(); // 32.5
```

---

#### Common

##### Gas

Breathing gas used in the model.

- `new(o2, he)`
  - o2 - oxygen partial pressure
  - he - helium partial pressure
- `partial_pressures(depth)` - compounded gas's components partial pressures at certain depth
- `inspired_partial_pressures(depth)` - inspired gas partial pressures in alveoli taking into account alveolar water vapor pressure
- `maximum_operating_depth(pp_o2_limit)` - maximum operating depth considering o2 partial, with maximum o2 partial pressure as parameter
- `equivalent_narcotic_depth(depth)` - equivalent depth at which given gas has the same narcotic potential as air. Assumes o2 - n2 1:1 narcotic ratio.

```rust
let mix = Gas::new(0.21, 0.);
mix.partial_pressures(10.); // PartialPressures { o2: 0.42, n2: 1.58, he: 0.0 }
mix.inspired_partial_pressures(10.); // PartialPressures { o2: 0.406833, n2: 1.530467, he: 0.0 }
```

---

### References

- [Eric C. Baker, P.E. Dissolved Gas Decompression Modeling](https://www.shearwater.com/wp-content/uploads/2012/08/Introductory-Deco-Lessons.pdf)
- [Eric C. Baker, P.E. (1998) Understanding M-Values](http://www.dive-tech.co.uk/resources/mvalues.pdf)
- [Eric C Baker, P.E., Oxygen Toxicity Calculations](https://njscuba.net/wp-content/uploads/gear/pdf/deco_oxy_tox_calcs.pdf)
- [Workman RD. Calculation of decompression schedules for nitrogen-oxygen and helium-oxygen dives.](https://apps.dtic.mil/sti/pdfs/AD0620879.pdf)
- [Ralph Lembcke and Matthias Heinrichs (2020), Decompression calculations in the OSTC](https://www.heinrichsweikamp.net/downloads/OSTC_GF_web_en.pdf)
- ["Tauchmedizin.", Albert A. Bühlmann, Ernst B. Völlm (Mitarbeiter), P. Nussberger; 5. edition in 2002, Springer, ISBN 3-540-42979-4](https://books.google.com/books?id=MYAGBgAAQBAJ&printsec=copyright&redir_esc=y#v=onepage&q&f=false)
- [Salm, Albi & Eisenstein, Yael & Vered, Nurit & Rosenblat, Miri. (2022). On the arbitrariness of the ZH-L Helium coefficients (16.08.2022). 10.13140/RG.2.2.19048.55040.](https://www.researchgate.net/publication/362716934_On_the_arbitrariness_of_the_ZH-L_Helium_coefficients_16082022)
- [Rosenblat, Miri & Salm, Albi. (2024). Introduction to Decompression Calculation.](https://www.researchgate.net/publication/362716934_On_the_arbitrariness_of_the_ZH-L_Helium_coefficients_16082022)

> :warning: Disclaimer: Not Suitable for Dive Planning,  Work-in-Progress Model
> This decompression model is currently in a developmental stage and should be treated as a work in progress. Users are advised that the information generated by this model may not be accurate and could contain errors. It is important to exercise caution and verify any critical information provided by the model through alternative sources.
> This model is not designed or intended to be used as a dive planning software. Diving involves inherent risks, and accurate planning is crucial for safety. Users are strongly advised to rely on specialized dive planning software and consult with certified dive professionals for accurate and reliable information related to diving activities.
> By using this model, users acknowledge that it is not a substitute for professional advice or dedicated tools designed for specific tasks, and the developers take no responsibility for any consequences arising from the use of information generated by this model.

[^1]: <https://en.wikipedia.org/wiki/B%C3%BChlmann_decompression_algorithm>
