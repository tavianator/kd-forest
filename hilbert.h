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

#ifndef HILBERT_H
#define HILBERT_H

#include <stdint.h>

/**
 * Compute the point corresponding to the given (compact) Hilbert index.
 *
 * @param dimensions
 *         The number of spatial dimensions.
 * @param extents
 *         The bit depth of each dimension.
 * @param index
 *         The (compact) Hilbert index of the desired point.
 * @param[out] point
 *         Will hold the point on the Hilbert curve at index.
 */
void hilbert_point(unsigned int dimensions, const unsigned int extents[], uint32_t index, uint32_t point[]);

#endif // HILBERT_H
