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
