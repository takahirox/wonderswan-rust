/* Headless adapter for gameblabla/oswan (external, unmodified core).
 * WsRun advances a fixed time slice, so normalize by actual presented frames.
 * Save file I/O and SDL presentation/playback are replaced by host callbacks. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <inttypes.h>
#include "WS.h"
#include "WSRender.h"
#include "WSApu.h"

static size_t presented, samples;
static size_t measured_start;
static int replay;
void graphics_paint(void) { ++presented; }
uint32_t WsInputGetState(void) {
    const unsigned directions[] = {5, 6, 7, 4};
    return replay ? 1u << directions[(presented - measured_start) / 64 % 4] : 0;
}
void WsLoadEeprom(void) {}
void WsSaveEeprom(void) {}
void WsRelease(void) {}
void Pause_Sound(void) {}
void Sound_APUClose(void) {}
void Sound_APU_Start(void) {}
void Sound_APU_End(void) {}
static void run(void) {
    WsRun();
    samples += (wBuf - rBuf + SND_RNGSIZE) % SND_RNGSIZE * 2;
    rBuf = wBuf;
}
static double now(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts)) abort();
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}
static uint64_t hash(const void *data, size_t size) {
    const unsigned char *p = data;
    uint64_t h = UINT64_C(0xcbf29ce484222325);
    for (size_t i = 0; i < size; ++i) h = (h ^ p[i]) * UINT64_C(0x100000001b3);
    return h;
}
int main(int argc, char **argv) {
    if (argc != 4) { fprintf(stderr, "usage: oswan-throughput ROM WARMUP FRAMES\n"); return 1; }
    size_t warmup = strtoul(argv[2], NULL, 10), frames = strtoul(argv[3], NULL, 10);
    if (!frames) return 1;
    FILE *file = fopen(argv[1], "rb");
    if (!file) { perror(argv[1]); return 1; }
    fseek(file, 0, SEEK_END); long length = ftell(file); rewind(file);
    if (length != 131072 && length != 262144) {
        fprintf(stderr, "adapter supports the 128/256 KiB benchmark ROMs only\n"); return 1;
    }
    uint8_t *rom = malloc((size_t)length);
    if (!rom || fread(rom, 1, (size_t)length, file) != (size_t)length) return 1;
    fclose(file);
    ROMBanks = (uint16_t)(length / 65536);
    memset(MemDummy, 0xa0, sizeof(MemDummy));
    for (unsigned i = 0; i < 256; ++i) { ROMMap[i] = MemDummy; RAMMap[i] = MemDummy; }
    for (unsigned i = 0; i < ROMBanks; ++i) ROMMap[256 - ROMBanks + i] = rom + i * 65536;
    unsigned save = rom[length - 5];
    if (save > 3) { fprintf(stderr, "adapter supports no save or SRAM only\n"); return 1; }
    RAMSize = save ? 1u << (save == 1 ? 15 : save == 2 ? 16 : 17) : 0;
    RAMBanks = RAMSize > 65536 ? 2 : 1;
    for (unsigned i = 0; i < RAMBanks; ++i) RAMMap[i] = calloc(65536, 1);
    CartKind = 0;
    apuInit(); WsReset();
    while (presented < warmup) run();
    size_t before = presented; samples = 0;
    measured_start = before;
    replay = getenv("WS_PERF_INPUT_REPLAY") != NULL;
    double start = now();
    while (presented - before < frames) run();
    double elapsed = now() - start;
    size_t actual = presented - before;
    printf("{\"frames\":%zu,\"seconds\":%.9f,\"us_per_frame\":%.3f,\"fps\":%.1f,\"sample_values\":%zu,\"video_hash\":\"%016" PRIx64 "\",\"ram_hash\":\"%016" PRIx64 "\",\"markers\":[%u,%u,%u]}\n",
           actual, elapsed, elapsed * 1e6 / actual, actual / elapsed, samples,
           hash(FrameBuffer, 240 * 144 * sizeof(uint16_t)), hash(IRAM, sizeof(IRAM)),
           IRAM[0x400] | IRAM[0x401]<<8, IRAM[0x402] | IRAM[0x403]<<8, IRAM[0x404] | IRAM[0x405]<<8);
    for (unsigned i = 0; i < RAMBanks; ++i) free(RAMMap[i]);
    free(rom);
    return 0;
}
