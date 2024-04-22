use criterion::criterion_main;

extern crate structdiff_benchmarks;

criterion_main!(
    structdiff_benchmarks::large::apply::benches,
    structdiff_benchmarks::large::generate::benches,
    structdiff_benchmarks::large::mutate::benches,
    structdiff_benchmarks::large::full::benches,
);
