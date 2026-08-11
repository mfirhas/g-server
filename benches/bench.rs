use criterion::{Criterion, criterion_group, criterion_main};

mod benches {
    pub mod noop_bench;
}

fn criterion_config() -> Criterion {
    Criterion::default()
}

criterion_group! {
    name = bench_name;
    config = criterion_config();
    targets =
        benches::noop_bench::bench_all,

}

criterion_main!(bench_name,);
