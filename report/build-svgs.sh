#!/bin/bash
for f in assets/**/*.svg; do rsvg-convert -o "${f%.svg}.png" -d $1 -p $1 "$f"; done
