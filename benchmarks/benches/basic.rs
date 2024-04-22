use criterion::criterion_main;

extern crate structdiff_benchmarks;

criterion_main!(
    structdiff_benchmarks::basic::apply::benches,
    structdiff_benchmarks::basic::generate::benches,
    structdiff_benchmarks::basic::mutate::benches,
    structdiff_benchmarks::basic::full::benches,
);
