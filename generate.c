/*********************************************************************
 * kd-forest                                                         *
 * Copyright (C) 2015 Tavian Barnes <tavianator@tavianator.com>      *
 *                                                                   *
 * This program is free software. It comes without any warranty, to  *
 * the extent permitted by applicable law. You can redistribute it   *
 * and/or modify it under the terms of the Do What The Fuck You Want *
 * To Public License, Version 2, as published by Sam Hocevar. See    *
 * the COPYING file or http://www.wtfpl.net/ for more details.       *
 *********************************************************************/

#include "generate.h"
#include "color.h"
#include "hilbert.h"
#include "util.h"
#include <stdlib.h>

uint32_t *
generate_colors(const options_t *options)
{
  const unsigned int bit_depth = options->bit_depth;
  mode_t mode = options->mode;

  // Allocate bits from most to least perceptually important
  unsigned int grb_bits[3];
  for (unsigned int i = 0; i < 3; ++i) {
    grb_bits[i] = (bit_depth + 2 - i)/3;
  }

  uint32_t *colors = xmalloc(options->ncolors*sizeof(uint32_t));
  for (uint32_t i = 0; i < (1 << bit_depth); ++i) {
    uint32_t n = i;
    uint32_t grb[3] = {0, 0, 0};

    switch (mode) {
    case MODE_MORTON:
      for (unsigned int j = 0; j < bit_depth; ++j) {
        grb[j%3] |= (i & (1 << j)) >> (j - j/3);
      }
      break;

    case MODE_HILBERT:
      hilbert_point(3, grb_bits, n, grb);
      break;

    default:
      for (unsigned int j = 0; j < 3; ++j) {
        grb[j] = n & ((1 << grb_bits[j]) - 1);
        n >>= grb_bits[j];
      }
      break;
    }

    // Pad out colors, and put them in RGB order
    grb[0] <<= 16U - grb_bits[0];
    grb[1] <<= 24U - grb_bits[1];
    grb[2] <<=  8U - grb_bits[2];

    colors[i] = grb[1] | grb[0] | grb[2];
  }

  switch (mode) {
  case MODE_HUE_SORT:
    qsort(colors, options->ncolors, sizeof(uint32_t), color_comparator);
    break;

  case MODE_RANDOM:
    // Fisher-Yates shuffle
    for (unsigned int i = options->ncolors; i-- > 0;) {
      unsigned int j = xrand(i + 1);
      uint32_t temp = colors[i];
      colors[i] = colors[j];
      colors[j] = temp;
    }
    break;

  default:
    break;
  }

  return colors;
}
