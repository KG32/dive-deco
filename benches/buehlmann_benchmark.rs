use criterion::{ Criterion, criterion_group, criterion_main };
use dive_deco::{ BuehlmannModel, DecoModel, Gas };

pub fn buehlmann_ndl_benchmark(c: &mut Criterion) {
    c.bench_function("Buehlmann NDL", |b| b.iter(|| {
        let mut model = BuehlmannModel::default();
        model.record(20., 5, &Gas::air());
        model.ndl();
    }));
}

pub fn buehlmann_deco_benchmark(c: &mut Criterion) {
    let mut model = BuehlmannModel::default();
    let air = Gas::new(0.21, 0.);
    let ean_50 = Gas::new(0.50, 0.);
    model.record(40.0001, 20 * 60, &air);
    c.bench_function("Buehlmann deco", |b| b.iter(|| {
        model.deco(vec![air, ean_50])
    }));
}

criterion_group!(
    benches,
    buehlmann_ndl_benchmark,
    buehlmann_deco_benchmark
);
criterion_main!(benches);
