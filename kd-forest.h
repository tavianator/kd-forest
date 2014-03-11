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

#ifndef KD_FOREST_H
#define KD_FOREST_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define KD_DIMEN 3

// Single node in a k-d tree
typedef struct kd_node_t {
  // Node coordinates
  double coords[KD_DIMEN];
  // Sub-trees
  struct kd_node_t *left, *right;
  // Used to keep track of which sub-tree a node is in during construction
  bool is_left;
  // State flags
  bool added, removed;

  // Corresponding image position for this node
  unsigned int x, y;
} kd_node_t;

void kd_node_init(kd_node_t *node, unsigned int x, unsigned int y);

// A forest of k-d trees
typedef struct {
  // Array of k-d tree roots
  kd_node_t **roots;
  // Size and capacity of the roots array
  unsigned int roots_size, roots_capacity;
  // The actual size of this tree
  size_t size;
  // The size estimate for this tree, counting removed nodes
  size_t size_est;
} kd_forest_t;

void kdf_init(kd_forest_t *kdf);
void kdf_destroy(kd_forest_t *kdf);
void kdf_insert(kd_forest_t *kdf, kd_node_t *node);
void kdf_remove(kd_forest_t *kdf, kd_node_t *node);
kd_node_t *kdf_find_nearest(kd_forest_t *kdf, kd_node_t *target);

#endif // KD_FOREST_H
