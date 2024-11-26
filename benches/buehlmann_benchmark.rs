use criterion::{criterion_group, criterion_main, Criterion};
use dive_deco::{BuehlmannConfig, BuehlmannModel, DecoModel, Depth, Gas, Unit};

pub fn buehlmann_ndl_benchmark(c: &mut Criterion) {
    c.bench_function("Buehlmann NDL", |b| {
        b.iter(|| {
            let mut model = BuehlmannModel::default();
            model.record(Depth::from_meters(20.), 5, &Gas::air());
            model.ndl();
        })
    });
}

pub fn buehlmann_deco_benchmark(c: &mut Criterion) {
    let mut model = BuehlmannModel::default();
    let air = Gas::new(0.21, 0.);
    let ean_50 = Gas::new(0.50, 0.);
    model.record(Depth::from_meters(40.0001), 20 * 60, &air);
    c.bench_function("Buehlmann deco", |b| {
        b.iter(|| model.deco(vec![air, ean_50]))
    });
}

pub fn buehlmann_deco_adaptive_recalc(c: &mut Criterion) {
    let config = BuehlmannConfig::default()
        .with_gradient_factors(30, 70)
        .with_ceiling_type(dive_deco::CeilingType::Adaptive);

    let mut model = BuehlmannModel::new(config);

    let air = Gas::air();
    let ean50 = Gas::new(0.50, 0.);
    let available_gasses = vec![air, ean50];

    c.bench_function("Record and deco", |b| {
        b.iter(|| {
            model.record(Depth::from_meters(40.), 1, &air);
            model.deco(available_gasses.clone()).unwrap();
            model.record(Depth::from_meters(40.), 1, &air);
            model.record(Depth::from_meters(40.), 1, &air);
            model.deco(available_gasses.clone()).unwrap();
        });
    });
}

pub fn buehlmann_full(c: &mut Criterion) {
    let config = BuehlmannConfig::default()
        .with_gradient_factors(30, 70)
        .with_ceiling_type(dive_deco::CeilingType::Adaptive)
        .with_all_m_values_recalculated(true);

    let mut model = BuehlmannModel::new(config);

    let air = Gas::air();
    let ean50 = Gas::new(0.50, 0.);
    let o2 = Gas::new(1., 0.);
    let available_gasses = vec![air, ean50, o2];

    c.bench_function("Buehlmann full", |b| {
        b.iter(|| {
            model.record(Depth::from_meters(40.), 20 * 60, &air);
            model.deco(available_gasses.clone()).unwrap();
            model.record(Depth::from_meters(40.), 5 * 60, &air);
            model.record_travel_with_rate(Depth::from_meters(35.), 10., &air);
            model.record_travel_with_rate(Depth::from_meters(21.), 10., &air);
            model.record(Depth::from_meters(21.), 60, &ean50);
            model.supersaturation();
            model.ceiling();
            model.deco(available_gasses.clone()).unwrap();
            model.in_deco();
            model.ndl();
            model.cns();
            model.otu();
        });
    });
}

criterion_group!(
    benches,
    buehlmann_ndl_benchmark,
    buehlmann_deco_benchmark,
    buehlmann_deco_adaptive_recalc,
    buehlmann_full,
);
criterion_main!(benches);
