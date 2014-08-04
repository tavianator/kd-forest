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

#include "util.h"
#include <stdlib.h>

void *
xmalloc(size_t size)
{
  void *ret = malloc(size);
  if (!ret) {
    abort();
  }
  return ret;
}

void *
xrealloc(void *ptr, size_t size)
{
  void *ret = realloc(ptr, size);
  if (!ret) {
    abort();
  }
  return ret;
}

// Based on sample rand() implementation from POSIX.1-2001

static unsigned long xrand_next = 0;

void xsrand(unsigned int seed) {
  xrand_next = seed;
}

static unsigned int xrand_simple(void) {
  xrand_next = xrand_next*1103515245 + 12345;
  return (unsigned int)(xrand_next/65536)%32768;
}

static unsigned int xrand_full(void) {
  unsigned int low = xrand_simple();
  unsigned int high = xrand_simple();
  return low | (high << 15);
}

#define XRAND_RANGE 1073741824U

unsigned int
xrand(unsigned int range)
{
  // Compensate for bias if XRAND_RANGE isn't a multiple of range
  unsigned int limit = XRAND_RANGE - XRAND_RANGE%range;
  unsigned int res;
  do {
    res = xrand_full();
  } while (res >= limit);
  return res%range;
}
