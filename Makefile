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

CC = gcc
CFLAGS = -std=c99 -D_POSIX_C_SOURCE=200809L -pipe -g -O3 -flto -Werror -Wall -Wpedantic -Wextra -Wno-sign-compare -Wno-unused-parameter -Wunreachable-code -Wshadow -Wpointer-arith -Wwrite-strings -Wcast-align -Wstrict-prototypes
LDFLAGS = -Wl,-O1,--sort-common,--as-needed,-z,relro
LIBS = -lm -lpng
RM = rm -f

HEADERS = color.h kd-forest.h options.h util.h

kd-forest: color.o kd-forest.o main.o options.o util.o
	$(CC) $(CFLAGS) $(LDFLAGS) $^ $(LIBS) -o kd-forest

%.o: %.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

image: kd-forest.png

kd-forest.png: kd-forest
	./kd-forest -b 24 -s -c Lab -o kd-forest.png

anim: kd-forest.mkv

kd-forest.mkv: kd-forest
	$(RM) kd-forest.mkv
	$(RM) -r frames
	mkdir -p frames
	./kd-forest -b 20 -s -c Lab -a -o frames
	ffmpeg -r 60 -i frames/%04d.png -c:v libx264 -preset veryslow -qp 0 kd-forest.mkv

clean:
	$(RM) *.o
	$(RM) kd-forest
	$(RM) -r frames

.PHONY: image anim clean
