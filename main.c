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

#define _POSIX_C_SOURCE 200809L

#include "kd-forest.h"
#include "util.h"
#include "color.h"
#include <errno.h>
#include <math.h>
#include <png.h>
#include <setjmp.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#if __unix__
#  include <unistd.h>
#endif

// Possible generation modes
typedef enum {
  MODE_HUE_SORT,
  MODE_RANDOM,
} mode_t;

// Possible color spaces
typedef enum {
  COLOR_SPACE_RGB,
  COLOR_SPACE_LAB,
  COLOR_SPACE_LUV,
} color_space_t;

// Command-line options
typedef struct {
  unsigned int bit_depth;
  mode_t mode;
  color_space_t color_space;
  bool animate;
  const char *filename;
  unsigned int seed;
  bool help;
} options_t;

static bool parse_options(options_t *options, int argc, char *argv[]);
static void print_usage(FILE *file, const char *command);

// All-encompasing state struct
typedef struct {
  const options_t *options;
  unsigned int width;
  unsigned int height;
  size_t size;
  uint32_t *colors;
  png_byte **bitmap;
} state_t;

static void init_state(state_t *state, const options_t *options);
static void generate_image(const state_t *state);
static void destroy_state(state_t *state);

// Entry point
int
main(int argc, char *argv[])
{
  options_t options;
  if (!parse_options(&options, argc, argv)) {
    fprintf(stderr, "\n");
    print_usage(stderr, argv[0]);
    return EXIT_FAILURE;
  }

  if (options.help) {
    print_usage(stdout, argv[0]);
    return EXIT_SUCCESS;
  }

  state_t state;
  init_state(&state, &options);
  generate_image(&state);
  destroy_state(&state);
  return EXIT_SUCCESS;
}

static bool
parse_arg(int argc, char *argv[],
          const char *short_form, const char *long_form,
          const char **value, int *i, bool *error)
{
  size_t short_len = strlen(short_form);
  size_t long_len = strlen(long_form);

  const char *actual_form;
  const char *arg = argv[*i];
  const char *candidate = NULL;

  if (strncmp(arg, short_form, short_len) == 0) {
    actual_form = short_form;
    if (strlen(arg) > short_len) {
      candidate = arg + short_len;
    }
  } else if (strncmp(arg, long_form, long_len) == 0) {
    actual_form = long_form;
    if (strlen(arg) > long_len) {
      if (arg[long_len] == '=') {
        candidate = arg + long_len + 1;
      } else {
        return false;
      }
    }
  } else {
    return false;
  }

  if (value) {
    if (candidate) {
      *value = candidate;
    } else if (*i < argc - 1) {
      ++*i;
      *value = argv[*i];
    } else {
      fprintf(stderr, "Expected a value for %s\n", arg);
      *error = true;
      return false;
    }
  } else if (candidate) {
    fprintf(stderr, "Unexpected value for %s: `%s'\n",
            actual_form, candidate);
    *error = true;
    return false;
  }

  return true;
}

static bool
str_to_uint(const char *str, unsigned int *value)
{
  char *endptr;
  long result = strtol(str, &endptr, 10);
  if (*str == '\0' || *endptr != '\0') {
    return false;
  }
  if (result < 0 || result > UINT_MAX) {
    return false;
  }

  *value = result;
  return true;
}

static void
strcatinc(char **destp, const char *src)
{
  strcpy(*destp, src);
  *destp += strlen(src);
}

typedef enum {
  COLORIZE_NORMAL,
  COLORIZE_AT,
  COLORIZE_BANG,
  COLORIZE_STAR,
  COLORIZE_SHORT_OPTION,
  COLORIZE_LONG_OPTION,
} colorize_state_t;

static void
print_colorized(FILE *file, bool tty, const char *format, ...)
{
  const char *bold = tty ? "\033[1m" : "";
  const char *red = tty ? "\033[1;31m" : "";
  const char *green = tty ? "\033[1;32m" : "";
  const char *normal = tty ? "\033[0m" : "";

  size_t size = strlen(format) + 1;
  char colorized[16*size];
  char *builder = colorized;

  colorize_state_t state = COLORIZE_NORMAL;
  for (size_t i = 0; i < size; ++i) {
    char c = format[i];

    if (c == '\\') {
      *builder++ = format[++i];
      continue;
    }

    switch (state) {
    case COLORIZE_AT:
      if (c == '@') {
        strcatinc(&builder, normal);
        state = COLORIZE_NORMAL;
      } else {
        *builder++ = c;
      }
      break;

    case COLORIZE_BANG:
      if (c == '!') {
        strcatinc(&builder, normal);
        state = COLORIZE_NORMAL;
      } else {
        *builder++ = c;
      }
      break;

    case COLORIZE_STAR:
      if (c == '*') {
        strcatinc(&builder, normal);
        state = COLORIZE_NORMAL;
      } else {
        *builder++ = c;
      }
      break;

    case COLORIZE_SHORT_OPTION:
      *builder++ = c;
      strcatinc(&builder, normal);
      state = COLORIZE_NORMAL;
      break;

    case COLORIZE_LONG_OPTION:
      if (!isalpha(c) && c != '-') {
        strcatinc(&builder, normal);
        state = COLORIZE_NORMAL;
      }
      *builder++ = c;
      break;

    default:
      switch (c) {
      case '@':
        state = COLORIZE_AT;
        strcatinc(&builder, green);
        break;

      case '!':
        state = COLORIZE_BANG;
        strcatinc(&builder, bold);
        break;

      case '*':
        state = COLORIZE_STAR;
        strcatinc(&builder, red);
        break;

      case '-':
        if (c == '-') {
          if (format[i + 1] == '-') {
            state = COLORIZE_LONG_OPTION;
          } else {
            state = COLORIZE_SHORT_OPTION;
          }
          strcatinc(&builder, red);
        }
        *builder++ = c;
        break;

      default:
        *builder++ = c;
        break;
      }
      break;
    }
  }

  va_list args;
  va_start(args, format);
  vprintf(colorized, args);
  va_end(args);
}

static void
print_usage(FILE *file, const char *command)
{
#if __unix__
  bool tty = isatty(fileno(file));
#else
  bool tty = false;
#endif

  size_t length = strlen(command);
  char whitespace[length + 1];
  memset(whitespace, ' ', length);
  whitespace[length] = '\0';

#define usage(...) print_colorized(file, tty, __VA_ARGS__)
  usage("Usage:\n");
  usage("  !$! *%s* [-b|--bit-depth @DEPTH@]\n", command);
  usage("    %s [-s|--hue-sort] [-r|--random]\n", whitespace);
  usage("    %s [-c|--color-space @RGB@|@Lab@|@Luv@]\n", whitespace);
  usage("    %s [-a|--animate]\n", whitespace);
  usage("    %s [-o|--output @PATH@]\n", whitespace);
  usage("    %s [-e|--seed @SEED@]\n", whitespace);
  usage("    %s [-h|--help]\n", whitespace);
  usage("\n");
  usage("  -b, --bit-depth @DEPTH@:\n");
  usage("          Use all @DEPTH@\\-bit colors (!default!: @24@)\n\n");
  usage("  -s, --hue-sort:\n");
  usage("          Sort colors by hue first (!default!)\n");
  usage("  -r, --random:\n");
  usage("          Randomize colors first\n\n");
  usage("  -c, --color-space @RGB@|@Lab@|@Luv@:\n");
  usage("          Use the given color space (!default!: @Lab@)\n\n");
  usage("  -a, --animate:\n");
  usage("          Generate frames of an animation\n\n");
  usage("  -o, --output @PATH@:\n");
  usage("          Output a PNG file at @PATH@ (!default!: @kd\\-forest.png@)\n\n");
  usage("          If -a/--animate is specified, this is treated as a directory which\n");
  usage("          will hold many frames\n\n");
  usage("  -e, --seed @SEED@:\n");
  usage("          Seed the random number generator (!default!: @0@)\n\n");
  usage("  -h, --help:\n");
  usage("          Show this message\n");
#undef usage
}

static bool
parse_options(options_t *options, int argc, char *argv[])
{
  // Set defaults
  options->bit_depth = 24;
  options->mode = MODE_HUE_SORT;
  options->seed = 0;
  options->color_space = COLOR_SPACE_LAB;
  options->animate = false;
  options->filename = NULL;
  options->help = false;

  bool result = true;

  for (int i = 1; i < argc; ++i) {
    const char *value;
    bool error = false;

    if (parse_arg(argc, argv, "-b", "--bit-depth", &value, &i, &error)) {
      if (!str_to_uint(value, &options->bit_depth)
          || options->bit_depth <= 1
          || options->bit_depth > 24) {
        fprintf(stderr, "Invalid bit depth: `%s'\n", value);
        error = true;
      }
    } else if (parse_arg(argc, argv, "-s", "--hue-sort", NULL, &i, &error)) {
      options->mode = MODE_HUE_SORT;
    } else if (parse_arg(argc, argv, "-r", "--random", NULL, &i, &error)) {
      options->mode = MODE_RANDOM;
    } else if (parse_arg(argc, argv, "-e", "--seed", &value, &i, &error)) {
      if (!str_to_uint(value, &options->seed)) {
        fprintf(stderr, "Invalid random seed: `%s'\n", value);
        error = true;
      }
    } else if (parse_arg(argc, argv, "-a", "--animate", NULL, &i, &error)) {
      options->animate = true;
    } else if (parse_arg(argc, argv, "-o", "--output", &value, &i, &error)) {
      options->filename = value;
    } else if (parse_arg(argc, argv, "-c", "--color-space", &value, &i, &error)) {
      if (strcmp(value, "RGB") == 0) {
        options->color_space = COLOR_SPACE_RGB;
      } else if (strcmp(value, "Lab") == 0) {
        options->color_space = COLOR_SPACE_LAB;
      } else if (strcmp(value, "Luv") == 0) {
        options->color_space = COLOR_SPACE_LUV;
      } else {
        fprintf(stderr, "Invalid color space: `%s'\n", value);
      }
    } else if (parse_arg(argc, argv, "-h", "--help", NULL, &i, &error)) {
      options->help = true;
    } else if (!error) {
      fprintf(stderr, "Unexpected argument `%s'\n", argv[i]);
      error = true;
    }

    if (error) {
      result = false;
    }
  }

  // Default filename depends on -a flag
  if (!options->filename) {
    options->filename = options->animate ? "frames" : "kd-forest.png";
  }

  return result;
}

static uint32_t *
create_colors(const state_t *state)
{
  const unsigned int bit_depth = state->options->bit_depth;

  // From least to most perceptually important
  const unsigned int bskip = 1U << (24 - bit_depth)/3;
  const unsigned int rskip = 1U << (24 - bit_depth + 1)/3;
  const unsigned int gskip = 1U << (24 - bit_depth + 2)/3;

  uint32_t *colors = xmalloc(state->size*sizeof(uint32_t));
  for (unsigned int b = 0, i = 0; b < 0x100; b += bskip) {
    for (unsigned int g = 0; g < 0x100; g += gskip) {
      for (unsigned int r = 0; r < 0x100; r += rskip, ++i) {
        colors[i] = (r << 16) | (g << 8) | b;
      }
    }
  }

  switch (state->options->mode) {
  case MODE_HUE_SORT:
    qsort(colors, state->size, sizeof(uint32_t), color_comparator);
    break;

  case MODE_RANDOM:
    // Fisher-Yates shuffle
    for (unsigned int i = state->size; i-- > 0;) {
      unsigned int j = xrand(i + 1);
      uint32_t temp = colors[i];
      colors[i] = colors[j];
      colors[j] = temp;
    }
    break;
  }

  return colors;
}

static png_byte **
create_bitmap(const state_t *state)
{
  png_byte **rows = xmalloc(state->height*sizeof(png_byte *));
  const size_t row_size = 3*state->width*sizeof(png_byte);
  for (unsigned int i = 0; i < state->height; ++i) {
    rows[i] = xmalloc(row_size);
    memset(rows[i], 0, row_size);
  }
  return rows;
}

static void
init_state(state_t *state, const options_t *options)
{
  xsrand(options->seed);

  state->options = options;
  state->width = 1U << (options->bit_depth + 1)/2; // Round up
  state->height = 1U << options->bit_depth/2; // Round down
  state->size = (size_t)state->width*state->height;

  printf("Generating a %u-bit, %ux%u image (%zu pixels)\n",
         options->bit_depth, state->width, state->height, state->size);

  state->colors = create_colors(state);
  state->bitmap = create_bitmap(state);
}

static void generate_bitmap(const state_t *state);
static void write_png(const state_t *state, const char *filename);

static void
generate_image(const state_t *state)
{
  generate_bitmap(state);

  if (!state->options->animate) {
    write_png(state, state->options->filename);
  }
}

static void
destroy_state(state_t *state)
{
  for (unsigned int i = 0; i < state->height; ++i) {
    free(state->bitmap[i]);
  }
  free(state->bitmap);
  free(state->colors);
}

static kd_node_t *
try_neighbor(const state_t *state, kd_node_t *node, int dx, int dy)
{
  if (dx < 0 && node->x < -dx) {
    return NULL;
  } else if (dx > 0 && node->x + dx >= state->width) {
    return NULL;
  } else if (dy < 0 && node->y < -dy) {
    return NULL;
  } else if (dy > 0 && node->y + dy >= state->height) {
    return NULL;
  }

  return node + (int)state->width*dy + dx;
}

static kd_node_t *
next_neighbor(const state_t *state, kd_node_t *node)
{
  kd_node_t *neighbors[8];
  unsigned int size = 0;
  for (int dy = -1; dy <= 1; ++dy) {
    for (int dx = -1; dx <= 1; ++dx) {
      if (dx == 0 && dy == 0) {
        continue;
      }

      kd_node_t *neighbor = try_neighbor(state, node, dx, dy);
      if (neighbor && !neighbor->added) {
        neighbors[size++] = neighbor;
      }
    }
  }

  if (size == 0) {
    return NULL;
  }

  return neighbors[xrand(size)];
}

static void
remove_if_surrounded(const state_t *state, kd_forest_t *kdf, kd_node_t *node)
{
  if (node->added && !node->removed && next_neighbor(state, node) == NULL) {
    kdf_remove(kdf, node);
  }
}

static void
remove_non_boundary(const state_t *state, kd_forest_t *kdf, kd_node_t *node)
{
  for (int dy = -1; dy <= 1; ++dy) {
    for (int dx = -1; dx <= 1; ++dx) {
      kd_node_t *neighbor = try_neighbor(state, node, dx, dy);
      if (neighbor) {
        remove_if_surrounded(state, kdf, neighbor);
      }
    }
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
generate_bitmap(const state_t *state)
{
  kd_node_t *nodes = xmalloc(state->size*sizeof(kd_node_t));
  for (unsigned int y = 0, i = 0; y < state->height; ++y) {
    for (unsigned int x = 0; x < state->width; ++x, ++i) {
      kd_node_init(nodes + y*state->width + x, x, y);
    }
  }

  // Make the forest
  kd_forest_t kdf;
  kdf_init(&kdf);

  bool animate = state->options->animate;
  unsigned int frame = 0;
  char filename[strlen(state->options->filename) + 10];

  size_t max_size = 0;

  // Do multiple passes to get rid of artifacts in HUE_SORT mode
  unsigned int bit_depth = state->options->bit_depth;
  for (unsigned int i = 1, progress = 0; i <= bit_depth; ++i) {
    unsigned int stripe = 1 << i;

    for (unsigned int j = stripe/2 - 1; j < state->size; j += stripe, ++progress) {
      if (progress%state->width == 0) {
        if (animate) {
          sprintf(filename, "%s/%04u.png", state->options->filename, frame);
          write_png(state, filename);
          ++frame;
        }

        print_progress("%.2f%%\t| boundary size: %zu\t| max boundary size: %zu",
                       100.0*progress/state->size, kdf.size, max_size);
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

      kd_node_t *new_node;
      if (j == 0) {
        // First node goes in the center
        new_node = nodes + state->size/2 + state->width/2;
      } else {
        kd_node_t *nearest = kdf_find_nearest(&kdf, target);
        new_node = next_neighbor(state, nearest);
      }

      memcpy(new_node->coords, target, sizeof(target));
      kdf_insert(&kdf, new_node);
      remove_non_boundary(state, &kdf, new_node);

      if (kdf.size > max_size) {
        max_size = kdf.size;
      }

      png_byte *pixel = state->bitmap[new_node->y] + 3*new_node->x;
      color_unpack(pixel, color);
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

  print_progress("%.2f%%\t| boundary size: 0\t| max boundary size: %zu\n",
                 100.0, max_size);

  kdf_destroy(&kdf);
  free(nodes);
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
  png_set_IHDR(png_ptr, info_ptr, state->width, state->height, 8,
               PNG_COLOR_TYPE_RGB, PNG_INTERLACE_ADAM7,
               PNG_COMPRESSION_TYPE_DEFAULT, PNG_FILTER_TYPE_DEFAULT);
  png_set_sRGB_gAMA_and_cHRM(png_ptr, info_ptr, PNG_sRGB_INTENT_ABSOLUTE);
  png_set_rows(png_ptr, info_ptr, state->bitmap);
  png_write_png(png_ptr, info_ptr, PNG_TRANSFORM_IDENTITY, NULL);
  png_destroy_write_struct(&png_ptr, &info_ptr);
  fclose(file);
}
