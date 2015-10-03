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

#ifndef GENERATE_H
#define GENERATE_H

#include "options.h"
#include <stdint.h>

// Generate the colors according to the mode
uint32_t *generate_colors(const options_t *options);

#endif // GENERATE_H
