#include <stdio.h>
static long collatz(long n) {
    long c = 0;
    while (n > 1) { n = (n & 1) ? n*3+1 : n/2; c++; }
    return c;
}
int main(void) {
    long total = 0;
    for (long i = 1; i <= 200000; ++i) total += collatz(i);
    printf("total=%ld\n", total);
    return 0;
}
