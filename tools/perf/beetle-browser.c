/* Standalone benchmark bridge, linked against an external Beetle archive. */
#include <libretro.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

static const uint8_t *video_data;
static size_t video_pitch, video_frames, pcm_values;
static uint32_t rgba[224 * 144];
static float pcm[4096];
static uint16_t buttons;
static double sample_rate, frame_rate;

static bool environment(unsigned command, void *data) {
    switch (command) {
    case RETRO_ENVIRONMENT_GET_CAN_DUPE:
        *(bool *)data = true;
        return true;
    case RETRO_ENVIRONMENT_SET_PIXEL_FORMAT:
        return *(enum retro_pixel_format *)data == RETRO_PIXEL_FORMAT_RGB565;
    case RETRO_ENVIRONMENT_GET_VARIABLE: {
        struct retro_variable *option = data;
        if (!strcmp(option->key, "wswan_sound_sample_rate")) option->value = "24000";
        else if (!strcmp(option->key, "wswan_60hz_mode") ||
                 !strcmp(option->key, "wswan_frameskip") ||
                 !strcmp(option->key, "wswan_sound_low_pass")) option->value = "disabled";
        else return false;
        return true;
    }
    case RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE:
        *(bool *)data = false;
        return true;
    default:
        return false;
    }
}

static void video(const void *data, unsigned width, unsigned height, size_t pitch) {
    if (!data || width != 224 || height != 144) abort();
    video_data = data;
    video_pitch = pitch;
    ++video_frames;
}

static void audio(int16_t left, int16_t right) {
    if (pcm_values + 2 > sizeof pcm / sizeof *pcm) abort();
    pcm[pcm_values++] = left / 32768.0f;
    pcm[pcm_values++] = right / 32768.0f;
}

static size_t batch(const int16_t *data, size_t frames) {
    for (size_t i = 0; i < frames; ++i) audio(data[2 * i], data[2 * i + 1]);
    return frames;
}

static void poll(void) {}

static int16_t input(unsigned port, unsigned device, unsigned index, unsigned id) {
    return port == 0 && device == RETRO_DEVICE_JOYPAD && index == 0 &&
           id < 16 && (buttons & (1u << id)) != 0;
}

bool bench_open(const void *rom, size_t size) {
    retro_set_environment(environment);
    retro_set_video_refresh(video);
    retro_set_audio_sample(audio);
    retro_set_audio_sample_batch(batch);
    retro_set_input_poll(poll);
    retro_set_input_state(input);
    retro_init();
    struct retro_game_info game = {"fixture.ws", rom, size, NULL};
    if (!retro_load_game(&game)) return false;
    struct retro_system_av_info info;
    retro_get_system_av_info(&info);
    sample_rate = info.timing.sample_rate;
    frame_rate = info.timing.fps;
    return true;
}

void bench_step(uint16_t held) {
    buttons = held;
    pcm_values = video_frames = 0;
    retro_run();
    if (video_frames != 1 || !pcm_values) abort();
}

unsigned bench_pcm_values(void) { return (unsigned)pcm_values; }
const float *bench_pcm(void) { return pcm; }
double bench_sample_rate(void) { return sample_rate; }
double bench_frame_rate(void) { return frame_rate; }

unsigned bench_marker(unsigned offset) {
    const uint8_t *ram = retro_get_memory_data(RETRO_MEMORY_SYSTEM_RAM);
    if (offset >= 0x10000u - 1 || !ram) abort();
    return ram[offset] | (ram[offset + 1] << 8);
}

const uint32_t *bench_rgba(void) {
    for (unsigned y = 0; y < 144; ++y) {
        const uint16_t *scanline = (const uint16_t *)(video_data + y * video_pitch);
        for (unsigned x = 0; x < 224; ++x) {
            uint16_t value = scanline[x];
            unsigned r = (value >> 11) & 31, g = (value >> 5) & 63, b = value & 31;
            rgba[y * 224 + x] = 0xff000000u | ((b << 3 | b >> 2) << 16) |
                               ((g << 2 | g >> 4) << 8) | (r << 3 | r >> 2);
        }
    }
    return rgba;
}

void bench_close(void) { retro_unload_game(); retro_deinit(); }
