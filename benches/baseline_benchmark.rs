// Baseline Benchmark Suite for Performance Comparison
// Created: November 15, 2025
// Purpose: Establish baseline metrics before optimization implementation
//
// Run with: cargo bench --bench baseline_benchmark
// Save baseline: cargo bench --bench baseline_benchmark -- --save-baseline main
// Compare: cargo bench --bench baseline_benchmark -- --baseline main

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dive_deco::{BuhlmannConfig, BuhlmannModel, CeilingType, DecoModel, Depth, Gas, Sim, Time};

/// Benchmark NDL calculation at various depths
/// Tests the linear search algorithm for No Decompression Limit
pub fn ndl_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("NDL Calculations");

    let depths = vec![10.0, 20.0, 30.0, 40.0];
    let air = Gas::air();

    for depth in depths {
        group.bench_with_input(
            BenchmarkId::new("NDL at depth", format!("{}m", depth)),
            &depth,
            |b, &d| {
                b.iter(|| {
                    let mut model = BuhlmannModel::default();
                    model.record(Depth::from_meters(d), Time::from_seconds(5.), &air);
                    black_box(model.ndl());
                })
            },
        );
    }

    group.finish();
}

/// Benchmark simple deco calculations (shallow, minimal deco)
/// Tests basic decompression schedule generation
pub fn simple_deco(c: &mut Criterion) {
    let mut group = c.benchmark_group("Simple Deco");

    let air = Gas::air();
    let ean50 = Gas::new(0.50, 0.);

    group.bench_function("40m/20min with EAN50", |b| {
        let mut model = BuhlmannModel::default();
        model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
        b.iter(|| black_box(model.deco(vec![air, ean50]).unwrap()));
    });

    group.finish();
}

/// Benchmark complex deco calculations (deep, long bottom time)
/// Tests worst-case performance with multiple deco stops
pub fn complex_deco(c: &mut Criterion) {
    let mut group = c.benchmark_group("Complex Deco");

    let trimix = Gas::new(0.18, 0.45);
    let ean50 = Gas::new(0.50, 0.);
    let oxygen = Gas::new(1.0, 0.);

    group.bench_function("70m/25min trimix multi-gas", |b| {
        let mut model = BuhlmannModel::default();
        model.record(Depth::from_meters(70.), Time::from_minutes(25.), &trimix);
        b.iter(|| black_box(model.deco(vec![trimix, ean50, oxygen]).unwrap()));
    });

    group.bench_function("60m/30min air with deco gases", |b| {
        let mut model = BuhlmannModel::default();
        let air = Gas::air();
        model.record(Depth::from_meters(60.), Time::from_minutes(30.), &air);
        b.iter(|| black_box(model.deco(vec![air, ean50, oxygen]).unwrap()));
    });

    group.finish();
}

/// Benchmark ceiling calculations (both Actual and Adaptive)
/// Tests leading compartment search and ceiling determination
pub fn ceiling_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Ceiling Calculations");

    let air = Gas::air();

    // Test with Actual ceiling (simple)
    group.bench_function("Actual ceiling - in deco", |b| {
        let config = BuhlmannConfig::default()
            .with_ceiling_type(CeilingType::Actual);
        let mut model = BuhlmannModel::new(config);
        model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
        b.iter(|| black_box(model.ceiling()));
    });

    // Test with Adaptive ceiling (expensive)
    group.bench_function("Adaptive ceiling - in deco", |b| {
        let config = BuhlmannConfig::default()
            .with_ceiling_type(CeilingType::Adaptive);
        let mut model = BuhlmannModel::new(config);
        model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
        b.iter(|| black_box(model.ceiling()));
    });

    group.finish();
}

/// Benchmark travel/ascent calculations
/// Tests 1-second interval recording during depth changes
pub fn travel_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Travel Calculations");

    let air = Gas::air();

    group.bench_function("Ascent 40m to surface @ 10m/min", |b| {
        b.iter(|| {
            let mut model = BuhlmannModel::default();
            model.record(Depth::from_meters(40.), Time::from_minutes(10.), &air);
            model.record_travel_with_rate(black_box(Depth::from_meters(0.)), 10., &air);
        });
    });

    group.bench_function("Ascent 60m to 20m @ 10m/min", |b| {
        b.iter(|| {
            let mut model = BuhlmannModel::default();
            model.record(Depth::from_meters(60.), Time::from_minutes(15.), &air);
            model.record_travel_with_rate(black_box(Depth::from_meters(20.)), 10., &air);
        });
    });

    group.finish();
}

/// Benchmark tissue recalculation with different GF configurations
/// Tests compartment saturation calculations
pub fn tissue_recalculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Tissue Recalculation");

    let air = Gas::air();

    // Test with all tissues recalculated (default, expensive)
    group.bench_function("All tissues recalc (GF 30/70)", |b| {
        let config = BuhlmannConfig::default()
            .with_gradient_factors(30, 70)
            .with_all_m_values_recalculated(true);
        let mut model = BuhlmannModel::new(config);
        model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
        b.iter(|| {
            model.record(Depth::from_meters(40.), Time::from_seconds(1.), &air);
        });
    });

    // Test with only leading tissue recalculated (cheaper)
    group.bench_function("Leading tissue only (GF 30/70)", |b| {
        let config = BuhlmannConfig::default()
            .with_gradient_factors(30, 70)
            .with_all_m_values_recalculated(false);
        let mut model = BuhlmannModel::new(config);
        model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
        b.iter(|| {
            model.record(Depth::from_meters(40.), Time::from_seconds(1.), &air);
        });
    });

    // Test without GF slope (no recalculation needed)
    group.bench_function("No GF slope (GF 100/100)", |b| {
        let config = BuhlmannConfig::default()
            .with_gradient_factors(100, 100);
        let mut model = BuhlmannModel::new(config);
        model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
        b.iter(|| {
            model.record(Depth::from_meters(40.), Time::from_seconds(1.), &air);
        });
    });

    group.finish();
}

/// Benchmark model forking/cloning
/// Tests the overhead of creating simulation models
pub fn model_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("Model Cloning");

    let air = Gas::air();
    let mut model = BuhlmannModel::default();
    model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);

    group.bench_function("Fork model (16 compartments)", |b| {
        b.iter(|| {
            let _sim = black_box(model.fork());
        });
    });

    group.bench_function("Clone model (16 compartments)", |b| {
        b.iter(|| {
            let _cloned = black_box(model.clone());
        });
    });

    group.finish();
}

/// Benchmark supersaturation calculations
/// Tests iteration over all compartments for GF values
pub fn supersaturation_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Supersaturation");

    let air = Gas::air();

    group.bench_function("Supersaturation at depth", |b| {
        let mut model = BuhlmannModel::default();
        model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
        b.iter(|| black_box(model.supersaturation()));
    });

    group.finish();
}

/// Benchmark realistic dive computer scenario
/// Tests repeated calculations as would happen in real-time dive computer
pub fn dive_computer_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Dive Computer Simulation");

    let air = Gas::air();
    let ean50 = Gas::new(0.50, 0.);
    let gases = vec![air, ean50];

    group.bench_function("1 minute of dive updates", |b| {
        b.iter(|| {
            let mut model = BuhlmannModel::default();
            model.record(Depth::from_meters(30.), Time::from_minutes(15.), &air);

            // Simulate 60 seconds of updates (1 per second)
            for _ in 0..60 {
                model.record(Depth::from_meters(30.), Time::from_seconds(1.), &air);
                black_box(model.ceiling());
                black_box(model.ndl());
                let _ = black_box(model.deco(gases.clone()));
            }
        });
    });

    group.finish();
}

/// Benchmark TTS@+5 calculation (nested simulation)
/// Tests the overhead of calculating future decompression time
pub fn tts_projection(c: &mut Criterion) {
    let mut group = c.benchmark_group("TTS Projection");

    let air = Gas::air();
    let ean50 = Gas::new(0.50, 0.);
    let gases = vec![air, ean50];

    group.bench_function("Deco with TTS@+5 (nested sim)", |b| {
        let mut model = BuhlmannModel::default();
        model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
        b.iter(|| {
            // This includes TTS@+5 calculation (automatic, not simulated)
            black_box(model.deco(gases.clone()).unwrap());
        });
    });

    group.finish();
}

/// Benchmark gas switching logic
/// Tests gas selection algorithm during deco
pub fn gas_switching(c: &mut Criterion) {
    let mut group = c.benchmark_group("Gas Switching");

    let air = Gas::air();
    let ean32 = Gas::new(0.32, 0.);
    let ean50 = Gas::new(0.50, 0.);
    let ean80 = Gas::new(0.80, 0.);
    let oxygen = Gas::new(1.0, 0.);

    group.bench_function("Deco with 2 gases", |b| {
        let mut model = BuhlmannModel::default();
        model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
        b.iter(|| black_box(model.deco(vec![air, ean50]).unwrap()));
    });

    group.bench_function("Deco with 5 gases", |b| {
        let mut model = BuhlmannModel::default();
        model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
        b.iter(|| black_box(model.deco(vec![air, ean32, ean50, ean80, oxygen]).unwrap()));
    });

    group.finish();
}

/// Full realistic dive profile benchmark
/// Combines all operations in a typical technical dive
pub fn full_dive_profile(c: &mut Criterion) {
    let mut group = c.benchmark_group("Full Dive Profile");

    let trimix = Gas::new(0.18, 0.45);
    let ean50 = Gas::new(0.50, 0.);
    let oxygen = Gas::new(1.0, 0.);
    let gases = vec![trimix, ean50, oxygen];

    group.bench_function("Complete technical dive profile", |b| {
        b.iter(|| {
            let config = BuhlmannConfig::default()
                .with_gradient_factors(30, 70)
                .with_ceiling_type(CeilingType::Adaptive)
                .with_all_m_values_recalculated(true);

            let mut model = BuhlmannModel::new(config);

            // Descent
            model.record_travel_with_rate(Depth::from_meters(60.), 20., &trimix);

            // Bottom time
            model.record(Depth::from_meters(60.), Time::from_minutes(25.), &trimix);

            // Start ascent
            model.record_travel_with_rate(Depth::from_meters(21.), 10., &trimix);

            // Gas switch to EAN50
            model.record(Depth::from_meters(21.), Time::zero(), &ean50);

            // Continue ascent with deco
            let deco = model.deco(gases.clone()).unwrap();

            // Various checks
            black_box(model.ceiling());
            black_box(model.ndl());
            black_box(model.supersaturation());
            black_box(model.cns());
            black_box(model.otu());
            black_box(deco);
        });
    });

    group.finish();
}

criterion_group!(
    baseline_benches,
    ndl_calculations,
    simple_deco,
    complex_deco,
    ceiling_calculations,
    travel_calculations,
    tissue_recalculation,
    model_cloning,
    supersaturation_calculations,
    dive_computer_simulation,
    tts_projection,
    gas_switching,
    full_dive_profile,
);

criterion_main!(baseline_benches);

