# Nova vs C(gcc) vs Rust Benchmark

## 目的
诚实对比 Nova 原生 AOT 编译产物的性能，对标 GCC 和 Rustc。

## Workload
1. **fib35** — 递归 fib(35)，测函数调用开销
2. **sieve** — 埃拉托斯特尼筛法到 1,000,000，测循环+整数算术
3. **loop_sum** — 累加 1..10,000,000，测循环+加法

## 测量方法
- `time -f "%e"` 取 wall-clock 秒数
- 每个用例跑 3 次取最快一次

## 运行
```bash
bash run_bench.sh
```
