/*********************************************************************
 * kd-forest                                                         *
 * Copyright (C) 2014 Tavian Barnes <tavianator@tavianator.com>      *
 *                                                                   *
 * This program is free software. It comes without any warranty, to  *
 * the extent permitted by applicable law. You can redistribute it   *
 * and/or modify it under the terms of the Do What The Fuck You Want *
 * To Public License, Version 2, as published by Sam Hocevar. See    *
 * the COPYING file or http://www.wtfpl.net/ for more details.       *
 *********************************************************************/

#include "color.h"
#include "generate.h"
#include "kd-forest.h"
#include "options.h"
#include "util.h"
#include <math.h>
#include <png.h>
#include <setjmp.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#if __unix__
#  include <unistd.h>
#endif

// A single pixel in all its glory
typedef struct {
  double value[KD_DIMEN];
  kd_node_t *node;
  unsigned int x, y;
  bool filled;
} pixel_t;

// All-encompasing state struct
typedef struct {
  const options_t *options;
  uint32_t *colors;
  pixel_t *pixels;
  png_byte **bitmap;
} state_t;

static void init_state(state_t *state, const options_t *options);
static void generate_image(state_t *state);
static void destroy_state(state_t *state);

// Entry point
int
main(int argc, char *argv[])
{
  options_t options;
  if (!parse_options(&options, argc, argv)) {
    fprintf(stderr, "\n");
    print_usage(stderr, argv[0], options.help);
    return EXIT_FAILURE;
  }

  if (options.help) {
    print_usage(stdout, argv[0], true);
    return EXIT_SUCCESS;
  }

  state_t state;
  init_state(&state, &options);
  generate_image(&state);
  destroy_state(&state);
  return EXIT_SUCCESS;
}

static pixel_t *
create_pixels(const options_t *options)
{
  pixel_t *pixels = xmalloc(options->npixels*sizeof(pixel_t));
  for (unsigned int y = 0, i = 0; y < options->height; ++y) {
    for (unsigned int x = 0; x < options->width; ++x, ++i) {
      pixel_t *pixel = pixels + i;
      pixel->node = NULL;
      pixel->x = x;
      pixel->y = y;
      pixel->filled = false;
    }
  }
  return pixels;
}

static png_byte **
create_bitmap(const options_t *options)
{
  png_byte **rows = xmalloc(options->height*sizeof(png_byte *));
  const size_t row_size = 4*options->width*sizeof(png_byte);
  for (unsigned int i = 0; i < options->height; ++i) {
    rows[i] = xmalloc(row_size);
    memset(rows[i], 0, row_size);
  }
  return rows;
}

static void
init_state(state_t *state, const options_t *options)
{
  printf("Generating a %u-bit, %ux%u image (%zu pixels)\n",
         options->bit_depth, options->width, options->height, options->npixels);

  xsrand(options->seed);

  state->options = options;
  state->colors = generate_colors(options);
  state->pixels = create_pixels(options);
  state->bitmap = create_bitmap(options);
}

static void generate_bitmap(state_t *state);
static void write_png(const state_t *state, const char *filename);

static void
generate_image(state_t *state)
{
  generate_bitmap(state);

  if (!state->options->animate) {
    write_png(state, state->options->filename);
  }
}

static void
destroy_state(state_t *state)
{
  for (unsigned int i = 0; i < state->options->height; ++i) {
    free(state->bitmap[i]);
  }
  free(state->bitmap);
  free(state->pixels);
  free(state->colors);
}

static pixel_t *
get_pixel(const state_t *state, unsigned int x, unsigned int y)
{
  return state->pixels + state->options->width*y + x;
}

static pixel_t *
try_neighbor(const state_t *state, pixel_t *pixel, int dx, int dy)
{
  if (dx < 0 && pixel->x < -dx) {
    return NULL;
  } else if (dx > 0 && pixel->x + dx >= state->options->width) {
    return NULL;
  } else if (dy < 0 && pixel->y < -dy) {
    return NULL;
  } else if (dy > 0 && pixel->y + dy >= state->options->height) {
    return NULL;
  }

  return pixel + (int)state->options->width*dy + dx;
}

static unsigned int
get_all_neighbors(const state_t *state, pixel_t *pixel, pixel_t *neighbors[8])
{
  unsigned int size = 0;

  for (int dy = -1; dy <= 1; ++dy) {
    for (int dx = -1; dx <= 1; ++dx) {
      if (dx == 0 && dy == 0) {
        continue;
      }

      pixel_t *neighbor = try_neighbor(state, pixel, dx, dy);
      if (neighbor) {
        neighbors[size++] = neighbor;
      }
    }
  }

  return size;
}

static unsigned int
get_neighbors(const state_t *state, pixel_t *pixel, pixel_t *neighbors[8], bool filled)
{
  unsigned int size = 0;

  for (int dy = -1; dy <= 1; ++dy) {
    for (int dx = -1; dx <= 1; ++dx) {
      if (dx == 0 && dy == 0) {
        continue;
      }

      pixel_t *neighbor = try_neighbor(state, pixel, dx, dy);
      if (neighbor && neighbor->filled == filled) {
        neighbors[size++] = neighbor;
      }
    }
  }

  return size;
}

static pixel_t *
select_empty_neighbor(const state_t *state, pixel_t *pixel)
{
  pixel_t *neighbors[8];
  unsigned int size = get_neighbors(state, pixel, neighbors, false);
  return neighbors[xrand(size)];
}

static pixel_t *
find_next_pixel(const state_t *state, const kd_forest_t *kdf, const double target[KD_DIMEN])
{
  kd_node_t *nearest = kdf_find_nearest(kdf, target);
  pixel_t *pixel = get_pixel(state, nearest->x, nearest->y);

  if (state->options->selection == SELECTION_MIN) {
    pixel = select_empty_neighbor(state, pixel);
  }

  return pixel;
}

static void
ensure_pixel_removed(kd_forest_t *kdf, pixel_t *pixel)
{
  if (pixel->node) {
    kdf_remove(kdf, pixel->node);
    pixel->node = NULL;
  }
}

static bool
has_empty_neighbors(const state_t *state, pixel_t *pixel)
{
  for (int dy = -1; dy <= 1; ++dy) {
    for (int dx = -1; dx <= 1; ++dx) {
      if (dx == 0 && dy == 0) {
        continue;
      }

      pixel_t *neighbor = try_neighbor(state, pixel, dx, dy);
      if (neighbor && !neighbor->filled) {
        return true;
      }
    }
  }

  return false;
}

static void
insert_new_pixel_min(state_t *state, kd_forest_t *kdf, pixel_t *pixel)
{
  pixel->filled = true;

  if (has_empty_neighbors(state, pixel)) {
    pixel->node = new_kd_node(pixel->value, pixel->x, pixel->y);
    kdf_insert(kdf, pixel->node);
  }

  pixel_t *neighbors[8];
  unsigned int size = get_all_neighbors(state, pixel, neighbors);
  for (unsigned int i = 0; i < size; ++i) {
    pixel_t *neighbor = neighbors[i];
    if (!has_empty_neighbors(state, neighbor)) {
      ensure_pixel_removed(kdf, neighbor);
    }
  }
}

static void
insert_new_pixel_mean(state_t *state, kd_forest_t *kdf, pixel_t *pixel)
{
  pixel->filled = true;
  ensure_pixel_removed(kdf, pixel);

  pixel_t *neighbors[8];
  unsigned int size = get_neighbors(state, pixel, neighbors, false);
  for (unsigned int i = 0; i < size; ++i) {
    pixel_t *neighbor = neighbors[i];

    double value[KD_DIMEN] = { 0.0 };

    pixel_t *filled[8];
    unsigned int nfilled = get_neighbors(state, neighbor, filled, true);
    for (unsigned int j = 0; j < nfilled; ++j) {
      for (unsigned int k = 0; k < KD_DIMEN; ++k) {
        value[k] += filled[j]->value[k];
      }
    }

    for (unsigned int j = 0; j < KD_DIMEN; ++j) {
      value[j] /= nfilled;
    }

    ensure_pixel_removed(kdf, neighbor);
    neighbor->node = new_kd_node(value, neighbor->x, neighbor->y);
    kdf_insert(kdf, neighbor->node);
  }
}

static void
insert_new_pixel(state_t *state, kd_forest_t *kdf, pixel_t *pixel)
{
  switch (state->options->selection) {
  case SELECTION_MIN:
    insert_new_pixel_min(state, kdf, pixel);
    break;

  case SELECTION_MEAN:
    insert_new_pixel_mean(state, kdf, pixel);
    break;
  }
}

static void
print_progress(const char *format, ...)
{
#if __unix__
  static bool tty_checked = false;
  static bool tty = false;
  if (!tty_checked) {
    tty = isatty(STDOUT_FILENO);
    tty_checked = true;
  }
  const char *clear_line = tty ? "\033[2K\r" : "";
  const char *new_line = tty ? "" : "\n";
#else
  const char *clear_line = "";
  const char *new_line = "\n";
#endif

  printf("%s", clear_line);

  va_list args;
  va_start(args, format);
  vprintf(format, args);
  va_end(args);

  printf("%s", new_line);
  fflush(stdout);
}

static void
generate_bitmap(state_t *state)
{
  // Make the forest
  kd_forest_t kdf;
  kdf_init(&kdf);

  bool animate = state->options->animate;
  unsigned int frame = 0;
  char filename[strlen(state->options->filename) + 10];

  size_t max_size = 0;
  unsigned int update_interval = 1U << (state->options->bit_depth + 1)/2;

  // Do multiple passes to get rid of artifacts in HUE_SORT mode
  unsigned int bit_depth = state->options->bit_depth;
  for (unsigned int i = 1, progress = 0; i <= bit_depth + 1; ++i) {
    unsigned int stripe = 1 << i;

    for (unsigned int j = stripe/2 - 1; j < state->options->ncolors; j += stripe, ++progress) {
      if (progress % update_interval == 0) {
        if (animate) {
          sprintf(filename, "%s/%04u.png", state->options->filename, frame);
          write_png(state, filename);
          ++frame;
        }

        print_progress("%.2f%%\t| boundary size: %zu\t| max boundary size: %zu",
                       100.0*progress/state->options->ncolors, kdf.size, max_size);
      }

      uint32_t color = state->colors[j];

      double target[KD_DIMEN];
      switch (state->options->color_space) {
      case COLOR_SPACE_RGB:
        color_set_RGB(target, color);
        break;
      case COLOR_SPACE_LAB:
        color_set_Lab(target, color);
        break;
      case COLOR_SPACE_LUV:
        color_set_Luv(target, color);
        break;
      }

      pixel_t *pixel;
      if (j == 0) {
        pixel = get_pixel(state, state->options->x, state->options->y);
      } else {
        pixel = find_next_pixel(state, &kdf, target);
      }

      memcpy(pixel->value, target, sizeof(target));
      insert_new_pixel(state, &kdf, pixel);
      if (kdf.size > max_size) {
        max_size = kdf.size;
      }

      png_byte *png_pixel = state->bitmap[pixel->y] + 4*pixel->x;
      color_unpack(png_pixel, color);
      png_pixel[3] = 0xFF;
    }
  }

  if (animate) {
#if __unix__
    sprintf(filename, "%s/last.png", state->options->filename);
    write_png(state, filename);

    for (int i = 0; i < 120; ++i) {
      sprintf(filename, "%s/%04u.png", state->options->filename, frame + i);
      if (symlink("last.png", filename) != 0) {
        abort();
      }
    }
#else
    for (int i = 0; i < 120; ++i) {
      sprintf(filename, "%s/%04u.png", state->options->filename, frame + i);
      write_png(state, filename);
    }
#endif
  }

  print_progress("%.2f%%\t| boundary size: %zu\t| max boundary size: %zu\n",
                 100.0, kdf.size, max_size);

  kdf_destroy(&kdf);
}

static void
write_png(const state_t *state, const char *filename)
{
  FILE *file = fopen(filename, "wb");
  if (!file) {
    abort();
  }

  png_struct *png_ptr =
    png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, NULL, NULL);
  if (!png_ptr) {
    abort();
  }

  png_info *info_ptr = png_create_info_struct(png_ptr);
  if (!info_ptr) {
    abort();
  }

  // libpng will longjmp here if it encounters an error from now on
  if (setjmp(png_jmpbuf(png_ptr))) {
    abort();
  }

  png_init_io(png_ptr, file);
  png_set_IHDR(png_ptr, info_ptr, state->options->width, state->options->height, 8,
               PNG_COLOR_TYPE_RGBA, PNG_INTERLACE_ADAM7,
               PNG_COMPRESSION_TYPE_DEFAULT, PNG_FILTER_TYPE_DEFAULT);
  png_set_sRGB_gAMA_and_cHRM(png_ptr, info_ptr, PNG_sRGB_INTENT_ABSOLUTE);
  png_set_rows(png_ptr, info_ptr, state->bitmap);
  png_write_png(png_ptr, info_ptr, PNG_TRANSFORM_IDENTITY, NULL);
  png_destroy_write_struct(&png_ptr, &info_ptr);
  fclose(file);
}
