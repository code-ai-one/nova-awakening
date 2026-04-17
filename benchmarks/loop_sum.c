#include <stdio.h>
int main(int argc, char **argv) {
    (void)argv;
    volatile long limit = 10000000L + (argc - 1);
    long s = 0;
    for (long i = 1; i <= limit; ++i) s += i;
    printf("sum=%ld\n", s);
    return 0;
}
