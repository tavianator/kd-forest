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
#include <math.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

void
kd_node_init(kd_node_t *node, unsigned int x, unsigned int y)
{
  node->left = node->right = NULL;
  node->x = x;
  node->y = y;
  node->added = node->removed = false;
}

static size_t
kd_collect_nodes(kd_node_t *root, kd_node_t **buffer, bool include_removed)
{
  size_t count = 0;
  if (include_removed || !root->removed) {
    buffer[0] = root;
    ++count;
  }
  if (root->left) {
    count += kd_collect_nodes(root->left, buffer + count, include_removed);
  }
  if (root->right) {
    count += kd_collect_nodes(root->right, buffer + count, include_removed);
  }
  return count;
}

typedef int kd_comparator(const void *a, const void* b);

static int kd_compare0(const void *a, const void *b) {
  double aval = (*(const kd_node_t **)a)->coords[0];
  double bval = (*(const kd_node_t **)b)->coords[0];
  return (aval > bval) - (aval < bval);
}

static int kd_compare1(const void *a, const void *b) {
  double aval = (*(const kd_node_t **)a)->coords[1];
  double bval = (*(const kd_node_t **)b)->coords[1];
  return (aval > bval) - (aval < bval);
}

static int kd_compare2(const void *a, const void *b) {
  double aval = (*(const kd_node_t **)a)->coords[2];
  double bval = (*(const kd_node_t **)b)->coords[2];
  return (aval > bval) - (aval < bval);
}

static kd_comparator *kd_comparators[KD_DIMEN] = {
  kd_compare0,
  kd_compare1,
  kd_compare2,
};

// When building k-d trees, we use KD_DIMEN sorted arrays of nodes plus one
// extra array for scratch space
#define KD_BUFSIZE (KD_DIMEN + 1)

static kd_node_t *
kd_build_tree_recursive(kd_node_t **buffers[KD_BUFSIZE], size_t size, unsigned int coord)
{
  if (size == 0) {
    return NULL;
  }

  size_t split = size/2;
  size_t left_size = split, right_size = size - left_size - 1;
  kd_node_t *root = buffers[coord][split];
  for (size_t i = 0; i < size; ++i) {
    buffers[coord][i]->is_left = i < left_size;
  }

  kd_node_t **right_buffers[KD_BUFSIZE];
  for (int i = 0; i < KD_DIMEN; ++i) {
    right_buffers[i] = buffers[i] + left_size + 1;
  }

  kd_node_t **scratch = buffers[KD_DIMEN];
  right_buffers[KD_DIMEN] = scratch;

  for (size_t i = 0; i < KD_DIMEN; ++i) {
    if (i == coord) {
      continue;
    }

    kd_node_t **buffer = buffers[i];
    kd_node_t **right_buffer = right_buffers[i];
    for (size_t j = 0, k = 0, skip = 0; j < size; ++j) {
      if (buffer[j]->is_left) {
        buffer[j - skip] = buffer[j];
      } else {
        if (buffer[j] != root) {
          scratch[k] = buffer[j];
          ++k;
        }
        ++skip;
      }
    }
    for (size_t j = 0; j < right_size; ++j) {
      right_buffer[j] = scratch[j];
    }
  }

  coord = (coord + 1)%KD_DIMEN;
  root->left = kd_build_tree_recursive(buffers, left_size, coord);
  root->right = kd_build_tree_recursive(right_buffers, right_size, coord);

  return root;
}

static kd_node_t *
kd_build_tree(kd_node_t **buffers[KD_BUFSIZE], size_t size)
{
  for (int i = 1; i < KD_DIMEN; ++i) {
    memcpy(buffers[i], buffers[0], size*sizeof(kd_node_t *));
  }
  for (int i = 0; i < KD_DIMEN; ++i) {
    qsort(buffers[i], size, sizeof(kd_node_t *), kd_comparators[i]);
  }
  return kd_build_tree_recursive(buffers, size, 0);
}

static double
kd_distance_sq(kd_node_t *a, kd_node_t *b)
{
  double result = 0.0;
  for (int i = 0; i < KD_DIMEN; ++i) {
    double d = a->coords[i] - b->coords[i];
    result += d*d;
  }
  return result;
}

static void
kd_find_nearest_recursive(kd_node_t *root, kd_node_t *target, kd_node_t **best, double *limit, unsigned int coord)
{
  double dist = target->coords[coord] - root->coords[coord];
  double dist_sq = dist*dist;

  if (!root->removed) {
    double root_dist_sq = kd_distance_sq(root, target);
    if (root_dist_sq < *limit) {
      *best = root;
      *limit = root_dist_sq;
    }
  }

  coord = (coord + 1)%KD_DIMEN;

  if (root->left && (dist <= 0 || dist_sq <= *limit)) {
    kd_find_nearest_recursive(root->left, target, best, limit, coord);
  }
  if (root->right && (dist >= 0 || dist_sq <= *limit)) {
    kd_find_nearest_recursive(root->right, target, best, limit, coord);
  }
}

static void
kd_find_nearest(kd_node_t *root, kd_node_t *target, kd_node_t **best, double *limit)
{
  kd_find_nearest_recursive(root, target, best, limit, 0);
}

void
kdf_init(kd_forest_t *kdf)
{
  kdf->roots = NULL;
  kdf->size = kdf->size_est = 0;
  kdf->roots_size = kdf->roots_capacity = 0;
}

void
kdf_destroy(kd_forest_t *kdf)
{
  free(kdf->roots);
}

static size_t
kdf_collect_nodes(kd_forest_t *kdf, kd_node_t **buffer, unsigned int slot, bool include_removed)
{
  size_t count = 0;
  for (unsigned int i = 0; i < slot; ++i) {
    if (kdf->roots[i]) {
      count += kd_collect_nodes(kdf->roots[i], buffer + count, include_removed);
    }
  }
  return count;
}

static void
kdf_balance(kd_forest_t *kdf, kd_node_t *new_node, bool force)
{
  ++kdf->size;

  size_t slot, buffer_size;
  if (force) {
    buffer_size = kdf->size_est = kdf->size;
    slot = kdf->roots_size;
  } else {
    ++kdf->size_est;
    for (slot = 0; slot < kdf->roots_size; ++slot) {
      if (!kdf->roots[slot]) {
        break;
      }
    }
    buffer_size = 1 << slot;
  }

  kd_node_t **buffer = xmalloc(buffer_size*sizeof(kd_node_t *));
  buffer[0] = new_node;
  kdf_collect_nodes(kdf, buffer + 1, slot, !force);

  kd_node_t **buffers[KD_BUFSIZE];
  for (int i = 1; i < KD_BUFSIZE; ++i) {
    buffers[i] = xmalloc(buffer_size*sizeof(kd_node_t *));
  }

  if (slot >= kdf->roots_capacity) {
    kdf->roots_capacity = slot + 1;
    kdf->roots = xrealloc(kdf->roots, kdf->roots_capacity*sizeof(kd_node_t *));
  }

  size_t i, offset;
  for (i = 0, offset = 0; offset < buffer_size; ++i) {
    size_t chunk_size = 1 << i;
    if (buffer_size & chunk_size) {
      buffers[0] = buffer + offset;
      kdf->roots[i] = kd_build_tree(buffers, chunk_size);
      offset |= chunk_size;
    } else {
      kdf->roots[i] = NULL;
    }
  }
  if (force || i > kdf->roots_size) {
    kdf->roots_size = i;
  }

  free(buffer);
  for (i = 1; i < KD_BUFSIZE; ++i) {
    free(buffers[i]);
  }
}

void
kdf_insert(kd_forest_t *kdf, kd_node_t *node)
{
  node->added = true;

  // If half or more of the nodes are removed, force a complete rebalance
  bool force = (kdf->size_est + 1) >= 2*(kdf->size + 1);
  kdf_balance(kdf, node, force);
}

void
kdf_remove(kd_forest_t *kdf, kd_node_t *node)
{
  --kdf->size;
  node->removed = true;
}

kd_node_t *
kdf_find_nearest(kd_forest_t *kdf, kd_node_t *target)
{
  double limit = INFINITY;
  kd_node_t *best = NULL;

  for (unsigned int i = 0; i < kdf->roots_size; ++i) {
    kd_node_t *root = kdf->roots[i];
    if (root != NULL) {
      kd_find_nearest(root, target, &best, &limit);
    }
  }

  return best;
}
