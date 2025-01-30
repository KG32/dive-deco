# dive-deco

A dive decompression models library.

## Buehlmann ZH-L16C

The Bühlmann decompression set of parameters is an Haldanian mathematical model of the way in which inert gases enter and leave the human body as the ambient pressure changes. Versions are used to create Bühlmann decompression tables and in personal dive computers to compute no-decompression limits and decompression schedules for dives in real-time.[^1]

### Features

- step-by-step decompression model (ZH-L16C params version) calculations using depth, time and used gas (incl. helium mixes)
- NDL (no-decompression limit)
- GF (gradient factors) ascent profile conservatism
- current deco runtime / deco stop planner
  - decompression stages as a runtime based on current model state
  - TTS (current time to surface including ascent and all decompression stops)
  - TTS @+5 (TTS after 5 mins given constant depth and breathing mix)
  - TTS Δ+5 (absolute change in TTS after 5 mins given current depth and gas mix)
- ceiling
- supersaturation
  - GF99 (the raw percentage of the Bühlmann supersaturation at the current depth, i.e. super-saturation percent gradient)
  - GFsurf(the surfacing gradient factor, i.e. super-saturation percentage gradient relative to the surface)
- oxygen toxicity
  - CNS (central nervous system toxicity)
  - OTU (pulmonary oxygen toxicity)
- configurable model settings
  - gradient factors
  - surface pressure
  - deco ascent rate
  - NDL definition
    - Actual (default) - both NDL time and ceiling are determined by the current tissues saturation, it counts down to a condition where calculated ceiling is below the surface
    - Adaptive - takes into account off-gassing on ascent, determines if real deco obligation assuming direct ascent with set ascent rate

### Planned features

- extended deco model config [water density and other configuration options] (currently metric and density assumed to be 1.03kg/l as salt water)
- travel records optimization (linear ascent / descent records using Schreiner equation instead of iterative Haldane equation)
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
- `deco_ascent_rate` - ascent rate in m/s that is assumed to be followed when calculating deco obligations and simulations. Default value: 10 m/min (33 ft/min)
- `ceiling_type` (enum `CeilingType`)
  - `Actual` (default) - both NDL time and ceiling are determined by the current tissues saturation, it counts down to a condition where calculated ceiling is below the surface
  - `Adaptive` - takes into account off-gassing on ascent, determines if real deco obligation assuming direct ascent with set ascent rate
- `recalc_all_tissues_m_values` - recalculate all tissues considering gradient factors (default: true). If set to false, only leading tissue is recalculated with max gf

```rust
// fluid-interface-like built config
let config = BuehlmannConfig::new()
    .with_gradient_factors(30, 70)
    .with_surface_pressure(1013)
    .with_deco_ascent_rate(10.)
    .with_ceiling_type(CeilingType::Actual);
let model = BuehlmannModel::new(config);
println!("{:?}", model.config()); // BuehlmannConfig { gf: (30, 70) }
```

---

#### Common

##### Depth

A VO representing depth, both constructed from and represented as meters or feet.

- `from_meters(val: Into<f64>) -> Depth`
- `from_feet(val: Into<f64>) -> Depth`
- `as_meters() -> f64`
- `as_feet() -> f64`

```rust
let depth_1 = Depth::from_meters(10.);
println!("{}m", depth_1.as_meters()); // 10m
println!("{}ft", depth_1.as_feet()); // 32.80ft

let depth_2 = Depth::from_feet(100.);
println!("{}m", depth_2.as_meters()); // 30.48m
println!("{}ft", depth_2.as_feet()); // 100ft

let depths_sum = depth_1 + depth_2;
println!(
    "{}m + {}ft = {}m / {}ft",
    depth_1.as_meters(),
    depth_2.as_feet(),
    depths_sum.as_meters(),
    depths_sum.as_feet()
); // 10m + 100ft = 40.48m / 132.80ft
```

##### Time

A VO representing time, both constructed from and represented as seconds or minutes.

```rust
let time = Time::from_minutes(1.); // same as Time::from_seconds(60.);
println!("{}m = {}s", time.as_minutes(), time.as_seconds()); // 1m = 60s
assert_eq!(Time::from_minutes(0.5), Time::from_seconds(30.));
```

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

#### Updating model state

##### Record

A DecoModel trait method that represents a single model record as a datapoint.

- `.record(depth, time, gas)`
  - depth - current depth in meters
  - time - duration in seconds
  - gas - breathing mix used for the duration of this record

```rust
let depth = Depth::from_meters(20.);
let time = 1; // 1 second
let nitrox = Gas::new(0.32, 0.);
// register 1 second at 20m breathing nitrox 32
model.record(depth, time, &nitrox);
```

##### Record travel

A DecoModel trait method that represents a linear change of depth. It assumes a travel from depth A (current model state depth) to B (target_depth) with rate derived from change of depth and time.

- `.record_travel(target_depth, time, gas)`
  - target_depth - final depth at the end of the travel
  - time - duration of travel in seconds
  - gas: breathing mix using for the duration of this record

```rust
let target_depth = Depth::from_meters(30.);
let descent_time = 4 * 60; // 4 minutes as seconds
let nitrox = Gas::new(0.32, 0.);
// register a 4 minute descent to 30m using nitrox 32
model.record_travel(target_depth, time, &nitrox);
```

---

#### Decompression data / model state

##### Decompression stages (current deco runtime) + TTS

All decompression stages calculated to clear deco obligations and resurface in a most efficient way - a partial deco runtime from current model state to resurfacing.

```text
.deco(Vec<Gas>) -> Result<DecoRuntime, DecoCalculationError>

<!-- DecoRuntime {
  deco_stages: Vec<DecoStage>,
  tts: u64,
  tts_at_5: u64,
  tts_delta_at_5: i64
} -->
```

- `DecoRuntime`
  - `deco_stages (DecoStage)`
    - `stage_type` (enum)
      - ```Ascent``` - linear ascent to shallowest depth possible, defined by deco stop depth (ceiling rounded using default 3m deco stop window) or surface if no deco obligation
      - ```DecoStop``` - a mandatory deco stop needed to desaturate enough to proceed to the next one
      - ```GasSwitch``` - a switch to another (most efficient) deco gas considering MOD and o2 content. Gas switch to another gas considered only if currently in decompression
    - `start_depth` - depth at which deco stage started
    - `end_depth` - depth at which deco stage ended
  - `duration` - duration of deco stage in seconds
  - `tts` - current time to surface in minutes. The least amount of time possible to surface without violating decompression obligations according to the current model. Includes the duration of all necessary deco stops (assuming switching to most optimal decompression gas) and travel time between them
  - `tts_at_5` (aka @+5) - TTS in 5 minutes assuming constant depth and gas mix
  - `tts_delta_at_5` (aka Δ+5) - absolute change in TTS after 5 mins assuming constant depth and gas mix
- `DecoCalculationError`
  - `EmptyGasList` - occurs when available gasses vector is empty
  - `CurrentGasNotInList` - occurs when provided available list doesn't include gas currently in use according to deco model's state

```rust
let config = BuehlmannConfig::new().with_gradient_factors(30, 70);
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

let bottom_depth = Depth::from_meters(40.);
let bottom_time = 20 * 60; // 20 min

// descent to 40m at a rate of 9min/min using air
model.record_travel_with_rate(bottom_depth, 9., &available_gas_mixes[0]);

// 20 min bottom time
model.record(bottom_depth, bottom_time, &air);

// calculate deco runtime providing available gasses
let deco_runtime = model.deco(available_gas_mixes);

println!("{:#?}", deco_runtime);
```

<details>
  <summary>Output</summary>
    <code>
    DecoRuntime {
      deco_stages: [
          DecoStage {
              stage_type: Ascent,
              start_depth: Depth { m: 40.0 },
              end_depth: Depth { m: 22.0 },
              duration: 120,
              gas: Gas {
                  o2_pp: 0.21,
                  n2_pp: 0.79,
                  he_pp: 0.0,
              },
          },
          DecoStage {
              stage_type: GasSwitch,
              start_depth: Depth { m: 22.0 },
              end_depth: Depth { m: 22.0 },
              duration: 0,
              gas: Gas {
                  o2_pp: 0.5,
                  n2_pp: 0.5,
                  he_pp: 0.0,
              },
          },
          DecoStage {
              stage_type: Ascent,
              start_depth: Depth { m: 22.0 },
              end_depth: Depth { m: 6.000000000000001 },
              duration: 106,
              gas: Gas {
                  o2_pp: 0.5,
                  n2_pp: 0.5,
                  he_pp: 0.0,
              },
          },
          DecoStage {
              stage_type: GasSwitch,
              start_depth: Depth { m: 6.000000000000001 },
              end_depth: Depth { m: 6.000000000000001 },
              duration: 0,
              gas: Gas {
                  o2_pp: 1.0,
                  n2_pp: 0.0,
                  he_pp: 0.0,
              },
          },
          DecoStage {
              stage_type: DecoStop,
              start_depth: Depth { m: 6.000000000000001 },
              end_depth: Depth { m: 6.000000000000001 },
              duration: 410,
              gas: Gas {
                  o2_pp: 1.0,
                  n2_pp: 0.0,
                  he_pp: 0.0,
              },
          },
          DecoStage {
              stage_type: Ascent,
              start_depth: Depth { m: 6.000000000000001 },
              end_depth: Depth { m: 3.0 },
              duration: 20,
              gas: Gas {
                  o2_pp: 1.0,
                  n2_pp: 0.0,
                  he_pp: 0.0,
              },
          },
          DecoStage {
              stage_type: DecoStop,
              start_depth: Depth { m: 3.0 },
              end_depth: Depth { m: 3.0 },
              duration: 226,
              gas: Gas {
                  o2_pp: 1.0,
                  n2_pp: 0.0,
                  he_pp: 0.0,
              },
          },
          DecoStage {
              stage_type: Ascent,
              start_depth: Depth { m: 3.0 },
              end_depth: Depth { m: 0.0 },
              duration: 20,
              gas: Gas {
                  o2_pp: 1.0,
                  n2_pp: 0.0,
                  he_pp: 0.0,
              },
          },
      ],
      tts: 16,
      tts_at_5: 20,
      tts_delta_at_5: 4,
    }
    </code>
</details>

:warning: Current deco stops implementation consideres gas switches based on MOD only - don't use with hypoxic trimix mixes

##### NDL (no-decompression limit)

The NDL is a theoretical time obtained by calculating inert gas uptake and release in the body that determines a time interval a diver may theoretically spend at given depth without aquiring any decompression obligations (given constant depth and gas mix).

- `ndl()` - no-decompression limit for current model state in minutes, assuming constant depth and gas mix. This method has a cut-off at 99 minutes.
NDL controllable by `ceiling_type` model config. By default (`Actual`), NDL is determined by the current tissues saturation, it counts down to a condition where ceiling isn't equal to the surface. The other ceiling type config (`Adaptive`) takes into account off-gassing during ascent and it's defined as a maximum time at given depth that won't create any decompression obligations (i.e. even on existing ceiling, limit occures when a direct ascent with configured ascent rate doesn't cause any tissue to intersect with its M-Value at a given time).

```rust
use dive_deco::{DecoModel, BuehlmannModel, BuehlmannConfig, Gas};

fn main() {
    // initialize a Buehlmann ZHL-16C deco model with default config (GF 100/100)
    let config = BuehlmannConfig::default();
    let mut model = BuehlmannModel::new(config);

    let air = Gas::new(0.21, 0.);
    let depth = Depth::from_meters(30.);
    let bottom_time = Time::from_minutes(10.);

    // a simulated instantaneous drop to 20m with a single record simulating 20 minutes bottom time using air
    model.record(depth, bottom_time, &air);
    // model.record(....)
    // model.record(....)

    // current NDL (no-decompression limit)
    let current_ndl = model.ndl();
    println!("NDL: {} min", current_ndl); // output: NDL: 5 min
    // if we used an `Adaptive` ceiling_type config that takes into account off-gassing on ascent, the output would be 9 min
}
```

##### Decompression Ceiling

Minimum theoretical depth that can be reached at the moment without breaking the decompression obligation. In case of Buehlmann algorithm, a depth restricted by M-value given leading tissue saturation and gradient factors setting.

- `ceiling()` - current decompression ceiling in meters, given current model state and gradient factors settings

```rust
use dive_deco::{ BuehlmannConfig, BuehlmannModel, DecoModel, Gas };

fn main() {
let mut model = BuehlmannModel::new(BuehlmannConfig::default());

let nitrox_32 = Gas::new(0.32, 0.);

// ceiling after 20 min at 20 meters using EAN32 - ceiling at 0m
model.record(Depth::from_meters(20.), 20 * 60, &nitrox_32);
println!("Ceiling: {}m", model.ceiling()); // Ceiling: 0m

// ceiling after another 42 min at 30 meters using EAN32 - ceiling at 3m
model.record(Depth::from_meters(30.), 42 * 60, &nitrox_32);
println!("Ceiling: {},", model.ceiling()); // Ceiling: 3.004(..)m
}
```

##### Current tissues oversaturation (gradient factors)

Current tissue oversaturation as gradient factors.

- `supersaturation() -> Supersaturation { gf_99, gf_surf }` - supersaturation in % relative to M-value ()
  - gf_99 (f64) - GF99, current oversaturation relative to ambient pressure
  - gf_surf (f64) - Surface GF, current oversaturation relative to surface pressure

```rust
// given model state after 120 seconds at 40 meters breathing air
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

##### OTU (Oxygen Toxicity Units) / UPTD (unit pulmonary toxic dose)

Pulmonary oxygen toxicity which concerns the effects to the lungs of long-term exposures
to oxygen at elevated partial pressures presented as units (1 OTU = 100% O2 @ 1bar equivalent).

- `otu()` - OTU

```rust
// given model
// (...)
let cns = model.otu(); // 78.43
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
