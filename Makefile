#####################################################################
# kd-forest                                                         #
# Copyright (C) 2014 Tavian Barnes <tavianator@tavianator.com>      #
#                                                                   #
# This program is free software. It comes without any warranty, to  #
# the extent permitted by applicable law. You can redistribute it   #
# and/or modify it under the terms of the Do What The Fuck You Want #
# To Public License, Version 2, as published by Sam Hocevar. See    #
# the COPYING file or http://www.wtfpl.net/ for more details.       #
#####################################################################

CC ?= gcc
CFLAGS ?= -std=c99 -pipe -O2 -Werror -Wall -Wpedantic -Wextra -Wno-sign-compare -Wno-unused-parameter -Wunreachable-code -Wshadow -Wpointer-arith -Wwrite-strings -Wcast-align -Wstrict-prototypes
LDFLAGS ?= -lm -lpng
RM ?= rm -f

kd-forest: kd-forest.c kd-forest.h util.c util.h main.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o kd-forest kd-forest.c util.c main.c

image: kd-forest
	./kd-forest

clean:
	$(RM) kd-forest

.PHONY: image clean
