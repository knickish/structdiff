use std::time::Duration;

use criterion::{black_box, criterion_group, BatchSize, Criterion};
use nanorand::{Rng, WyRand};
use structdiff::collections::rope::Rope;

criterion::criterion_main!(benches);

criterion_group!(benches, rope, vec);

const GROUP_NAME: &str = "rope";
const SAMPLE_SIZE: usize = 1000;
const MEASUREMENT_TIME: Duration = Duration::from_secs(5);

fn rand_string(rng: &mut WyRand) -> String {
    let base = vec![(); rng.generate_range::<u8, _>(5..15) as usize];
    base.into_iter()
        .map(|_| rng.generate::<u8>() as u32)
        .filter_map(char::from_u32)
        .collect::<String>()
}

trait Random {
    fn generate_random(rng: &mut WyRand) -> Self;
    fn generate_random_large(rng: &mut WyRand) -> Self;
    fn random_mutate(self, rng: &mut WyRand) -> Self;
    fn random_mutate_large(self, rng: &mut WyRand) -> Self;
}

impl Random for Rope<String> {
    fn generate_random(rng: &mut WyRand) -> Self {
        (0..rng.generate_range::<u8, _>(5..15))
            .map(|_| rand_string(rng))
            .into_iter()
            .collect()
    }

    fn generate_random_large(rng: &mut WyRand) -> Self {
        (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
            .map(|_| rand_string(rng))
            .into_iter()
            .collect()
    }

    fn random_mutate(mut self, rng: &mut WyRand) -> Self {
        match rng.generate_range(0..4) {
            0 => self.insert(rng.generate_range(0..self.len()), rand_string(rng)),
            1 => self.remove(rng.generate_range(0..self.len())),
            2 => {
                if self.len() == 0 {
                    return self;
                }
                let l = rng.generate_range(0..self.len());
                let r = rng.generate_range(0..self.len());
                self.swap(l, r)
            }
            3 => {
                let l = rng.generate_range(0..self.len());
                let r = rng.generate_range(l..self.len());
                self.drain(l..=r);
            }
            _ => (),
        }
        self
    }

    fn random_mutate_large(self, rng: &mut WyRand) -> Self {
        self.random_mutate(rng)
    }
}

impl Random for Vec<String> {
    fn generate_random(rng: &mut WyRand) -> Self {
        (0..rng.generate_range::<u8, _>(5..15))
            .map(|_| rand_string(rng))
            .into_iter()
            .collect()
    }

    fn generate_random_large(rng: &mut WyRand) -> Self {
        (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
            .map(|_| rand_string(rng))
            .into_iter()
            .collect()
    }

    fn random_mutate(mut self, rng: &mut WyRand) -> Self {
        match rng.generate_range(0..4) {
            0 => self.insert(rng.generate_range(0..self.len()), rand_string(rng)),
            1 => {
                self.remove(rng.generate_range(0..self.len()));
            }
            2 => {
                if self.len() == 0 {
                    return self;
                }
                let l = rng.generate_range(0..self.len());
                let r = rng.generate_range(0..self.len());
                self.swap(l, r)
            }
            3 => {
                let l = rng.generate_range(0..self.len());
                let r = rng.generate_range(l..self.len());
                self.drain(l..=r);
            }
            _ => (),
        };
        self
    }

    fn random_mutate_large(self, rng: &mut WyRand) -> Self {
        self.random_mutate(rng)
    }
}

fn rope(c: &mut Criterion) {
    let mut group = c.benchmark_group(GROUP_NAME);
    group
        .sample_size(SAMPLE_SIZE)
        .measurement_time(MEASUREMENT_TIME);
    group.bench_function("small_rope", |b| {
        b.iter_batched(
            || {
                let mut rng = WyRand::new();
                let start = Rope::generate_random(&mut rng);
                (start, rng)
            },
            |(start, mut rng)| {
                black_box(start.random_mutate(&mut rng));
            },
            BatchSize::LargeInput,
        )
    });
    group.bench_function("large_rope", |b| {
        b.iter_batched(
            || {
                let mut rng = WyRand::new();
                let start = Rope::generate_random_large(&mut rng);
                (start, rng)
            },
            |(start, mut rng)| {
                black_box(start.random_mutate(&mut rng));
            },
            BatchSize::LargeInput,
        )
    });
    group.finish();
}

fn vec(c: &mut Criterion) {
    let mut group = c.benchmark_group(GROUP_NAME);
    group
        .sample_size(SAMPLE_SIZE)
        .measurement_time(MEASUREMENT_TIME);
    group.bench_function("small_vec", |b| {
        b.iter_batched(
            || {
                let mut rng = WyRand::new();
                let start = Vec::generate_random(&mut rng);
                (start, rng)
            },
            |(start, mut rng)| {
                black_box(start.random_mutate(&mut rng));
            },
            BatchSize::LargeInput,
        )
    });
    group.bench_function("large_vec", |b| {
        b.iter_batched(
            || {
                let mut rng = WyRand::new();
                let start = Vec::generate_random_large(&mut rng);
                (start, rng)
            },
            |(start, mut rng)| {
                black_box(start.random_mutate(&mut rng));
            },
            BatchSize::LargeInput,
        )
    });
    group.finish();
}
