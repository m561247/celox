/**
 * Verilator benchmark harness for linear_sec encoder/decoder (P=6).
 * Mirrors Celox benchmark_linear_sec in simulation.rs.
 *
 * Output format: BENCH <name> <nanoseconds>
 */

#include "VLinearSecTop.h"
#include "verilated.h"
#include <chrono>
#include <cstdint>
#include <cstdio>

using Clock = std::chrono::high_resolution_clock;

int main(int argc, char **argv) {
    Verilated::commandArgs(argc, argv);

    // --- simulation_eval_linear_sec_p6_x1 ---
    {
        VLinearSecTop *top = new VLinearSecTop;

        // Warm up
        for (uint64_t i = 0; i < 100000; i++) {
            top->i_word = i;
            top->eval();
        }

        const int ITERS = 100000;
        uint64_t input = 0;
        auto start = Clock::now();
        for (int i = 0; i < ITERS; i++) {
            top->i_word = input;
            top->eval();
            input++;
        }
        auto end = Clock::now();

        double ns =
            std::chrono::duration<double, std::nano>(end - start).count() /
            ITERS;
        std::printf("BENCH simulation_eval_linear_sec_p6_x1 %.2f\n", ns);
        delete top;
    }

    // --- simulation_eval_linear_sec_p6_x1000000 ---
    {
        VLinearSecTop *top = new VLinearSecTop;

        const int EVALS = 1000000;
        uint64_t input = 0;
        auto start = Clock::now();
        for (int i = 0; i < EVALS; i++) {
            top->i_word = input;
            top->eval();
            input++;
        }
        auto end = Clock::now();

        double ns =
            std::chrono::duration<double, std::nano>(end - start).count();
        std::printf("BENCH simulation_eval_linear_sec_p6_x1000000 %.2f\n", ns);
        delete top;
    }

    // --- testbench_eval_linear_sec_p6_x1000000 (eval + read corrected) ---
    {
        VLinearSecTop *top = new VLinearSecTop;

        volatile uint8_t sink = 0;
        const int EVALS = 1000000;
        uint64_t input = 0;
        auto start = Clock::now();
        for (int i = 0; i < EVALS; i++) {
            top->i_word = input;
            top->eval();
            sink = top->o_corrected;
            input++;
        }
        auto end = Clock::now();

        double ns =
            std::chrono::duration<double, std::nano>(end - start).count();
        std::printf("BENCH testbench_eval_linear_sec_p6_x1000000 %.2f\n", ns);
        (void)sink;
        delete top;
    }

    return 0;
}
