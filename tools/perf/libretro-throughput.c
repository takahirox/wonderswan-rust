/* Independently authored headless libretro host. No emulator code is bundled.
 * Build with the reference core's libretro.h include directory and -ldl on Linux.
 * Video/audio production stays enabled; presentation and playback are excluded. */
#include <libretro.h>
#include <dlfcn.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static const void *last_video;
static unsigned last_width, last_height, bytes_per_pixel = 2;
static size_t last_pitch, samples, video_frames;
static uint64_t hash(const void *data, size_t size, uint64_t h) {
    const unsigned char *p = data;
    for (size_t i = 0; i < size; ++i) h = (h ^ p[i]) * UINT64_C(0x100000001b3);
    return h;
}
static bool environment(unsigned cmd, void *data) {
    switch (cmd) {
    case RETRO_ENVIRONMENT_GET_CAN_DUPE: *(bool *)data = true; return true;
    case RETRO_ENVIRONMENT_SET_PIXEL_FORMAT:
        bytes_per_pixel = *(enum retro_pixel_format *)data == RETRO_PIXEL_FORMAT_XRGB8888 ? 4 : 2;
        return true;
    case RETRO_ENVIRONMENT_GET_VARIABLE: {
        struct retro_variable *v = data;
        if (!strcmp(v->key, "wswan_sound_sample_rate")) v->value = "24000";
        else if (!strcmp(v->key, "wswan_60hz_mode")) v->value = "disabled";
        else if (!strcmp(v->key, "wswan_frameskip")) v->value = "disabled";
        else if (!strcmp(v->key, "wswan_sound_low_pass")) v->value = "disabled";
        else return false;
        return true;
    }
    case RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE: *(bool *)data = false; return true;
    default: return false;
    }
}
static void video(const void *data, unsigned w, unsigned h, size_t pitch) {
    if (data) { last_video = data; last_width = w; last_height = h; last_pitch = pitch; ++video_frames; }
}
static size_t audio(const int16_t *data, size_t frames) {
    /* Only retain the final block's checksum. Hashing every sample here would
     * disproportionately charge the reference core for the host's verification. */
    (void)data;
    samples += frames * 2;
    return frames;
}
static void audio_one(int16_t l, int16_t r) { (void)l; (void)r; samples += 2; }
static void poll(void) {}
static int16_t input(unsigned port, unsigned device, unsigned index, unsigned id) {
    (void)port; (void)device; (void)index; (void)id; return 0;
}
static double now(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts)) abort();
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}
#define LOAD(name) __typeof__(&name) p_##name = (__typeof__(&name))dlsym(core, #name); if (!p_##name) { fprintf(stderr, "missing %s\n", #name); return 1; }
int main(int argc, char **argv) {
    if (argc != 5) { fprintf(stderr, "usage: libretro-throughput CORE ROM WARMUP FRAMES\n"); return 1; }
    unsigned warmup = (unsigned)strtoul(argv[3], NULL, 10), frames = (unsigned)strtoul(argv[4], NULL, 10);
    if (!frames) return 1;
    void *core = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!core) { fprintf(stderr, "%s\n", dlerror()); return 1; }
    LOAD(retro_set_environment); LOAD(retro_set_video_refresh); LOAD(retro_set_audio_sample_batch);
    LOAD(retro_set_audio_sample); LOAD(retro_set_input_poll); LOAD(retro_set_input_state);
    LOAD(retro_init); LOAD(retro_load_game); LOAD(retro_run); LOAD(retro_unload_game); LOAD(retro_deinit);
    LOAD(retro_get_system_av_info); LOAD(retro_get_memory_data); LOAD(retro_get_memory_size);
    FILE *file = fopen(argv[2], "rb");
    if (!file) { perror(argv[2]); return 1; }
    fseek(file, 0, SEEK_END); long length = ftell(file); rewind(file);
    if (length <= 0) return 1;
    void *rom = malloc((size_t)length);
    if (!rom || fread(rom, 1, (size_t)length, file) != (size_t)length) return 1;
    fclose(file);
    p_retro_set_environment(environment); p_retro_set_video_refresh(video);
    p_retro_set_audio_sample_batch(audio); p_retro_set_audio_sample(audio_one);
    p_retro_set_input_poll(poll); p_retro_set_input_state(input); p_retro_init();
    struct retro_game_info game = {argv[2], rom, (size_t)length, NULL};
    if (!p_retro_load_game(&game)) return 1;
    for (unsigned i = 0; i < warmup; ++i) p_retro_run();
    samples = video_frames = 0;
    double start = now();
    for (unsigned i = 0; i < frames; ++i) p_retro_run();
    double elapsed = now() - start;
    if (!last_video || video_frames != frames || !samples) { fprintf(stderr, "missing video/audio output\n"); return 1; }
    uint64_t video_hash = UINT64_C(0xcbf29ce484222325);
    for (unsigned y = 0; y < last_height; ++y)
        video_hash = hash((const char *)last_video + y * last_pitch, last_width * bytes_per_pixel, video_hash);
    struct retro_system_av_info av; p_retro_get_system_av_info(&av);
    size_t ram_size = p_retro_get_memory_size(RETRO_MEMORY_SYSTEM_RAM);
    void *ram = p_retro_get_memory_data(RETRO_MEMORY_SYSTEM_RAM);
    uint64_t ram_hash = ram ? hash(ram, ram_size, UINT64_C(0xcbf29ce484222325)) : 0;
    unsigned markers[3] = {0};
    if (ram && ram_size >= 0x406) {
        const uint8_t *bytes = ram;
        for (unsigned i = 0; i < 3; ++i) markers[i] = bytes[0x400 + i*2] | bytes[0x401 + i*2] << 8;
    }
    printf("{\"frames\":%u,\"seconds\":%.9f,\"us_per_frame\":%.3f,\"fps\":%.1f,\"sample_values\":%zu,\"video_frames\":%zu,\"sample_rate\":%.0f,\"emulated_fps\":%.6f,\"video_hash\":\"%016" PRIx64 "\",\"ram_hash\":\"%016" PRIx64 "\",\"markers\":[%u,%u,%u]}\n",
           frames, elapsed, elapsed * 1e6 / frames, frames / elapsed, samples, video_frames, av.timing.sample_rate, av.timing.fps, video_hash, ram_hash, markers[0], markers[1], markers[2]);
    p_retro_unload_game(); p_retro_deinit(); free(rom); dlclose(core);
    return 0;
}
