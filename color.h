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

#ifndef COLOR_H
#define COLOR_H

#include "kd-forest.h"
#include <stdint.h>

void color_unpack(uint8_t pixel[3], uint32_t color);

// Use RGB coordinates
void color_set_RGB(double coords[3], uint32_t color);
// Use CIE L*a*b* coordinates
void color_set_Lab(double coords[3], uint32_t color);
// Use CIE L*u*v* coordinates
void color_set_Luv(double coords[3], uint32_t color);

#endif // COLOR_H
