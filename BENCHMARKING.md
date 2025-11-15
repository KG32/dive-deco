# Benchmarking Guide

This guide explains how to use the benchmark suite to measure and compare performance improvements in dive-deco.

## Quick Start

### Run All Benchmarks
```bash
cargo bench
```

### Run Specific Benchmark Suite
```bash
# Original benchmark suite
cargo bench --bench buhlmann_benchmark

# Baseline benchmark suite (comprehensive)
cargo bench --bench baseline_benchmark
```

## Baseline Benchmark Suite

The `baseline_benchmark.rs` suite provides comprehensive performance metrics across all critical operations:

### Benchmark Categories

1. **NDL Calculations** - Tests No Decompression Limit calculation at various depths (10m, 20m, 30m, 40m)
2. **Simple Deco** - Basic decompression schedules with minimal deco obligation
3. **Complex Deco** - Deep technical dives with extended bottom time and multiple gas switches
4. **Ceiling Calculations** - Both Actual and Adaptive ceiling calculations
5. **Travel Calculations** - Depth change simulations with 1-second intervals
6. **Tissue Recalculation** - Compartment saturation with different GF configurations
7. **Model Cloning** - Fork and clone operations overhead
8. **Supersaturation** - GF99 and surface GF calculations
9. **Dive Computer Simulation** - Real-time dive computer scenario (60 updates)
10. **TTS Projection** - Time-To-Surface with +5min projection
11. **Gas Switching** - Gas selection algorithm with 2-5 gases
12. **Full Dive Profile** - Complete technical dive simulation

## Comparing Performance Before/After Changes

### Step 1: Establish Baseline on Main Branch

Before making any optimization changes, run and save the baseline:

```bash
# Make sure you're on main branch with no changes
git checkout main
git pull

# Run benchmarks (this will take several minutes)
cargo bench --bench baseline_benchmark

# Results are automatically saved in target/criterion/
```

### Step 2: Create Your Feature Branch

```bash
git checkout -b optimization/feature-name
```

### Step 3: Implement Your Changes

Make your performance improvements...

### Step 4: Run Benchmarks and Compare

```bash
# Run benchmarks on your feature branch
cargo bench --bench baseline_benchmark

# Criterion will automatically compare with previous runs
# and show percentage changes in the output
```

### Step 5: Review Results

Criterion generates detailed HTML reports in `target/criterion/`:

```bash
# Open the report in your browser
open target/criterion/report/index.html
```

## Understanding Benchmark Output

### Time Format
- `ns` = nanoseconds (10⁻⁹ seconds)
- `µs` = microseconds (10⁻⁶ seconds)
- `ms` = milliseconds (10⁻³ seconds)
- `s` = seconds

### Example Output
```
NDL Calculations/NDL at depth/20m
                        time:   [9.2909 s 9.3099 s 9.3305 s]
                        change: [-15.234% -14.892% -14.521%] (p = 0.00 < 0.05)
                        Performance has improved.
```

This means:
- Current time: ~9.3 seconds
- Compared to baseline: ~15% faster
- p-value < 0.05: statistically significant

## Benchmark Targets by Optimization

### If optimizing NDL calculation:
```bash
cargo bench --bench baseline_benchmark ndl
```

### If optimizing deco calculation:
```bash
cargo bench --bench baseline_benchmark deco
```

### If optimizing ceiling calculation:
```bash
cargo bench --bench baseline_benchmark ceiling
```

### If optimizing tissue recalculation:
```bash
cargo bench --bench baseline_benchmark tissue
```

### If optimizing travel/ascent:
```bash
cargo bench --bench baseline_benchmark travel
```

## Expected Performance Targets

Based on the optimization analysis, here are the expected improvements for each phase:

### Phase 1: Critical Path Optimizations
- **NDL Calculations**: 5-7× faster (binary search vs linear)
- **Simple Deco**: 3-5× faster (Schreiner equation)
- **Complex Deco**: 10-20× faster (combined improvements)
- **Travel Calculations**: 5-10× faster (Schreiner + larger intervals)

### Phase 2: Configuration Optimizations
- **Tissue Recalculation**: 1.5-2× faster (leading tissue only)
- **Gas Switching**: 1.2-1.5× faster (pre-sorted gases)
- **Model Cloning**: 2-3× faster (COW pattern)

### Phase 3: Advanced Optimizations
- **Ceiling Calculations**: 2-5× faster (caching, analytical solutions)
- **Supersaturation**: 1.5-2× faster (lazy evaluation, caching)

## Current Baseline Performance (Main Branch - Nov 15, 2025)

Initial baseline measurements:

| Benchmark | Time | Notes |
|-----------|------|-------|
| NDL @ 20m | ~9.3s | Linear search, 1-min intervals |
| Simple Deco (40m/20min) | ~313s | 1-second deco stop intervals |
| Complex Deco (70m/25min) | ~1.5ms | Deep technical dive |
| Adaptive Ceiling | ~33.2s | Recursive simulation |
| Ascent 40m-0m | ~41.4s | 1-second travel intervals |
| Tissue Recalc (all) | TBD | 16 compartments with GF slope |
| Model Fork | TBD | 16 compartments clone |

## Tips for Accurate Benchmarking

1. **Close other applications** - Reduce system load
2. **Disable CPU frequency scaling** - For consistent results
3. **Run multiple times** - Criterion does this automatically (100 samples)
4. **Warm up your system** - Let CPU reach normal operating temperature
5. **Use release builds** - Benchmarks always run in release mode
6. **Check for regressions** - Not all changes improve performance!

## Advanced: Custom Benchmark Profiles

You can modify benchmark behavior in `Cargo.toml`:

```toml
[profile.bench]
opt-level = 3
lto = "fat"         # Link-time optimization
codegen-units = 1   # Single codegen unit for max optimization
```

## Troubleshooting

### Benchmark takes too long
```bash
# Reduce sample count (default is 100)
cargo bench -- --sample-size 20
```

### Need faster iteration during development
```bash
# Quick bench with fewer samples and no plotting
cargo bench -- --quick
```

### Benchmark fails to compile
```bash
# Clean and rebuild
cargo clean
cargo bench --bench baseline_benchmark
```

## CI/CD Integration

For automated performance regression detection:

```bash
# In your CI pipeline
cargo bench --bench baseline_benchmark -- --save-baseline ci-baseline

# On subsequent runs
cargo bench --bench baseline_benchmark -- --baseline ci-baseline
```

## Profiling

For detailed profiling beyond benchmarking:

### Using cargo-flamegraph
```bash
cargo install flamegraph
cargo flamegraph --bench baseline_benchmark
```

### Using perf (Linux)
```bash
cargo bench --bench baseline_benchmark --profile-time 30
perf record -F 99 -g target/release/deps/baseline_benchmark-*
perf report
```

### Using Instruments (macOS)
```bash
# Build benchmark
cargo bench --bench baseline_benchmark --no-run

# Find the binary
find target/release/deps -name "baseline_benchmark-*" -type f

# Profile with Instruments
instruments -t "Time Profiler" <binary-path>
```

## Contributing

When submitting performance improvements:

1. Run baseline benchmarks before changes
2. Implement optimization
3. Run benchmarks again
4. Include before/after results in PR description
5. Verify correctness with existing tests
6. Document any trade-offs

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Optimization Analysis](./docs/optimization-analysis.md)

---

**Last Updated:** November 15, 2025

