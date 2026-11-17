/* The headless core uses no SDL types or calls. Keep the include boundary only. */
#ifndef WS_PERF_SDL_SHIM_H
#define WS_PERF_SDL_SHIM_H
#include <stdint.h>
#include <string.h>
typedef struct SDL_Surface SDL_Surface;
#endif
