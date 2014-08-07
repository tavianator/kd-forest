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

#include "options.h"
#include <ctype.h>
#include <limits.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
#if __unix__
#  include <unistd.h>
#endif

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

void
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
  usage("    %s [-l|--selection @min@|@mean@]\n", whitespace);
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
  usage("  -l, --selection @min@|@mean@:\n");
  usage("          Specify the selection mode (!default!: @min@)\n\n");
  usage("          @min@:  Pick the pixel with the closest neighboring pixel\n");
  usage("          @mean@: Pick the pixel with the closest average of all its neighbors\n\n");
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

bool
parse_options(options_t *options, int argc, char *argv[])
{
  // Set defaults
  options->bit_depth = 24;
  options->mode = MODE_HUE_SORT;
  options->selection = SELECTION_MIN;
  options->color_space = COLOR_SPACE_LAB;
  options->animate = false;
  options->filename = NULL;
  options->seed = 0;
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
    } else if (parse_arg(argc, argv, "-a", "--animate", NULL, &i, &error)) {
      options->animate = true;
    } else if (parse_arg(argc, argv, "-o", "--output", &value, &i, &error)) {
      options->filename = value;
    } else if (parse_arg(argc, argv, "-l", "--selection", &value, &i, &error)) {
      if (strcmp(value, "min") == 0) {
        options->selection = SELECTION_MIN;
      } else if (strcmp(value, "mean") == 0) {
        options->selection = SELECTION_MEAN;
      } else {
        fprintf(stderr, "Invalid selection mode: `%s'\n", value);
        error = true;
      }
    } else if (parse_arg(argc, argv, "-c", "--color-space", &value, &i, &error)) {
      if (strcmp(value, "RGB") == 0) {
        options->color_space = COLOR_SPACE_RGB;
      } else if (strcmp(value, "Lab") == 0) {
        options->color_space = COLOR_SPACE_LAB;
      } else if (strcmp(value, "Luv") == 0) {
        options->color_space = COLOR_SPACE_LUV;
      } else {
        fprintf(stderr, "Invalid color space: `%s'\n", value);
        error = true;
      }
    } else if (parse_arg(argc, argv, "-e", "--seed", &value, &i, &error)) {
      if (!str_to_uint(value, &options->seed)) {
        fprintf(stderr, "Invalid random seed: `%s'\n", value);
        error = true;
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
