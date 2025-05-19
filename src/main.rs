// if not explicitly used, uses default allocator
#[allow(unused_imports)]
use rcmalloc::*;

fn main() {
    let v = Box::new(123_u64);
    println!("Allocated value: {}", v);

    let mut counter = 0;
    loop {
        let v = Box::new(true);

        counter += 1;
        println!("counter: {}", counter);
    }

    println!("counter: {}", counter);
}
