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

#include "kd-forest.h"
#include "util.h"
#include "color.h"
#include <errno.h>
#include <math.h>
#include <png.h>
#include <setjmp.h>
#include <stdio.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#if __unix__
#  include <unistd.h>
#endif

// Number of trailing zero bits on each color chanel, set to zero for all
// 24-bit images
#define BIT_DEPTH 24
// Whether to sort by hue
#define HUE_SORT true

// Which color space to use
#define USE_RGB false
#define USE_LAB true
#define USE_LUV false

// Computed constants
static const unsigned int WIDTH = 1U << (BIT_DEPTH + 1)/2; // Round up
static const unsigned int HEIGHT = 1U << (BIT_DEPTH)/2;    // Round down
static const unsigned int SIZE = 1U << BIT_DEPTH;

static unsigned int
rand_in(unsigned int range)
{
  // Compensate for bias if (RAND_MAX + 1) isn't a multiple of range
  unsigned int limit = RAND_MAX + 1U - ((RAND_MAX + 1U)%range);
  unsigned int res;
  do {
    res = rand();
  } while (res >= limit);
  return res%range;
}

static kd_node_t *
try_neighbor(kd_node_t *node, int dx, int dy)
{
  if (dx < 0 && node->x < -dx) {
    return NULL;
  } else if (dx > 0 && node->x + dx >= WIDTH) {
    return NULL;
  } else if (dy < 0 && node->y < -dy) {
    return NULL;
  } else if (dy > 0 && node->y + dy >= HEIGHT) {
    return NULL;
  }

  return node + (int)WIDTH*dy + dx;
}

// Star pattern
static int neighbor_order[][2] = {
  { -1, -1 },
  {  0, +1 },
  { +1, -1 },
  { -1,  0 },
  { +1, +1 },
  {  0, -1 },
  { -1, +1 },
  { +1,  0 },
};

static kd_node_t *
next_neighbor(kd_node_t *node)
{
  unsigned int first = rand_in(8);
  for (unsigned int i = first; i < first + 8; ++i) {
    int *delta = neighbor_order[i%8];
    kd_node_t *neighbor = try_neighbor(node, delta[0], delta[1]);
    if (neighbor && !neighbor->added) {
      return neighbor;
    }
  }

  return NULL;
}

static void
remove_if_surrounded(kd_forest_t *kdf, kd_node_t *node)
{
  if (node->added && !node->removed && next_neighbor(node) == NULL) {
    kdf_remove(kdf, node);
  }
}

static void
remove_non_boundary(kd_forest_t *kdf, kd_node_t *node)
{
  for (int dy = -1; dy <= 1; ++dy) {
    for (int dx = -1; dx <= 1; ++dx) {
      kd_node_t *neighbor = try_neighbor(node, dx, dy);
      if (neighbor) {
        remove_if_surrounded(kdf, neighbor);
      }
    }
  }
}

static uint32_t *
create_colors(void)
{
  // From least to most perceptually important
  const unsigned int bskip = 1U << (24 - BIT_DEPTH)/3;
  const unsigned int rskip = 1U << (24 - BIT_DEPTH + 1)/3;
  const unsigned int gskip = 1U << (24 - BIT_DEPTH + 2)/3;

  uint32_t *colors = xmalloc(SIZE*sizeof(uint32_t));
  for (unsigned int b = 0, i = 0; b < 0x100; b += bskip) {
    for (unsigned int g = 0; g < 0x100; g += gskip) {
      for (unsigned int r = 0; r < 0x100; r += rskip, ++i) {
        colors[i] = (r << 16) | (g << 8) | b;
      }
    }
  }

  if (HUE_SORT) {
    qsort(colors, SIZE, sizeof(uint32_t), color_comparator);
  } else {
    // Fisher-Yates shuffle
    for (unsigned int i = SIZE; i-- > 0;) {
      unsigned int j = rand_in(i + 1);
      uint32_t temp = colors[i];
      colors[i] = colors[j];
      colors[j] = temp;
    }
  }

  return colors;
}

static kd_node_t *
create_kd_nodes(void)
{
  return xmalloc(SIZE*sizeof(kd_node_t));
}

static png_byte **
create_bitmap(void)
{
  png_byte **rows = xmalloc(HEIGHT*sizeof(png_byte *));
  const size_t row_size = 3*WIDTH*sizeof(png_byte);
  for (unsigned int i = 0; i < HEIGHT; ++i) {
    rows[i] = xmalloc(row_size);
    memset(rows[i], 0, row_size);
  }
  return rows;
}

static void
generate_image(const uint32_t *colors, kd_node_t *nodes,
               unsigned int initial_x, unsigned int initial_y,
               png_byte **bitmap)
{
  for (unsigned int y = 0, i = 0; y < HEIGHT; ++y) {
    for (unsigned int x = 0; x < WIDTH; ++x, ++i) {
      kd_node_init(nodes + y*WIDTH + x, x, y);
    }
  }

  // Make the forest
  kd_forest_t kdf;
  kdf_init(&kdf);

#if __unix__
  bool tty = isatty(1);
  const char *clear_line = tty ? "\033[2K\r" : "";
  const char *new_line = tty ? "" : "\n";
#else
  const char *clear_line = "";
  const char *new_line = "\n";
#endif

  size_t max_size = 0;

  // Do multiple passes to get rid of artifacts in HUE_SORT mode
  for (unsigned int i = 1, progress = 0; i <= BIT_DEPTH; ++i) {
    unsigned int stripe = 1 << i;

    for (unsigned int j = stripe/2 - 1; j < SIZE; j += stripe, ++progress) {
      if (progress%WIDTH == 0) {
        printf("%s%.2f%%\t| boundary size: %zu\t| max boundary size: %zu%s",
               clear_line, 100.0*progress/SIZE, kdf.size, max_size, new_line);
        fflush(stdout);
      }

      uint32_t color = colors[j];

      kd_node_t target;
#if USE_RGB
      color_set_RGB(target.coords, color);
#elif USE_LAB
      color_set_Lab(target.coords, color);
#elif USE_LUV
      color_set_Luv(target.coords, color);
#else
#  error "Pick one!"
#endif

      kd_node_t *new_node;
      if (j == 0) {
        // First node goes in the center
        new_node = nodes + WIDTH*initial_y + initial_x;
      } else {
        kd_node_t *nearest = kdf_find_nearest(&kdf, &target);
        new_node = next_neighbor(nearest);
      }

      memcpy(new_node->coords, target.coords, sizeof(target.coords));
      kdf_insert(&kdf, new_node);
      remove_non_boundary(&kdf, new_node);

      if (kdf.size > max_size) {
        max_size = kdf.size;
      }

      png_byte *pixel = bitmap[new_node->y] + 3*new_node->x;
      color_unpack(pixel, color);
    }
  }

  printf("%s%.2f%%\t| boundary size: 0\t| max boundary size: %zu\n",
         clear_line, 100.0, max_size);

  kdf_destroy(&kdf);
}

static void
write_png(const char *filename, png_byte **bitmap)
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
  png_set_IHDR(png_ptr, info_ptr, WIDTH, HEIGHT, 8,
               PNG_COLOR_TYPE_RGB, PNG_INTERLACE_ADAM7,
               PNG_COMPRESSION_TYPE_DEFAULT, PNG_FILTER_TYPE_DEFAULT);
  png_set_sRGB_gAMA_and_cHRM(png_ptr, info_ptr, PNG_sRGB_INTENT_ABSOLUTE);
  png_set_rows(png_ptr, info_ptr, bitmap);
  png_write_png(png_ptr, info_ptr, PNG_TRANSFORM_IDENTITY, NULL);
  png_destroy_write_struct(&png_ptr, &info_ptr);
  fclose(file);
}

int
main(void)
{
  printf("Generating a %ux%u image (%u pixels)\n", WIDTH, HEIGHT, SIZE);

  // For consistent images
  srand(0);

  // Generate all the colors
  uint32_t *colors = create_colors();
  // Make a pool of potential k-d nodes
  kd_node_t *nodes = create_kd_nodes();

  // Allocate the bitmap
  png_byte **bitmap = create_bitmap();

  // Generate the image
  generate_image(colors, nodes, WIDTH/2, HEIGHT/2, bitmap);

  // Write out the image
  write_png("kd-forest.png", bitmap);

  // Clean up
  for (unsigned int i = 0; i < HEIGHT; ++i) {
    free(bitmap[i]);
  }
  free(bitmap);
  free(nodes);
  free(colors);
  return 0;
}
