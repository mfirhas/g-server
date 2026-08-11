pub fn bench_all(c: &mut criterion::Criterion) {
    noop_bench(c);
}

pub fn noop_bench(_: &mut criterion::Criterion) {}
