# dive-deco

A dive decompression models library.

### Buehlmann ZH-L16C

The Bühlmann decompression set of parameters is an Haldanian mathematical model (algorithm) of the way in which inert gases enter and leave the human body as the ambient pressure changes. Versions are used to create Bühlmann decompression tables and in personal dive computers to compute no-decompression limits and decompression schedules for dives in real-time.[^1]

#### Features

- step-by-step decompression model (ZH-L16C params version) calculations using depth, time and used gas
- current decompression ceiling
- current gradient factors
  - current gradient factor (the raw percentage of the Bühlmann allowable supersaturation at the current depth, i.e. super-saturation percent gradient, a.k.a GF99)
  - surface gradient factor (the surfacing gradient factor, i.e. super-saturation percentage gradient relative to the surface)

#### To-do

- gradient factors settings (currently working effectively as GF 100/100)
- NDL (no decompression limit) calculations
- helium support

#### References

- [Eric C. Baker, P.E. (1998) Understanding M-Values](http://www.dive-tech.co.uk/resources/mvalues.pdf)
- [Workman RD. Calculation of decompression schedules for nitrogen-oxygen and helium-oxygen dives.](https://apps.dtic.mil/sti/pdfs/AD0620879.pdf)

[^1]: https://en.wikipedia.org/wiki/B%C3%BChlmann_decompression_algorithm
