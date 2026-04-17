═══ Nova Benchmark ═══
Date: 2026年 04月 17日 星期五 12:11:33 CST
Host: x86_64 Linux
CPU:  Intel(R) Core(TM) i5-14600KF

=== fib35 ===
  C/gcc-O2   0.0118205 s
  Rust-O     0.0181839 s
  Nova       0.0484975 s
  ─── ratios ───
  Nova/C  = 4.10x
  Nova/Rs = 2.67x

=== loop_sum ===
  C/gcc-O2   0.000711679 s
  Rust-O     0.000861406 s
  Nova       0.00740385 s
  ─── ratios ───
  Nova/C  = 10.40x
  Nova/Rs = 8.60x

=== collatz ===
  C/gcc-O2   0.0257712 s
  Rust-O     0.0194009 s
  Nova       0.0544965 s
  ─── ratios ───
  Nova/C  = 2.11x
  Nova/Rs = 2.81x

═══ Summary ═══
产物大小 (bytes):
  fib35_c            16000
  fib35_rs           3954712
  fib35_nova         16384
  loop_sum_c         15968
  loop_sum_rs        3954552
  loop_sum_nova      16384
  collatz_c          15968
  collatz_rs         3954640
  collatz_nova       16384
