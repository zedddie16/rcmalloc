use std::time::Instant;
// if not explicitly used, uses default allocator
#[allow(unused_imports)]
use rcmalloc::*;

fn main() {
    let mut counter = 0;
    let timer = Instant::now();
    while counter < 1310590_u64 {
        let _v = 1_u64;
        counter += 1;
        println!("counter: {}", counter);
    }
    let elapsed = timer.elapsed();

    let timer2 = Instant::now();

    counter = 0;
    while counter < 1310590_u64 {
        let _v = Box::new(1_u64);
        counter += 1;
        println!("counter: {}", counter);
    }
    let elapsed2 = timer2.elapsed();

    println!("time spent by STACK allocations ==> {:?}", elapsed);
    println!("time spent by HEAP allocations ==> {:?}", elapsed2);
}
