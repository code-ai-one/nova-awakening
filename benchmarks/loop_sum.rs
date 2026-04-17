fn main() {
    let argc = std::env::args().count() as i64;
    let limit = std::hint::black_box(10_000_000_i64 + (argc - 1));
    let mut s: i64 = 0;
    let mut i: i64 = 1;
    while i <= limit { s += i; i += 1; }
    println!("sum={}", s);
}
