fn collatz(mut n: i64) -> i64 {
    let mut c = 0i64;
    while n > 1 { n = if n & 1 == 0 { n / 2 } else { n*3+1 }; c += 1; }
    c
}
fn main() {
    let mut total: i64 = 0;
    for i in 1..=200_000i64 { total += collatz(i); }
    println!("total={}", total);
}
