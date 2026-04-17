#include <stdio.h>

#define N 64
#define ITER 10000

static double M[N * N];
static double v[N];
static double r[N];

void matvec(double *m, int cols, double *vin, double *out) {
    int rows = (N * N) / cols;
    for (int i = 0; i < rows; i++) {
        double s = 0.0;
        for (int j = 0; j < cols; j++) {
            s += m[i * cols + j] * vin[j];
        }
        out[i] = s;
    }
}

int main(void) {
    for (int i = 0; i < N * N; i++) M[i] = 0.5;
    for (int i = 0; i < N; i++) v[i] = 1.0;
    double checksum = 0.0;
    for (int it = 0; it < ITER; it++) {
        matvec(M, N, v, r);
        checksum += r[0];
    }
    printf("iter=%d\n", ITER);
    printf("checksum=%f\n", checksum);
    return 0;
}
