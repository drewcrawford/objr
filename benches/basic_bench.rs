use criterion::{black_box, criterion_group, criterion_main, Criterion};
extern crate objr;
use objr::foundation::*;

fn alloc_init_description(c: &mut Criterion) {
    autoreleasepool(|pool| {
        c.bench_function("NSObject_alloc_init_description", |b| b.iter(|| {
            let class = NSObject::class();
            let instance = class.alloc_init(pool);
            let description = instance.description(pool).to_str(pool).len();
            black_box(description)
        }));
    });
}

criterion_group!(benches, alloc_init_description);
criterion_main!(benches);