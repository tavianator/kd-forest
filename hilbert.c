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

#include "hilbert.h"
#include <stdint.h>

// These algorithms are described in "Compact Hilbert Indices" by Chris Hamilton

// Right rotation of x by b bits out of n
static uint32_t
ror(uint32_t x, unsigned int b, unsigned int n)
{
  uint32_t l = x & ((1 << b) - 1);
  uint32_t r = x >> b;
  return (l << (n - b)) | r;
}

// Left rotation of x by b bits out of n
static uint32_t
rol(uint32_t x, unsigned int b, unsigned int n)
{
  return ror(x, n - b, n);
}

// Binary reflected Gray code
uint32_t
gray_code(uint32_t i)
{
  return i ^ (i >> 1);
}

// e(i), the entry point for the ith sub-hypercube
uint32_t
entry_point(uint32_t i)
{
  if (i == 0) {
    return 0;
  } else {
    return gray_code((i - 1) & ~1U);
  }
}

// g(i), the inter sub-hypercube direction
unsigned int
inter_direction(uint32_t i)
{
  // g(i) counts the trailing set bits in i
  unsigned int g = 0;
  while (i & 1) {
    ++g;
    i >>= 1;
  }
  return g;
}

// d(i), the intra sub-hypercube direction
unsigned int
intra_direction(uint32_t i)
{
  if (i & 1) {
    return inter_direction(i);
  } else if (i > 0) {
    return inter_direction(i - 1);
  } else {
    return 0;
  }
}

// T transformation inverse
uint32_t
t_inverse(unsigned int dimensions, uint32_t e, unsigned int d, uint32_t a)
{
  return rol(a, d, dimensions) ^ e;
}

// GrayCodeRankInverse
void
gray_code_rank_inverse(unsigned int dimensions, uint32_t mu, uint32_t pi, unsigned int r, unsigned int free_bits, uint32_t *i, uint32_t *g)
{
  // *i is the inverse rank of r
  // *g is gray_code(i)

  *i = 0;
  *g = 0;

  for (unsigned int j = free_bits - 1, k = dimensions; k-- > 0;) {
    if (mu & (1 << k)) {
      *i |= ((r >> j) & 1) << k;
      *g |= (*i ^ (*i >> 1)) & (1 << k);
      --j;
    } else {
      *g |= pi & (1 << k);
      *i |= (*g ^ (*i >> 1)) & (1 << k);
    }
  }
}

// ExtractMask
void
extract_mask(unsigned int dimensions, const unsigned int extents[], unsigned int i, uint32_t *mu, unsigned int *free_bits)
{
  // *mu is the mask
  // *free_bits is popcount(*mu)

  *mu = 0;
  *free_bits = 0;

  for (unsigned int j = dimensions; j-- > 0;) {
    *mu <<= 1;

    if (extents[j] > i) {
      *mu |= 1;
      *free_bits += 1;
    }
  }
}

// CompactHilbertIndexInverse
void
hilbert_point(unsigned int dimensions, const unsigned int extents[], uint32_t index, uint32_t point[])
{
  unsigned int max_extent = 0, total_extent = 0;
  for (unsigned int i = 0; i < dimensions; ++i) {
    if (extents[i] > max_extent) {
      max_extent = extents[i];
    }
    total_extent += extents[i];
    point[i] = 0;
  }

  uint32_t e = 0;
  unsigned int k = 0;

  // Next direction; we use d instead of d + 1 everywhere
  unsigned int d = 1;

  for (unsigned int i = max_extent; i-- > 0;) {
    uint32_t mu;
    unsigned int free_bits;
    extract_mask(dimensions, extents, i, &mu, &free_bits);
    mu = ror(mu, d, dimensions);

    uint32_t pi = ror(e, d, dimensions) & ~mu;

    unsigned int r = (index >> (total_extent - k - free_bits)) & ((1 << free_bits) - 1);

    k += free_bits;

    uint32_t w, l;
    gray_code_rank_inverse(dimensions, mu, pi, r, free_bits, &w, &l);

    l = t_inverse(dimensions, e, d, l);

    for (unsigned int j = 0; j < 3; ++j) {
      point[j] |= (l & 1) << i;
      l >>= 1;
    }

    e = e ^ ror(entry_point(w), d, dimensions);
    d = (d + intra_direction(w) + 1)%dimensions;
  }
}
