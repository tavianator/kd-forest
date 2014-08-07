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

#ifndef OPTIONS_H
#define OPTIONS_H

#include <stdbool.h>
#include <stdio.h>

// Possible generation modes
typedef enum {
  MODE_HUE_SORT,
  MODE_RANDOM,
} mode_t;

// Possible pixel selection modes
typedef enum {
  SELECTION_MIN,
  SELECTION_MEAN,
} selection_t;

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
  selection_t selection;
  color_space_t color_space;
  bool animate;
  const char *filename;
  unsigned int seed;
  bool help;
} options_t;

bool parse_options(options_t *options, int argc, char *argv[]);
void print_usage(FILE *file, const char *command);

#endif // OPTIONS_H
